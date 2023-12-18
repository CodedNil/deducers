use axum::{extract::ws::WebSocketUpgrade, response::Html, routing::get, Router};
use std::{env, time::Duration};
use tower_http::services::ServeDir;

mod backend;
mod connection;
mod lobby_utils;
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

#[tokio::main]
async fn main() {
    lobby_utils::init();

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
            lobby_utils::lobby_loop().await;
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    });

    axum::Server::bind(&addr.to_string().parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
