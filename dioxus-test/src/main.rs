use axum::{extract::ws::WebSocketUpgrade, response::Html, routing::get, Router};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{Arc, OnceLock},
    time::Duration,
};
use tokio::sync::Mutex;

mod connection;
mod gameview;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Lobby {
    #[serde(skip_serializing, default)]
    id: String,
    started: bool,
    elapsed_time: f64,
    #[serde(skip_serializing, default)]
    last_update: u128,
    key_player: String,
    players: HashMap<String, Player>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Player {
    name: String,
    #[serde(skip_serializing, default)]
    last_contact: u128,
    score: usize,
    coins: usize,
}

type LobbyStorage = Arc<Mutex<HashMap<String, Lobby>>>;
pub static LOBBYS: OnceLock<LobbyStorage> = OnceLock::new();

#[tokio::main]
async fn main() {
    // Initialize the LOBBYS global variable
    LOBBYS.get_or_init(|| Arc::new(Mutex::new(HashMap::new())));

    let addr: std::net::SocketAddr = ([127, 0, 0, 1], 3030).into();

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
        server_loop().await;
    });

    axum::Server::bind(&addr.to_string().parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

#[must_use]
pub fn get_current_time() -> u128 {
    let now = std::time::SystemTime::now();
    now.duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

#[allow(clippy::significant_drop_in_scrutinee, clippy::cast_precision_loss)]
async fn server_loop() {
    loop {
        let lobbys = LOBBYS.get().unwrap();
        let mut lobbys_lock = lobbys.lock().await;

        // Increment elapsed time for each lobby
        for lobby in lobbys_lock.values_mut() {
            lobby.elapsed_time += (get_current_time() - lobby.last_update) as f64 / 1000.0;
            lobby.last_update = get_current_time();
        }

        drop(lobbys_lock);

        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}
