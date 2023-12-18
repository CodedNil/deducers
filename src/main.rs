use anyhow::Result;
use axum::{extract::ws::WebSocketUpgrade, response::Html, routing::get, Router};
use std::{
    collections::HashMap,
    env,
    sync::{Arc, OnceLock},
    time::Duration,
};
use tokio::sync::Mutex;
use tower_http::services::ServeDir;

mod backend;
mod connection;
mod ui;

pub const SERVER_PORT: u16 = 3013;

pub const IDLE_KICK_TIME: f64 = 10.0;

pub const STARTING_COINS: usize = 4;
pub const COINS_EVERY_X_SECONDS: f64 = 4.0;
pub const SUBMIT_QUESTION_EVERY_X_SECONDS: f64 = 10.0;
pub const ADD_ITEM_EVERY_X_QUESTIONS: usize = 5;

pub const SUBMIT_QUESTION_COST: usize = 4;
pub const ANONYMOUS_QUESTION_COST: usize = 8;
pub const GUESS_ITEM_COST: usize = 3;
pub const QUESTION_MIN_VOTES: usize = 2;

pub const SCORE_TO_COINS_RATIO: usize = 3;

pub const MAX_QUESTION_LENGTH: usize = 70;
pub const QUESTION_PATTERN: &str = "^[a-zA-Z0-9 ?]+$"; // Alphanumeric and spaces and question mark only
pub const MAX_GUESS_ITEM_LENGTH: usize = 20;
pub const GUESS_ITEM_PATTERN: &str = "^[a-zA-Z0-9 ]+$"; // Alphanumeric and spaces only
pub const MAX_LOBBY_ID_LENGTH: usize = 20;
pub const LOBBY_ID_PATTERN: &str = "^[a-zA-Z0-9]+$"; // Alphanumeric only
pub const MAX_PLAYER_NAME_LENGTH: usize = 20;
pub const PLAYER_NAME_PATTERN: &str = "^[a-zA-Z0-9 ]+$"; // Alphanumeric and spaces only

#[derive(Clone, Debug)]
pub struct Lobby {
    started: bool,
    elapsed_time: f64,
    last_update: f64,
    key_player: String,
    players: HashMap<String, Player>,
    questions_queue: Vec<QueuedQuestion>,
    questions_queue_waiting: bool,
    questions_queue_countdown: f64,
    items: Vec<Item>,
    items_history: Vec<String>,
    items_queue: Vec<String>,
    last_add_to_queue: f64,
    questions_counter: usize,
}

#[derive(Clone, Debug)]
pub enum PlayerMessage {
    ItemAdded,
    QuestionAsked,
    GameStart,
    CoinGiven,
    ItemGuessed(String, usize, String),
    GuessIncorrect,
    ItemRemoved(usize, String),
}

#[derive(Clone, Debug)]
pub struct Player {
    name: String,
    last_contact: f64,
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

#[derive(Clone, Debug, PartialEq)]
struct Item {
    name: String,
    id: usize,
    questions: Vec<Question>,
}

#[derive(Clone, Debug, PartialEq)]
struct Question {
    player: String,
    id: usize,
    question: String,
    answer: Answer,
    anonymous: bool,
}

#[derive(Clone, Debug, PartialEq)]
enum Answer {
    Yes,
    No,
    Maybe,
}

pub static LOBBYS: OnceLock<Arc<Mutex<HashMap<String, Lobby>>>> = OnceLock::new();

#[tokio::main]
async fn main() {
    LOBBYS.get_or_init(|| Arc::new(Mutex::new(HashMap::new())));

    // Get the server IP from an environment variable or default to localhost
    let addr: std::net::SocketAddr = ([0, 0, 0, 0], SERVER_PORT).into();
    let server_ip = env::var("SERVER_IP").unwrap_or_else(|_| "127.0.0.1".to_string());
    let server_address = format!("{server_ip}:{SERVER_PORT}");

    let view = dioxus_liveview::LiveViewPool::new();
    let index_page_with_glue = |glue: &str| {
        Html(format!(
            r#"
        <!DOCTYPE html>
        <html>
            <head>
                <title>Deducers</title>
                <meta name="darkreader-lock">
                <link rel="stylesheet" type="text/css" href="assets/style.css">
            </head>
            <body>
                <div id="main"></div>
            </body>
            {glue}
            <script src="assets/sounds.js"></script>
        </html>
        "#,
        ))
    };

    let app = Router::new()
        .route(
            "/",
            get(move || async move { index_page_with_glue(&dioxus_liveview::interpreter_glue(&format!("ws://{server_address}/ws"))) }),
        )
        .route(
            "/as-path",
            get(move || async move { index_page_with_glue(&dioxus_liveview::interpreter_glue("/ws")) }),
        )
        .route(
            "/ws",
            get(move |ws: WebSocketUpgrade| async move {
                ws.on_upgrade(move |socket| async move {
                    _ = view.launch(dioxus_liveview::axum_socket(socket), connection::app).await;
                })
            }),
        )
        .nest_service("/assets/", ServeDir::new("assets"));

    println!("Listening on http://{addr}");

    tokio::spawn(async move {
        loop {
            connection::lobby_loop().await;
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    });

    axum::Server::bind(&addr.to_string().parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

#[allow(clippy::missing_errors_doc, clippy::future_not_send)]
pub async fn with_lobby_mut<F, T>(lobby_id: &str, f: F) -> Result<T>
where
    F: FnOnce(&mut Lobby) -> Result<T>,
{
    let lobbys = LOBBYS.get().ok_or_else(|| anyhow::anyhow!("LOBBYS not initialized"))?;
    let mut lobbys_lock = lobbys.lock().await;

    let lobby = lobbys_lock
        .get_mut(lobby_id)
        .ok_or_else(|| anyhow::anyhow!("Lobby '{lobby_id}' not found"))?;

    let result = f(lobby);
    drop(lobbys_lock);
    result
}

#[allow(clippy::missing_errors_doc, clippy::future_not_send)]
pub async fn with_player<F, T>(lobby_id: &str, player_name: &str, f: F) -> Result<T>
where
    F: FnOnce(Lobby, Player) -> Result<T>,
{
    let lobbys = LOBBYS.get().ok_or_else(|| anyhow::anyhow!("LOBBYS not initialized"))?;
    let lobbys_lock = lobbys.lock().await;

    let lobby = lobbys_lock
        .get(lobby_id)
        .ok_or_else(|| anyhow::anyhow!("Lobby '{lobby_id}' not found"))?;
    let lobby_state = lobby.clone();
    let player = lobby
        .players
        .get(player_name)
        .ok_or_else(|| anyhow::anyhow!("Player '{player_name}' not found"))?;

    let result = f(lobby_state, player.clone());
    drop(lobbys_lock);
    result
}

#[allow(clippy::missing_errors_doc, clippy::future_not_send)]
pub async fn with_player_mut<F, T>(lobby_id: &str, player_name: &str, f: F) -> Result<T>
where
    F: FnOnce(Lobby, &mut Player) -> Result<T>,
{
    let lobbys = LOBBYS.get().ok_or_else(|| anyhow::anyhow!("LOBBYS not initialized"))?;
    let mut lobbys_lock = lobbys.lock().await;

    let lobby = lobbys_lock
        .get_mut(lobby_id)
        .ok_or_else(|| anyhow::anyhow!("Lobby '{lobby_id}' not found"))?;
    let lobby_state = lobby.clone();
    let player = lobby
        .players
        .get_mut(player_name)
        .ok_or_else(|| anyhow::anyhow!("Player '{player_name}' not found"))?;

    let result = f(lobby_state, player);
    drop(lobbys_lock);
    result
}

#[must_use]
pub fn get_current_time() -> f64 {
    let now = std::time::SystemTime::now();
    now.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs_f64()
}

#[must_use]
pub fn get_time_diff(start: f64) -> f64 {
    get_current_time() - start
}
