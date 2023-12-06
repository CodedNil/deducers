use axum::{
    extract::Path,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Extension, Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::{net::TcpListener, sync::Mutex};

const SERVER_PORT: u16 = 3013;
const IDLE_KICK_TIME: i64 = 10;

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Server {
    id: String,
    started: bool,
    key_player: String,
    players: HashMap<String, Player>,
    questions_queue: Vec<QueuedQuestion>,
    items: Vec<Item>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Player {
    name: String,
    last_contact: DateTime<Utc>,
    score: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct QueuedQuestion {
    player: String,
    question: String,
    votes: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Item {
    name: String,
    questions: Vec<Question>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Question {
    player: String,
    question: String,
    answer: Answer,
    anonymous: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
enum Answer {
    Yes,
    No,
    Sometimes,
    Depends,
    Irrelevant,
}

type ServerStorage = Arc<Mutex<HashMap<String, Server>>>;

#[tokio::main]
async fn main() {
    // Shared server storage
    let servers = ServerStorage::new(Mutex::new(HashMap::new()));

    // Launch the server loop in a separate async task
    let servers_clone = servers.clone();
    tokio::spawn(async move {
        server_loop(servers_clone).await;
    });

    // Router setup
    let app = Router::new()
        .route(
            "/server/:server_id/connect/:player_name",
            post(connect_player),
        )
        .route(
            "/server/:server_id/getstate/:player_name",
            get(get_game_state),
        )
        .route(
            "/server/:server_id/disconnect/:player_name",
            post(disconnect_player),
        )
        .route("/server/:server_id/start/:player_name", post(start_server))
        .layer(Extension(servers));

    // Server setup
    let address = format!("0.0.0.0:{SERVER_PORT}");
    let listener = TcpListener::bind(&address).await.unwrap();
    println!("Server running on {address}");
    axum::serve(listener, app).await.unwrap();
}

#[derive(Serialize)]
enum GameStateResponse {
    ServerState(Server),
    Error(String),
}

async fn connect_player(
    Path((server_id, player_name)): Path<(String, String)>,
    Extension(servers): Extension<ServerStorage>,
) -> impl IntoResponse {
    let mut servers = servers.lock().await;

    // Get the server or create a new one
    let server = servers.entry(server_id.clone()).or_insert_with(|| Server {
        id: server_id.clone(),
        key_player: player_name.clone(),
        players: HashMap::new(),
        started: false,
        questions_queue: Vec::new(),
        items: Vec::new(),
    });

    // Check if player with the same name is already connected
    if server.players.contains_key(&player_name) {
        return (
            StatusCode::BAD_REQUEST,
            Json(GameStateResponse::Error(format!(
                "Player '{player_name}' is already connected to server '{server_id}'"
            ))),
        );
    }

    // Add the player to the server
    server.players.entry(player_name.clone()).or_insert(Player {
        name: player_name.clone(),
        last_contact: Utc::now(),
        score: 0,
    });

    // Return the game state
    println!("Player '{player_name}' connected to server '{server_id}'");
    (
        StatusCode::OK,
        Json(GameStateResponse::ServerState(server.clone())),
    )
}

async fn disconnect_player(
    Path((server_id, player_name)): Path<(String, String)>,
    Extension(servers): Extension<ServerStorage>,
) -> impl IntoResponse {
    let mut servers = servers.lock().await;

    if let Some(server) = servers.get_mut(&server_id) {
        server.players.remove(&player_name);
        if player_name == server.key_player {
            servers.remove(&server_id);
            println!("Key player left, server '{server_id}' closed");
            return (
                StatusCode::OK,
                format!("Key player left, server '{server_id}' closed"),
            );
        }
        println!("Player '{player_name}' disconnected from server '{server_id}'");
        (
            StatusCode::OK,
            format!("Player '{player_name}' disconnected from server '{server_id}'"),
        )
    } else {
        (
            StatusCode::NOT_FOUND,
            format!("Server '{server_id}' not found."),
        )
    }
}

async fn start_server(
    Path((server_id, player_name)): Path<(String, String)>,
    Extension(servers): Extension<ServerStorage>,
) -> impl IntoResponse {
    let mut servers = servers.lock().await;

    if let Some(server) = servers.get_mut(&server_id) {
        if player_name == server.key_player {
            server.started = true;
            println!("Server '{server_id}' started by key player '{player_name}'");
            (
                StatusCode::OK,
                format!("Server '{server_id}' started by key player '{player_name}'"),
            )
        } else {
            (
                StatusCode::FORBIDDEN,
                "Only the key player can start the server".to_string(),
            )
        }
    } else {
        (
            StatusCode::NOT_FOUND,
            format!("Server '{server_id}' not found"),
        )
    }
}

async fn get_game_state(
    Path((server_id, player_name)): Path<(String, String)>,
    Extension(servers): Extension<ServerStorage>,
) -> impl IntoResponse {
    let mut servers = servers.lock().await;

    if let Some(server) = servers.get_mut(&server_id) {
        if let Some(player) = server.players.get_mut(&player_name) {
            // Update last contact time for the player
            player.last_contact = Utc::now();

            // Return the entire state of the server
            (
                StatusCode::OK,
                Json(GameStateResponse::ServerState(server.clone())),
            )
        } else {
            (
                StatusCode::NOT_FOUND,
                Json(GameStateResponse::Error(format!(
                    "Player '{player_name}' not found in server '{server_id}'"
                ))),
            )
        }
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(GameStateResponse::Error(format!(
                "Server '{server_id}' not found"
            ))),
        )
    }
}

async fn server_loop(servers: ServerStorage) {
    loop {
        // Lock the servers once and perform all operations
        let mut servers = servers.lock().await;

        servers.retain(|id, server| {
            server.players.retain(|player_id, player| {
                if Utc::now()
                    .signed_duration_since(player.last_contact)
                    .num_seconds()
                    > IDLE_KICK_TIME
                {
                    println!("Kicking player '{player_id}' due to idle");
                    false // Remove idle players
                } else {
                    true // Keep active players
                }
            });

            if server.players.is_empty() {
                println!("Removing server '{id}' due to no players");
                false // Remove server if no players
            } else {
                true // Keep server with players
            }
        });

        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}
