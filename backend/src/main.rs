use axum::{
    extract::Path,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Extension, Json, Router,
};
use serde::Serialize;
use std::{collections::HashMap, net::SocketAddr, sync::Arc, time::Duration};
use tokio::{net::TcpListener, sync::Mutex, time::Instant};

mod game_state;
mod items;
mod openai;
mod question_queue;

pub const SERVER_PORT: u16 = 3013;

pub const IDLE_KICK_TIME: u64 = 10;

pub const COINS_EVERY_X_SECONDS: f64 = 4.0;
pub const SUBMIT_QUESTION_EVERY_X_SECONDS: f64 = 10.0;
pub const ADD_ITEM_EVERY_X_QUESTIONS: usize = 5;

pub const SUBMIT_QUESTION_COST: usize = 4;
pub const ANONYMOUS_QUESTION_COST: usize = 8;
pub const VOTE_QUESTION_COST: usize = 1;
pub const GUESS_ITEM_COST: usize = 3;

pub const SCORE_TO_COINS_RATIO: usize = 3;

#[derive(Clone, Debug)]
pub struct Server {
    id: String,
    started: bool,
    elapsed_time: f64,
    last_update: Instant,
    key_player: String,
    players: HashMap<String, Player>,
    questions_queue: Vec<QueuedQuestion>,
    items: Vec<Item>,
    items_history: Vec<String>,
    items_queue: Vec<String>,
    last_add_to_queue: Instant,
    questions_counter: usize,
}

#[derive(Clone, Debug, Serialize)]
pub enum PlayerMessage {
    ItemAdded,
    QuestionAsked,
    GameStart,
    CoinGiven,
    ItemGuessed(String, usize, String),
}

#[derive(Clone, Debug)]
struct Player {
    name: String,
    last_contact: Instant,
    score: usize,
    coins: usize,
    messages: Vec<PlayerMessage>,
}

#[derive(Clone, Debug)]
struct QueuedQuestion {
    player: String,
    question: String,
    anonymous: bool,
    votes: usize,
}

#[derive(Clone, Debug)]
struct Item {
    name: String,
    id: usize,
    questions: Vec<Question>,
}

#[derive(Clone, Debug)]
struct Question {
    player: String,
    id: usize,
    question: String,
    answer: Answer,
    anonymous: bool,
}

#[derive(Clone, Debug, Serialize)]
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
        .route("/server/:server_id/connect/:player_name", post(connect_player))
        .route("/server/:server_id/disconnect/:player_name", post(disconnect_player))
        .route("/server/:server_id/start/:player_name", post(start_server))
        .route("/server/:server_id/getstate/:player_name", get(game_state::get_state))
        .route(
            "/server/:server_id/submitquestion/:player_name/:question/:options",
            post(question_queue::player_submit_question),
        )
        .route(
            "/server/:server_id/votequestion/:player_name/:question",
            post(question_queue::player_vote_question),
        )
        .route("/server/:server_id/guessitem/:player_name/:itemchoice/:guess", post(items::player_guess_item))
        .route("/server/:server_id/convertscore/:player_name", post(question_queue::player_convert_score))
        .route("/server/:server_id/kickplayer/:player_name/:kick_player", post(kick_player))
        .route("/internal/:server_id/additemqueued/:item_name", post(items::add_item_to_server_queue))
        .layer(Extension(servers))
        .into_make_service_with_connect_info::<SocketAddr>();

    // Server setup
    let address = format!("0.0.0.0:{SERVER_PORT}");
    let listener = TcpListener::bind(&address).await.unwrap();
    println!("Server running on {address}");
    axum::serve(listener, app).await.unwrap();
}

async fn connect_player(Path((server_id, player_name)): Path<(String, String)>, Extension(servers): Extension<ServerStorage>) -> impl IntoResponse {
    let mut servers_lock = servers.lock().await;

    // Get the server or create a new one
    let server = servers_lock.entry(server_id.clone()).or_insert_with(|| {
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
            last_update: Instant::now(),
            key_player: player_name.clone(),
            players: HashMap::new(),
            questions_queue: Vec::new(),
            items: Vec::new(),
            items_history: Vec::new(),
            items_queue: Vec::new(),
            last_add_to_queue: Instant::now(),
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
        last_contact: Instant::now(),
        score: 0,
        coins: 3,
        messages: Vec::new(),
    });

    // Return the game state
    println!("Player '{player_name}' connected to server '{server_id}'");
    let minimal_server = game_state::convert_to_minimal(server, &player_name);
    drop(servers_lock);
    (StatusCode::OK, Json(game_state::Response::ServerState(minimal_server)))
}

async fn disconnect_player(Path((server_id, player_name)): Path<(String, String)>, Extension(servers): Extension<ServerStorage>) -> impl IntoResponse {
    let mut servers_lock = servers.lock().await;
    let Some(server) = servers_lock.get_mut(&server_id) else {
        return (StatusCode::NOT_FOUND, "Server not found".to_string());
    };

    server.players.remove(&player_name);
    println!("Player '{player_name}' disconnected from server '{server_id}'");
    if player_name == server.key_player {
        servers_lock.remove(&server_id);
        drop(servers_lock);
        println!("Key player left, server '{server_id}' closed");
        return (StatusCode::OK, format!("Key player left, server '{server_id}' closed"));
    }
    drop(servers_lock);
    (StatusCode::OK, format!("Player '{player_name}' disconnected from server '{server_id}'"))
}

async fn start_server(Path((server_id, player_name)): Path<(String, String)>, Extension(servers): Extension<ServerStorage>) -> impl IntoResponse {
    let mut servers_lock = servers.lock().await;
    let Some(server) = servers_lock.get_mut(&server_id) else {
        return (StatusCode::NOT_FOUND, "Server not found".to_string());
    };

    if server.started {
        return (StatusCode::BAD_REQUEST, "Server already started".to_string());
    } else if player_name != server.key_player {
        return (StatusCode::FORBIDDEN, "Only the key player can start the server".to_string());
    } else if server.items_queue.is_empty() {
        return (StatusCode::BAD_REQUEST, "No items in queue".to_string());
    }
    server.started = true;
    server.last_update = Instant::now();

    // Send message to all players of game started
    for player in server.players.values_mut() {
        player.messages.push(PlayerMessage::GameStart);
    }

    // Add 2 items to the server
    items::add_item_to_server(server);
    items::add_item_to_server(server);
    drop(servers_lock);

    println!("Server '{server_id}' started by key player '{player_name}'");
    (StatusCode::OK, format!("Server '{server_id}' started by key player '{player_name}'"))
}

pub async fn kick_player(
    Path((server_id, player_name, kick_player_name)): Path<(String, String, String)>,
    Extension(servers): Extension<ServerStorage>,
) -> impl IntoResponse {
    let mut servers_lock = servers.lock().await;
    let Some(server) = servers_lock.get_mut(&server_id) else {
        return (StatusCode::NOT_FOUND, "Server not found".to_string());
    };

    if player_name != server.key_player {
        return (StatusCode::FORBIDDEN, "Only the key player can kick other players".to_string());
    }

    server.players.remove(&kick_player_name);
    drop(servers_lock);

    println!("Server '{server_id}' player '{kick_player_name}' kicked by key player");
    (StatusCode::OK, format!("Server '{server_id}' player '{kick_player_name}' kicked by key player"))
}

async fn server_loop(servers: ServerStorage) {
    loop {
        let mut servers_locked = servers.lock().await;

        // Iterate through servers to update or remove
        servers_locked.retain(|id, server| {
            // Remove inactive players and check if key player is active
            server.players.retain(|player_id, player| {
                if player.last_contact.elapsed().as_secs() > IDLE_KICK_TIME {
                    // Log player kicking due to inactivity
                    println!("Kicking player '{player_id}' due to idle");
                    false
                } else {
                    true
                }
            });
            let key_player_active = server.players.contains_key(&server.key_player);

            // Remove server if key player is inactive or no players are left
            if !key_player_active || server.players.is_empty() {
                println!("Removing server '{id}' due to no key player or no players");
                false
            } else {
                // Update server state if server is started
                if server.started {
                    let elapsed_time_update = server.last_update.elapsed().as_secs_f64();

                    // Distribute coins if the elapsed time has crossed a multiple of COINS_EVERY_X_SECONDS
                    let previous_coin_multiple = server.elapsed_time / COINS_EVERY_X_SECONDS;
                    let current_coin_multiple = (server.elapsed_time + elapsed_time_update) / COINS_EVERY_X_SECONDS;
                    if current_coin_multiple.trunc() > previous_coin_multiple.trunc() {
                        for player in server.players.values_mut() {
                            player.coins += 1;
                            player.messages.push(PlayerMessage::CoinGiven);
                        }
                    }

                    // Submit a question to the queue if the elapsed time has crossed a multiple of SUBMIT_QUESTION_EVERY_X_SECONDS
                    let previous_question_multiple = server.elapsed_time / SUBMIT_QUESTION_EVERY_X_SECONDS;
                    let current_question_multiple = (server.elapsed_time + elapsed_time_update) / SUBMIT_QUESTION_EVERY_X_SECONDS;
                    if current_question_multiple.trunc() > previous_question_multiple.trunc() {
                        let server_id_clone = server.id.clone();
                        let servers_clone = servers.clone();
                        tokio::spawn(async move {
                            items::ask_top_question(servers_clone, server_id_clone).await;
                        });
                    }

                    // Add more items to the server's item queue if it's low
                    let time_since_last_add_to_queue = server.last_add_to_queue.elapsed().as_secs();
                    if server.items_queue.len() < 3 && time_since_last_add_to_queue > 5 {
                        server.last_add_to_queue = Instant::now();
                        let server_id_clone = server.id.clone();
                        tokio::spawn(async move {
                            items::add_item_to_queue(server_id_clone, vec![], 0).await;
                        });
                    }

                    // Update the elapsed time and last update time for the server
                    server.elapsed_time += elapsed_time_update;
                    server.last_update = Instant::now();
                }
                true
            }
        });
        drop(servers_locked);

        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}
