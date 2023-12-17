use axum::{extract::ws::WebSocketUpgrade, response::Html, routing::get, Router};
use std::{
    collections::HashMap,
    sync::{Arc, OnceLock},
    time::Duration,
};
use tokio::sync::Mutex;

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

pub const SCORE_TO_COINS_RATIO: usize = 3;

pub const MAX_QUESTION_LENGTH: usize = 100;

#[derive(Clone, Debug)]
pub struct Lobby {
    started: bool,
    elapsed_time: f64,
    last_update: f64,
    key_player: String,
    players: HashMap<String, Player>,
    questions_queue: Vec<QueuedQuestion>,
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
}

#[derive(Clone, Debug)]
struct Player {
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

#[derive(Clone, Debug)]
enum Answer {
    Yes,
    No,
    Maybe,
}

type LobbyStorage = Arc<Mutex<HashMap<String, Lobby>>>;
pub static LOBBYS: OnceLock<LobbyStorage> = OnceLock::new();

#[tokio::main]
async fn main() {
    // Initialize the LOBBYS global variable
    LOBBYS.get_or_init(|| Arc::new(Mutex::new(HashMap::new())));

    let addr: std::net::SocketAddr = ([127, 0, 0, 1], SERVER_PORT).into();

    // Load style from style.scss
    let style = include_str!("style.css");

    let view = dioxus_liveview::LiveViewPool::new();
    let index_page_with_glue = |glue: &str, style: &str| {
        Html(format!(
            r#"
        <!DOCTYPE html>
        <html>
            <head>
                <title>Deducers</title>
                <meta name="darkreader-lock">
                <style>{style}</style>
            </head>
            <body> <div id="main"></div> </body>
            {glue}
        </html>
        "#,
        ))
    };

    let app = Router::new()
        .route(
            "/",
            get(move || async move {
                index_page_with_glue(
                    &dioxus_liveview::interpreter_glue(&format!("ws://{addr}/ws")),
                    style,
                )
            }),
        )
        .route(
            "/as-path",
            get(move || async move {
                index_page_with_glue(&dioxus_liveview::interpreter_glue("/ws"), style)
            }),
        )
        .route(
            "/ws",
            get(move |ws: WebSocketUpgrade| async move {
                ws.on_upgrade(move |socket| async move {
                    _ = view
                        .launch(dioxus_liveview::axum_socket(socket), connection::app)
                        .await;
                })
            }),
        );

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

#[must_use]
pub fn get_current_time() -> f64 {
    let now = std::time::SystemTime::now();
    now.duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
}

#[must_use]
pub fn get_time_diff(start: f64) -> f64 {
    get_current_time() - start
}

#[must_use]
pub fn filter_input(input: &str, max_length: usize, allow_spaces: bool) -> String {
    input
        .chars()
        .filter(|c| c.is_alphanumeric() || (allow_spaces && *c == ' '))
        .take(max_length)
        .collect()
}
