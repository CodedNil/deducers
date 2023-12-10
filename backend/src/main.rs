use axum::{
    extract::Path,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Extension, Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, net::SocketAddr, sync::Arc, time::Duration};
use tokio::{net::TcpListener, sync::Mutex};

mod game_state;
mod items;
mod openai;
mod question_queue;

pub const SERVER_PORT: u16 = 3013;
pub const IDLE_KICK_TIME: i64 = 10;
pub const COINS_EVERY_X_SECONDS: f64 = 3.0;
pub const SUBMIT_QUESTION_EVERY_X_SECONDS: f64 = 10.0;
pub const SUBMIT_QUESTION_COST: i32 = 2;
pub const ANONYMOUS_QUESTION_COST: i32 = 5;
pub const VOTE_QUESTION_COST: i32 = 1;
pub const SCORE_TO_COINS_RATIO: i32 = 2;
pub const ADD_ITEM_EVERY_X_QUESTIONS: u32 = 5;

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
    items_queue: Vec<String>,
    last_add_to_queue: DateTime<Utc>,
    questions_counter: u32,
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
    id: u32,
    question: String,
    answer: Answer,
    anonymous: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
enum Answer {
    Yes,
    No,
    Maybe,
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
            get(game_state::get_state),
        )
        .route(
            "/server/:server_id/submitquestion/:player_name/:question/:options",
            post(question_queue::player_submit_question),
        )
        .route(
            "/server/:server_id/votequestion/:player_name/:question",
            post(question_queue::player_vote_question),
        )
        .route(
            "/server/:server_id/convertscore/:player_name",
            post(question_queue::player_convert_score),
        )
        .route(
            "/server/:server_id/kickplayer/:player_name/:kick_player",
            post(kick_player),
        )
        .route(
            "/internal/:server_id/additemqueued/:item_name",
            post(items::add_item_to_server_queue),
        )
        .layer(Extension(servers))
        .into_make_service_with_connect_info::<SocketAddr>();

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
    let mut servers_locked = servers.lock().await;

    // Get the server or create a new one
    let server = servers_locked.entry(server_id.clone()).or_insert_with(|| {
        println!("Creating new server '{server_id}'");
        // Add initial items to the servers queue
        let server_id_clone = server_id.clone();
        tokio::spawn(async move {
            items::add_item_to_queue(server_id_clone, vec![], 0).await;
        });

        Server {
            id: server_id.clone(),
            started: false,
            elapsed_time: 0.0,
            last_update: Utc::now(),
            key_player: player_name.clone(),
            players: HashMap::new(),
            questions_queue: Vec::new(),
            items: Vec::new(),
            items_history: Vec::new(),
            items_queue: Vec::new(),
            last_add_to_queue: Utc::now(),
            questions_counter: 0,
        }
    });

    // Check if player with the same name is already connected
    if server.players.contains_key(&player_name) {
        return (
            StatusCode::BAD_REQUEST,
            Json(game_state::Response::Error(format!(
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
    let minimal_server = game_state::convert_to_minimal(server, &player_name);
    (
        StatusCode::OK,
        Json(game_state::Response::ServerState(minimal_server)),
    )
}

async fn disconnect_player(
    Path((server_id, player_name)): Path<(String, String)>,
    Extension(servers): Extension<ServerStorage>,
) -> impl IntoResponse {
    let mut servers_locked = servers.lock().await;

    if let Some(server) = servers_locked.get_mut(&server_id) {
        server.players.remove(&player_name);
        println!("Player '{player_name}' disconnected from server '{server_id}'");
        if player_name == server.key_player {
            servers_locked.remove(&server_id);
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
    let mut servers_locked = servers.lock().await;

    if let Some(server) = servers_locked.get_mut(&server_id) {
        if server.started {
            println!("Server '{server_id}' attempted to start, already started'");
            return (
                StatusCode::BAD_REQUEST,
                "Server already started".to_string(),
            );
        } else if player_name != server.key_player {
            return (
                StatusCode::FORBIDDEN,
                "Only the key player can start the server".to_string(),
            );
        } else if server.items_queue.is_empty() {
            println!("Server '{server_id}' attempted to start, no items in queue'");
            return (StatusCode::BAD_REQUEST, "No items in queue".to_string());
        }
        server.started = true;
        server.last_update = Utc::now();

        // Add 2 items to the server
        items::add_item_to_server(server);
        items::add_item_to_server(server);

        println!("Server '{server_id}' started by key player '{player_name}'");
        (
            StatusCode::OK,
            format!("Server '{server_id}' started by key player '{player_name}'"),
        )
    } else {
        (
            StatusCode::NOT_FOUND,
            format!("Server '{server_id}' not found"),
        )
    }
}

pub async fn kick_player(
    Path((server_id, player_name, kick_player)): Path<(String, String, String)>,
    Extension(servers): Extension<ServerStorage>,
) -> impl IntoResponse {
    let mut servers_locked = servers.lock().await;

    if let Some(server) = servers_locked.get_mut(&server_id) {
        if player_name != server.key_player {
            return (
                StatusCode::FORBIDDEN,
                "Only the key player can kick other players".to_string(),
            );
        }

        server.players.remove(&kick_player);

        println!("Server '{server_id}' player '{kick_player}' kicked by key player");
        (
            StatusCode::OK,
            format!("Server '{server_id}' player '{kick_player}' kicked by key player"),
        )
    } else {
        (
            StatusCode::NOT_FOUND,
            format!("Server '{server_id}' not found"),
        )
    }
}

#[allow(clippy::cast_precision_loss)]
async fn server_loop(servers: ServerStorage) {
    loop {
        // Lock and process servers to decide which ones to update or remove
        let mut servers_to_update = Vec::new();
        {
            let servers_locked = servers.lock().await;
            for (id, server) in &*servers_locked {
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
            let mut servers_locked = servers.lock().await;
            // Remove servers that are not in the update list
            servers_locked.retain(|id, _| {
                servers_to_update
                    .iter()
                    .any(|(update_id, _)| update_id == id)
            });
            // Update servers that are in the update list
            for (id, active_players) in servers_to_update {
                if let Some(server) = servers_locked.get_mut(&id) {
                    server.players = active_players;
                    if server.started {
                        let elapsed_time_update = Utc::now()
                            .signed_duration_since(server.last_update)
                            .num_milliseconds()
                            as f64
                            / 1_000.0;

                        // Check if the elapsed time has crossed a multiple of x seconds, if so give coins to each player
                        let previous_multiple = server.elapsed_time / COINS_EVERY_X_SECONDS;
                        let current_multiple =
                            (server.elapsed_time + elapsed_time_update) / COINS_EVERY_X_SECONDS;
                        if current_multiple.trunc() > previous_multiple.trunc() {
                            for player in server.players.values_mut() {
                                player.coins += 1;
                            }
                        }

                        // Check if the elapsed time has crossed a multiple of x seconds, if so submit a question to the queue
                        let previous_multiple =
                            server.elapsed_time / SUBMIT_QUESTION_EVERY_X_SECONDS;
                        let current_multiple = (server.elapsed_time + elapsed_time_update)
                            / SUBMIT_QUESTION_EVERY_X_SECONDS;
                        if current_multiple.trunc() > previous_multiple.trunc() {
                            items::ask_top_question(server);
                        }

                        // If server item queue is low, add more items
                        let time_since_last_add_to_queue = Utc::now()
                            .signed_duration_since(server.last_add_to_queue)
                            .num_seconds();
                        if server.items_queue.len() < 3 && time_since_last_add_to_queue > 5 {
                            server.last_add_to_queue = Utc::now();
                            let server_id_clone = server.id.clone();
                            tokio::spawn(async move {
                                items::add_item_to_queue(server_id_clone, vec![], 0).await;
                            });
                        }

                        // Update elapsed time and last update time
                        server.elapsed_time += elapsed_time_update;
                        server.last_update = Utc::now();
                    }
                }
            }
        }

        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}
