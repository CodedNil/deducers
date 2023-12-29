#![warn(clippy::nursery, clippy::pedantic)]
#![allow(clippy::too_many_lines)]
use crate::ui::connection::app;
use axum::{extract::ws::WebSocketUpgrade, response::Html, routing::get, Router};
use std::{env, net::SocketAddr, time::Duration};
use tokio::time::sleep;
use tower_http::services::ServeDir;

mod backend;
mod lobby_utils;
mod ui;

pub const SERVER_PORT: u16 = 3013;

pub const IDLE_KICK_TIME: f64 = 10.0;

pub const MAX_QUESTION_LENGTH: usize = 70;
pub const QUESTION_PATTERN: &str = "^[a-zA-Z0-9 ?]+$"; // Alphanumeric and spaces and question mark only

pub const MAX_ITEM_NAME_LENGTH: usize = 30;
pub const ITEM_NAME_PATTERN: &str = "^[a-zA-Z]+$"; // Alphabetic only

pub const MAX_LOBBY_ID_LENGTH: usize = 20;
pub const LOBBY_ID_PATTERN: &str = "^[a-zA-Z0-9]+$"; // Alphanumeric only
pub const MAX_PLAYER_NAME_LENGTH: usize = 20;
pub const PLAYER_NAME_PATTERN: &str = "^[a-zA-Z0-9 ]+$"; // Alphanumeric and spaces only

pub const MAX_LOBBY_ITEMS: usize = 20;

pub const MAX_CHAT_LENGTH: usize = 100;
pub const MAX_CHAT_MESSAGES: usize = 20;

#[tokio::main]
async fn main() {
    // Get the server IP from an environment variable or default to localhost
    let addr: SocketAddr = ([0, 0, 0, 0], SERVER_PORT).into();
    let server_ip = env::var("SERVER_IP").unwrap_or_else(|_| "127.0.0.1".to_owned());
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
                <link rel="icon" href="/assets/favicon.ico" type="image/x-icon">
                <link rel="stylesheet" type="text/css" href="assets/style.css">
            </head>
            <body>
                <div id="main"></div>
            </body>
            {glue}
            <script src="assets/client.js"></script>
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
                    _ = view.launch(dioxus_liveview::axum_socket(socket), app).await;
                })
            }),
        )
        .nest_service("/assets/", ServeDir::new("assets"));

    println!("Listening on http://{addr}");

    tokio::spawn(async move {
        loop {
            lobby_utils::lobby_loop();
            sleep(Duration::from_millis(500)).await;
        }
    });

    axum::Server::bind(&addr.to_string().parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
