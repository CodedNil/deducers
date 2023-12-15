use axum::{extract::ws::WebSocketUpgrade, response::Html, routing::get, Router};
use std::{
    collections::HashMap,
    sync::{Arc, OnceLock},
};
use tokio::{sync::Mutex, time::Instant};

mod connection;

#[derive(Clone, Debug)]
pub struct Server {
    id: String,
    started: bool,
    elapsed_time: f64,
    last_update: Instant,
    key_player: String,
    players: HashMap<String, Player>,
}

#[derive(Clone, Debug)]
pub struct Player {
    name: String,
    last_contact: Instant,
    score: usize,
    coins: usize,
}

type ServerStorage = Arc<Mutex<HashMap<String, Server>>>;
pub static SERVERS: OnceLock<ServerStorage> = OnceLock::new();

#[tokio::main]
async fn main() {
    // Initialize the SERVERS global variable
    SERVERS.get_or_init(|| Arc::new(Mutex::new(HashMap::new())));

    let addr: std::net::SocketAddr = ([127, 0, 0, 1], 3030).into();

    let view = dioxus_liveview::LiveViewPool::new();
    let index_page_with_glue = |glue: &str| {
        Html(format!(
            r#"
        <!DOCTYPE html>
        <html>
            <head> <title>Deducers</title>  </head>
            <meta name="darkreader-lock">
            <body> <div id="main"></div> </body>
            {glue}
        </html>
        "#,
        ))
    };

    let app =
        Router::new()
            .route(
                "/",
                get(move || async move {
                    index_page_with_glue(&dioxus_liveview::interpreter_glue(&format!(
                        "ws://{addr}/ws"
                    )))
                }),
            )
            .route(
                "/as-path",
                get(move || async move {
                    index_page_with_glue(&dioxus_liveview::interpreter_glue("/ws"))
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

    axum::Server::bind(&addr.to_string().parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
