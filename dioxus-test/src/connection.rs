use anyhow::{anyhow, Result};
use dioxus::prelude::*;
use std::{collections::HashMap, time::Duration};

use crate::{
    get_current_time, ui::gameview::game_view, Answer, Item, Lobby, Player, Question,
    QueuedQuestion, LOBBYS,
};

#[allow(clippy::too_many_lines)]
pub fn app(cx: Scope) -> Element {
    let player_name: &UseState<String> = use_state(cx, || String::from("dan"));
    let lobby_id: &UseState<String> = use_state(cx, || String::from("test"));
    let is_connected: &UseState<bool> = use_state(cx, || false);

    let lobby_state: &UseState<Option<Lobby>> = use_state(cx, || None::<Lobby>);

    let error_message: &UseState<Option<String>> = use_state(cx, || None::<String>);

    // Get lobby state every x seconds if connected
    if *is_connected.get() {
        use_effect(cx, (), |()| {
            let lobby_state = lobby_state.clone();
            let player_name = player_name.clone();
            let lobby_id = lobby_id.clone();
            let error_message = error_message.clone();
            let is_connected = is_connected.clone();
            async move {
                while *is_connected.get() {
                    match get_state(lobby_id.get().to_string(), player_name.get().to_string()).await
                    {
                        Ok(state_json) => match serde_json::from_str::<Lobby>(&state_json) {
                            Ok(lobby) => {
                                lobby_state.set(Some(lobby));
                            }
                            Err(error) => {
                                error_message
                                    .set(Some(format!("Disconnected from lobby: {error}")));
                                is_connected.set(false);
                                break;
                            }
                        },
                        Err(error) => {
                            error_message.set(Some(format!("Disconnected from lobby: {error}")));
                            is_connected.set(false);
                            break;
                        }
                    }

                    tokio::time::sleep(Duration::from_millis(500)).await;
                }
            }
        });
    }

    let connect = {
        move || {
            let player_name = player_name.clone();
            let lobby_id = lobby_id.clone();
            let is_connected = is_connected.clone();
            let error_message = error_message.clone();

            cx.spawn(async move {
                match connect_player(lobby_id.get().to_string(), player_name.get().to_string())
                    .await
                {
                    Ok(_) => {
                        is_connected.set(true);
                    }
                    Err(error) => {
                        error_message.set(Some(format!("Failed to connect to lobby: {error}")));
                    }
                };
            });
        }
    };

    let disconnect = Box::new(move || {
        let player_name = player_name.clone();
        let lobby_id = lobby_id.clone();
        is_connected.set(false);

        cx.spawn(async move {
            let _ =
                disconnect_player(lobby_id.get().to_string(), player_name.get().to_string()).await;
        });
    });

    // Error dialog rendering
    let show_dialog = error_message.get().is_some();
    let error_msg = error_message.get().clone().unwrap_or_default();
    let render_error_dialog = rsx! {
        div { class: "error-dialog", top: if show_dialog { "50%" } else { "-100%" },
            "{error_msg}"
            button { onclick: move |_| error_message.set(None), "OK" }
        }
    };

    if *is_connected.get() {
        if let Some(lobby) = lobby_state.get() {
            cx.render(rsx! { game_view(cx, disconnect, player_name, lobby_id, lobby), {}, render_error_dialog })
        } else {
            cx.render(rsx! { div { "Loading..." } })
        }
    } else {
        cx.render(rsx! {
            form {
                class: "login-dialog",

                onsubmit: move |_| {
                    connect();
                },

                input {
                    r#type: "text",
                    value: "{player_name}",
                    placeholder: "Player Name",
                    oninput: move |e| {
                        let input = e.value.clone();
                        let filtered_input: String = input
                            .chars()
                            .filter(|c| c.is_alphanumeric())
                            .take(20)
                            .collect();
                        player_name.set(filtered_input);
                    }
                }

                input {
                    r#type: "text",
                    value: "{lobby_id}",
                    placeholder: "Lobby Id",
                    oninput: move |e| {
                        let input = e.value.clone();
                        let filtered_input: String = input
                            .chars()
                            .filter(|c| c.is_alphanumeric())
                            .take(20)
                            .collect();
                        lobby_id.set(filtered_input);
                    }
                }

                button { r#type: "submit", "Connect" }
            }

            render_error_dialog
        })
    }
}

async fn connect_player(lobby_id: String, player_name: String) -> Result<String> {
    if lobby_id.trim() == "" {
        return Err(anyhow!("Lobby ID cannot be empty"));
    }
    if player_name.trim() == "" {
        return Err(anyhow!("Player name cannot be empty"));
    }
    if lobby_id.len() < 3 {
        return Err(anyhow!("Lobby ID must be at least 3 characters long"));
    }
    if player_name.len() < 3 {
        return Err(anyhow!("Player name must be at least 3 characters long"));
    }

    let lobbys = LOBBYS
        .get()
        .ok_or_else(|| anyhow::anyhow!("LOBBYS not initialized"))?;
    let mut lobbys_lock = lobbys.lock().await;

    // Get the lobby or create a new one
    let lobby = lobbys_lock.entry(lobby_id.clone()).or_insert_with(|| {
        println!("Creating new lobby '{lobby_id}'");

        Lobby {
            started: false,
            elapsed_time: 0.0,
            last_update: get_current_time(),
            key_player: player_name.clone(),
            players: HashMap::new(),
            questions_queue: vec![
                QueuedQuestion {
                    player: "Dan".to_string(),
                    question: "Is it alive?".to_string(),
                    anonymous: false,
                    votes: 2,
                },
                QueuedQuestion {
                    player: "Dan".to_string(),
                    question: "Is it smaller than a breadbox?".to_string(),
                    anonymous: false,
                    votes: 1,
                },
            ],
            items: vec![Item {
                name: "Axe".to_string(),
                id: 1,
                questions: vec![Question {
                    id: 1,
                    player: "Dan".to_string(),
                    question: "Is it metallic?".to_string(),
                    anonymous: false,
                    answer: Answer::Yes,
                }],
            }],
            items_history: Vec::new(),
            items_queue: vec![
                "Axe".to_string(),
                "Banana".to_string(),
                "Car".to_string(),
                "Dog".to_string(),
                "Egg".to_string(),
            ],
            last_add_to_queue: 0,
            questions_counter: 0,
        }
    });

    // Check if player with the same name is already connected
    if lobby.players.contains_key(&player_name) {
        drop(lobbys_lock);
        return Err(anyhow!(
            "Player '{player_name}' is already connected to lobby '{lobby_id}'"
        ));
    }

    // Add the player to the lobby
    lobby.players.entry(player_name.clone()).or_insert(Player {
        name: player_name.clone(),
        last_contact: get_current_time(),
        score: 0,
        coins: 3,
        messages: Vec::new(),
    });

    // Return the game state
    println!("Player '{player_name}' connected to lobby '{lobby_id}'");
    Ok(format!(
        "Player '{player_name}' connected to lobby '{lobby_id}'"
    ))
}

async fn disconnect_player(lobby_id: String, player_name: String) -> Result<String> {
    let lobbys = LOBBYS
        .get()
        .ok_or_else(|| anyhow::anyhow!("LOBBYS not initialized"))?;
    let mut lobbys_lock = lobbys.lock().await;
    let Some(lobby) = lobbys_lock.get_mut(&lobby_id) else {
        drop(lobbys_lock);
        return Err(anyhow!("Lobby '{lobby_id}' not found"));
    };

    lobby.players.remove(&player_name);
    println!("Player '{player_name}' disconnected from lobby '{lobby_id}'");
    if player_name == lobby.key_player {
        lobbys_lock.remove(&lobby_id);
        drop(lobbys_lock);
        println!("Key player left, lobby '{lobby_id}' closed");
        return Ok(format!("Key player left, lobby '{lobby_id}' closed",));
    }
    Ok(format!(
        "Player '{player_name}' disconnected from lobby '{lobby_id}'"
    ))
}

async fn get_state(lobby_id: String, player_name: String) -> Result<String> {
    let lobbys = LOBBYS
        .get()
        .ok_or_else(|| anyhow::anyhow!("LOBBYS not initialized"))?;
    let mut lobbys_lock = lobbys.lock().await;
    let Some(lobby) = lobbys_lock.get_mut(&lobby_id) else {
        drop(lobbys_lock);
        return Err(anyhow!("Lobby '{lobby_id}' not found"));
    };
    let Some(player) = lobby.players.get_mut(&player_name) else {
        drop(lobbys_lock);
        return Err(anyhow!("Player '{player_name}' not found"));
    };

    // Update last contact time for the player and convert to minimal lobby
    player.last_contact = get_current_time();

    // Return the entire state of the lobby
    Ok(serde_json::to_string(&lobby)?)
}
