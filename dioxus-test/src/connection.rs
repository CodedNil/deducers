use anyhow::{anyhow, Result};
use dioxus::prelude::*;
use std::collections::HashMap;
use tokio::time::Instant;

use crate::{Player, Server, SERVERS};

#[allow(clippy::too_many_lines)]
pub fn app(cx: Scope) -> Element {
    let player_name = use_state(cx, String::new);
    let server_id = use_state(cx, String::new);
    let is_connected = use_state(cx, || false);

    let error_message = use_state(cx, || None::<String>);

    let connect = {
        move |_| {
            let player_name = player_name.clone();
            let server_id = server_id.clone();
            let is_connected = is_connected.clone();
            let error_message = error_message.clone();

            if player_name.get() == "" {
                error_message.set(Some("Player name cannot be empty".into()));
                return;
            }
            if server_id.get() == "" {
                error_message.set(Some("Server ID cannot be empty".into()));
                return;
            }

            cx.spawn(async move {
                match connect_player(server_id.get().to_string(), player_name.get().to_string())
                    .await
                {
                    Ok(_) => {
                        is_connected.set(true);
                    }
                    Err(error) => {
                        error_message.set(Some(format!("Failed to connect to server: {error}")));
                    }
                };
            });
        }
    };

    let disconnect = {
        move |_| {
            let player_name = player_name.clone();
            let server_id = server_id.clone();
            player_name.set(String::new());
            server_id.set(String::new());
            is_connected.set(false);

            cx.spawn(async move {
                let _ =
                    disconnect_player(server_id.get().to_string(), player_name.get().to_string())
                        .await;
            });
        }
    };

    // Error dialog rendering
    let render_error_dialog = {
        move || {
            error_message.get().clone().map_or_else(
                || rsx! { div {} },
                |msg| {
                    rsx! {
                        div {
                            position: "fixed",
                            top: "50%",
                            left: "50%",
                            transform: "translate(-50%, -50%)",
                            background_color: "darkred",
                            padding: "20px",
                            border_radius: "10px",
                            color: "white",
                            font_family: "sans-serif",
                            font_weight: "bold",
                            display: "flex",
                            flex_direction: "column",
                            align_items: "center",
                            gap: "10px",
                            "{msg}"
                            button {
                                width: "fit-content",
                                background_color: "#313131",
                                border: "none",
                                border_radius: "5px",
                                padding: "5px 10px",
                                color: "white",
                                font_family: "sans-serif",
                                font_weight: "bold",
                                onclick: move |_| error_message.set(None),
                                "OK"
                            }
                        }
                    }
                },
            )
        }
    };

    if *is_connected.get() {
        cx.render(rsx! {
            div {
                position: "absolute",
                top: "0px",
                left: "0px",
                bottom: "0px",
                right: "0px",

                div {
                    "Server Id: {server_id}"
                    button { onclick: disconnect, "Disconnect" }
                }
            }

            render_error_dialog()
        })
    } else {
        cx.render(rsx! {
            div {
                position: "absolute",
                top: "0px",
                left: "0px",
                bottom: "0px",
                right: "0px",
                background_color: "rgb(30, 30, 30)",

                div {
                    position: "fixed",
                    top: "50%",
                    left: "50%",
                    transform: "translate(-50%, -50%)",
                    background_color: "#2c5e93",
                    padding: "20px",
                    border_radius: "10px",
                    display: "flex",
                    flex_direction: "column",
                    align_items: "center",
                    gap: "10px",

                    div {
                        input {
                            r#type: "text",
                            value: "{player_name}",
                            placeholder: "Player Name",
                            font_family: "sans-serif",
                            font_weight: "bold",
                            oninput: move |e| player_name.set(e.value.clone())
                        }
                    }

                    div {
                        input {
                            r#type: "text",
                            value: "{server_id}",
                            placeholder: "Lobby Id",
                            font_family: "sans-serif",
                            font_weight: "bold",
                            oninput: move |e| server_id.set(e.value.clone())
                        }
                    }

                    button {
                        width: "fit-content",
                        background_color: "#313131",
                        border: "none",
                        border_radius: "5px",
                        padding: "5px 10px",
                        color: "white",
                        font_family: "sans-serif",
                        font_weight: "bold",
                        onclick: connect,
                        "Connect"
                    }
                }
            }

            render_error_dialog()
        })
    }
}

async fn connect_player(server_id: String, player_name: String) -> Result<String> {
    let servers = SERVERS
        .get()
        .ok_or_else(|| anyhow::anyhow!("SERVERS not initialized"))?;
    let mut servers_lock = servers.lock().await;

    // Get the server or create a new one
    let server = servers_lock.entry(server_id.clone()).or_insert_with(|| {
        println!("Creating new server '{server_id}'");

        Server {
            id: server_id.clone(),
            started: false,
            elapsed_time: 0.0,
            last_update: Instant::now(),
            key_player: player_name.clone(),
            players: HashMap::new(),
        }
    });

    // Check if player with the same name is already connected
    if server.players.contains_key(&player_name) {
        return Err(anyhow!(
            "Player '{player_name}' is already connected to server '{server_id}'"
        ));
    }

    // Add the player to the server
    server.players.entry(player_name.clone()).or_insert(Player {
        name: player_name.clone(),
        last_contact: Instant::now(),
        score: 0,
        coins: 3,
    });

    // Return the game state
    println!("Player '{player_name}' connected to server '{server_id}'");
    drop(servers_lock);
    Ok(format!(
        "Player '{player_name}' connected to server '{server_id}'"
    ))
}

async fn disconnect_player(server_id: String, player_name: String) -> Result<String> {
    let servers = SERVERS
        .get()
        .ok_or_else(|| anyhow::anyhow!("SERVERS not initialized"))?;
    let mut servers_lock = servers.lock().await;
    let Some(server) = servers_lock.get_mut(&server_id) else {
        return Err(anyhow!("Server '{server_id}' not found"));
    };

    server.players.remove(&player_name);
    println!("Player '{player_name}' disconnected from server '{server_id}'");
    if player_name == server.key_player {
        servers_lock.remove(&server_id);
        drop(servers_lock);
        println!("Key player left, server '{server_id}' closed");
        return Ok(format!("Key player left, server '{server_id}' closed",));
    }
    drop(servers_lock);
    Ok(format!(
        "Player '{player_name}' disconnected from server '{server_id}'"
    ))
}

async fn get_state(server_id: String, player_name: String) -> Result<String> {
    let servers = SERVERS
        .get()
        .ok_or_else(|| anyhow::anyhow!("SERVERS not initialized"))?;
    let mut servers_lock = servers.lock().await;
    let Some(server) = servers_lock.get_mut(&server_id) else {
        return Err(anyhow!("Server '{server_id}' not found"));
    };
    let Some(player) = server.players.get_mut(&player_name) else {
        return Err(anyhow!("Player '{player_name}' not found"));
    };

    // Update last contact time for the player and convert to minimal server
    player.last_contact = Instant::now();
    drop(servers_lock);

    // Return the entire state of the server
    Ok("Success".into())
}
