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
mod items;
mod question_queue;

const SERVER_PORT: u16 = 3013;
const IDLE_KICK_TIME: i64 = 10;
pub const COINS_EVERY_X_SECONDS: f64 = 3.0;
pub const SUBMIT_QUESTION_EVERY_X_SECONDS: f64 = 5.0;
pub const SUBMIT_QUESTION_COST: i32 = 2;
pub const ANONYMOUS_QUESTION_COST: i32 = 5;
pub const VOTE_QUESTION_COST: i32 = 1;
pub const SCORE_TO_COINS_RATIO: i32 = 2;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Server {
    id: String,
    started: bool,
    elapsed_time: f64,
    last_update: DateTime<Utc>,
    key_player: String,
    players: HashMap<String, Player>,
    questions_queue: Vec<QueuedQuestion>,
    items: Vec<Item>,
    items_history: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Player {
    name: String,
    last_contact: DateTime<Utc>,
    score: i32,
    coins: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct QueuedQuestion {
    player: String,
    question: String,
    anonymous: bool,
    votes: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Item {
    name: String,
    id: u32,
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

pub type ServerStorage = Arc<Mutex<HashMap<String, Server>>>;

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
            "/server/:server_id/disconnect/:player_name",
            post(disconnect_player),
        )
        .route("/server/:server_id/start/:player_name", post(start_server))
        .route(
            "/server/:server_id/getstate/:player_name",
            get(get_game_state),
        )
        .route(
            "/server/:server_id/submitquestion/:player_name/:question/:options",
            post(question_queue::player_submit_question),
        )
        .route(
            "/server/:server_id/votequestion/:player_name/:question",
            post(question_queue::player_vote_question),
        )
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
    let server = servers.entry(server_id.clone()).or_insert_with(|| {
        println!("Creating new server '{server_id}'");
        Server {
            id: server_id.clone(),
            key_player: player_name.clone(),
            players: HashMap::new(),
            started: false,
            elapsed_time: 0.0,
            last_update: Utc::now(),
            questions_queue: Vec::new(),
            items: Vec::new(),
            items_history: Vec::new(),
        }
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
        coins: 3,
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
        println!("Player '{player_name}' disconnected from server '{server_id}'");
        if player_name == server.key_player {
            servers.remove(&server_id);
            println!("Key player left, server '{server_id}' closed");
            return (
                StatusCode::OK,
                format!("Key player left, server '{server_id}' closed"),
            );
        }
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
        if server.started {
            println!("Server '{server_id}' attempted to start, already started'");
            (
                StatusCode::BAD_REQUEST,
                "Server already started".to_string(),
            )
        } else if player_name == server.key_player {
            server.started = true;
            server.last_update = Utc::now();
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

#[allow(clippy::cast_precision_loss)]
async fn server_loop(servers: ServerStorage) {
    loop {
        let current_time = Utc::now();

        // Lock and process servers to decide which ones to update or remove
        let mut servers_to_update = Vec::new();
        {
            let servers = servers.lock().await;
            for (id, server) in servers.iter() {
                let mut active_players = HashMap::new();
                let mut remove_server = true;

                for (player_id, player) in &server.players {
                    if Utc::now()
                        .signed_duration_since(player.last_contact)
                        .num_seconds()
                        <= IDLE_KICK_TIME
                    {
                        active_players.insert(player_id.clone(), player.clone());
                        if &server.key_player == player_id {
                            remove_server = false; // Key player is active
                        }
                    } else {
                        println!("Kicking player '{player_id}' due to idle");
                    }
                }

                if remove_server {
                    println!("Removing server '{id}' due to no key player or no players");
                } else {
                    servers_to_update.push((id.clone(), active_players));
                }
            }
        }

        // Apply updates to the servers
        {
            let mut servers = servers.lock().await;
            // Remove servers that are not in the update list
            servers.retain(|id, _| {
                servers_to_update
                    .iter()
                    .any(|(update_id, _)| update_id == id)
            });
            // Update servers that are in the update list
            for (id, active_players) in servers_to_update {
                if let Some(server) = servers.get_mut(&id) {
                    server.players = active_players;
                    if server.started {
                        let elapsed_time_update = current_time
                            .signed_duration_since(server.last_update)
                            .num_milliseconds()
                            as f64
                            / 1_000.0;

                        // Determine the previous and current multiples of x seconds
                        let previous_multiple = server.elapsed_time / COINS_EVERY_X_SECONDS;
                        let current_multiple =
                            (server.elapsed_time + elapsed_time_update) / COINS_EVERY_X_SECONDS;

                        // Check if the elapsed time has crossed a multiple of x seconds
                        if current_multiple.trunc() > previous_multiple.trunc() {
                            // Give a coin to each player
                            for player in server.players.values_mut() {
                                player.coins += 1;
                            }
                        }

                        // Update elapsed time and last update time
                        server.elapsed_time += elapsed_time_update;
                        server.last_update = current_time;
                    }
                }
            }
        }

        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}
