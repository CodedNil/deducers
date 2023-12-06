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

#[allow(dead_code)]
struct Server {
    id: String,
    started: bool,
    key_player: String,
    players: HashMap<String, Player>,
    questions_queue: Vec<QueuedQuestion>,
    items: Vec<Item>,
}

#[derive(Clone, Serialize, Deserialize)]
struct Player {
    name: String,
    last_contact: DateTime<Utc>,
    score: i32,
}

#[allow(dead_code)]
struct QueuedQuestion {
    player: String,
    question: String,
    votes: u32,
}

#[allow(dead_code)]
struct Item {
    name: String,
    questions: Vec<Question>,
}

#[allow(dead_code)]
struct Question {
    player: String,
    question: String,
    answer: Answer,
    anonymous: bool,
}

#[allow(dead_code)]
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

    // Add a demo run that runs after 1 second and makes http requests to the server
    // tokio::spawn(async move {
    //     tokio::time::sleep(Duration::from_secs(1)).await;
    //     let client = reqwest::Client::new();
    //     let server = format!("http://localhost:{SERVER_PORT}");
    //     let server_id = "demo".to_string();
    //     let player_name = "dan".to_string();

    //     // Connect player
    //     let url = format!("{server}/server/{server_id}/connect/{player_name}");
    //     let response = client.post(&url).send().await.unwrap();
    //     println!("Response: {}", response.text().await.unwrap());

    //     // Get players
    //     let url = format!("{server}/server/{server_id}/getplayers");
    //     let response = client.get(&url).send().await.unwrap();
    //     println!("Response: {}", response.text().await.unwrap());

    //     // Start server
    //     let url = format!("{server}/server/{server_id}/start/{player_name}");
    //     let response = client.post(&url).send().await.unwrap();
    //     println!("Response: {}", response.text().await.unwrap());
    // });

    // Router setup
    let app = Router::new()
        .route(
            "/server/:server_id/connect/:player_name",
            post(connect_player),
        )
        .route("/server/:server_id/getplayers", get(get_players))
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

async fn connect_player(
    Path((server_id, player_name)): Path<(String, String)>,
    Extension(servers): Extension<ServerStorage>,
) -> impl IntoResponse {
    let mut servers = servers.lock().await;

    let server = servers.entry(server_id.clone()).or_insert_with(|| Server {
        id: server_id.clone(),
        key_player: player_name.clone(),
        players: HashMap::new(),
        started: false,
        questions_queue: Vec::new(),
        items: Vec::new(),
    });

    server
        .players
        .entry(player_name.clone())
        .and_modify(|player| player.last_contact = Utc::now())
        .or_insert(Player {
            name: player_name,
            last_contact: Utc::now(),
            score: 0,
        });

    (
        StatusCode::OK,
        format!("Player connected to server: {server_id}"),
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
            return (
                StatusCode::OK,
                format!("Key player left, server {server_id} closed."),
            );
        }
        (
            StatusCode::OK,
            format!("Player {player_name} disconnected from server: {server_id}"),
        )
    } else {
        (
            StatusCode::NOT_FOUND,
            format!("Server {server_id} not found."),
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
            (
                StatusCode::OK,
                format!("Server {server_id} started by key player {player_name}."),
            )
        } else {
            (
                StatusCode::FORBIDDEN,
                "Only the key player can start the server.".to_string(),
            )
        }
    } else {
        (
            StatusCode::NOT_FOUND,
            format!("Server {server_id} not found."),
        )
    }
}

async fn get_players(
    Path(server_id): Path<String>,
    Extension(servers): Extension<ServerStorage>,
) -> impl IntoResponse {
    let servers = servers.lock().await;

    if let Some(server) = servers.get(&server_id) {
        let player_list = server.players.values().cloned().collect::<Vec<_>>();
        (StatusCode::OK, Json(player_list))
    } else {
        (StatusCode::NOT_FOUND, Json(vec![]))
    }
}

async fn server_loop(servers: ServerStorage) {
    loop {
        // Create a list of server IDs that need to be modified or removed
        let mut to_modify = Vec::new();
        let mut to_remove = Vec::new();

        // Lock, read, and immediately release the lock
        {
            let servers = servers.lock().await;
            for (id, server) in servers.iter() {
                let is_active = server.players.iter().any(|(_, player)| {
                    Utc::now()
                        .signed_duration_since(player.last_contact)
                        .num_seconds()
                        < IDLE_KICK_TIME
                });

                if is_active {
                    to_modify.push(id.clone());
                } else {
                    to_remove.push(id.clone());
                }
            }
        }

        // Lock again to modify the data
        {
            let mut servers = servers.lock().await;
            for id in to_modify {
                if let Some(server) = servers.get_mut(&id) {
                    server.players.retain(|_, player| {
                        Utc::now()
                            .signed_duration_since(player.last_contact)
                            .num_seconds()
                            < IDLE_KICK_TIME
                    });
                }
            }

            for id in to_remove {
                servers.remove(&id);
            }
        }

        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}
