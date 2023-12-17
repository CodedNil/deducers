use crate::{
    backend::items, filter_input, get_current_time, get_time_diff, ui::gameview::game_view, Lobby,
    Player, PlayerMessage, COINS_EVERY_X_SECONDS, IDLE_KICK_TIME, LOBBYS, STARTING_COINS,
    SUBMIT_QUESTION_EVERY_X_SECONDS,
};
use anyhow::{anyhow, Result};
use dioxus::prelude::*;
use std::{collections::HashMap, time::Duration};

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
                        Ok(lobby) => {
                            lobby_state.set(Some(lobby));
                        }
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
            lobby_state.set(None);

            cx.spawn(async move {
                match connect_player(lobby_id.get().to_string(), player_name.get().to_string())
                    .await
                {
                    Ok(()) => {
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
        lobby_state.set(None);

        cx.spawn(async move {
            let _result =
                disconnect_player(lobby_id.get().to_string(), player_name.get().to_string()).await;
        });
    });

    let start = Box::new(move || {
        let player_name = player_name.clone();
        let lobby_id = lobby_id.clone();

        cx.spawn(async move {
            let _result =
                start_lobby(lobby_id.get().to_string(), player_name.get().to_string()).await;
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
            cx.render(rsx! {game_view(cx, player_name, lobby_id, lobby, disconnect, start), {}, render_error_dialog})
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
                        player_name.set(filter_input(&e.value, 30, true));
                    }
                }

                input {
                    r#type: "text",
                    value: "{lobby_id}",
                    placeholder: "Lobby Id",
                    oninput: move |e| {
                        lobby_id.set(filter_input(&e.value, 20, false));
                    }
                }

                button { r#type: "submit", "Connect" }
            }

            render_error_dialog
        })
    }
}

async fn connect_player(lobby_id: String, player_name: String) -> Result<()> {
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

        // Add initial items to the lobbys queue
        let lobby_id_clone = lobby_id.clone();
        tokio::spawn(async move {
            items::add_item_to_queue(lobby_id_clone, vec![], 0).await;
        });

        Lobby {
            started: false,
            elapsed_time: 0.0,
            last_update: get_current_time(),
            key_player: player_name.clone(),
            players: HashMap::new(),
            questions_queue: Vec::new(),
            items: Vec::new(),
            items_history: Vec::new(),
            items_queue: Vec::new(),
            last_add_to_queue: 0.0,
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
        coins: STARTING_COINS,
        messages: Vec::new(),
    });

    println!("Player '{player_name}' connected to lobby '{lobby_id}'");
    Ok(())
}

async fn disconnect_player(lobby_id: String, player_name: String) -> Result<()> {
    let lobbys = LOBBYS
        .get()
        .ok_or_else(|| anyhow::anyhow!("LOBBYS not initialized"))?;
    let mut lobbys_lock = lobbys.lock().await;

    let lobby = lobbys_lock
        .get_mut(&lobby_id)
        .ok_or_else(|| anyhow::anyhow!("Lobby '{lobby_id}' not found"))?;

    lobby.players.remove(&player_name);
    println!("Player '{player_name}' disconnected from lobby '{lobby_id}'");
    if player_name == lobby.key_player {
        lobbys_lock.remove(&lobby_id);
        drop(lobbys_lock);
        println!("Key player left, lobby '{lobby_id}' closed");
    }
    Ok(())
}

async fn start_lobby(lobby_id: String, player_name: String) -> Result<()> {
    let lobbys = LOBBYS
        .get()
        .ok_or_else(|| anyhow::anyhow!("LOBBYS not initialized"))?;
    let mut lobbys_lock = lobbys.lock().await;

    let lobby = lobbys_lock
        .get_mut(&lobby_id)
        .ok_or_else(|| anyhow::anyhow!("Lobby '{lobby_id}' not found"))?;

    if lobby.started {
        return Err(anyhow!("Lobby '{lobby_id}' already started"));
    } else if player_name != lobby.key_player {
        return Err(anyhow!(
            "Only the key player can start the lobby '{lobby_id}'",
        ));
    } else if lobby.items_queue.is_empty() {
        return Err(anyhow!(
            "Not enough items in queue to start lobby '{lobby_id}'",
        ));
    }
    lobby.started = true;
    lobby.last_update = get_current_time();

    // Send message to all players of game started
    for player in lobby.players.values_mut() {
        player.messages.push(PlayerMessage::GameStart);
    }

    // Add 2 items to the lobby
    items::add_item_to_lobby(lobby);
    items::add_item_to_lobby(lobby);
    drop(lobbys_lock);

    println!("Lobby '{lobby_id}' started by key player '{player_name}'");
    Ok(())
}

pub async fn kick_player(lobby_id: String, player_name: String) -> Result<()> {
    let lobbys = LOBBYS
        .get()
        .ok_or_else(|| anyhow::anyhow!("LOBBYS not initialized"))?;
    let mut lobbys_lock = lobbys.lock().await;

    let lobby = lobbys_lock
        .get_mut(&lobby_id)
        .ok_or_else(|| anyhow::anyhow!("Lobby '{lobby_id}' not found"))?;

    lobby.players.remove(&player_name);
    drop(lobbys_lock);

    println!("Lobby '{lobby_id}' player '{player_name}' kicked by key player");
    Ok(())
}

async fn get_state(lobby_id: String, player_name: String) -> Result<Lobby> {
    let lobbys = LOBBYS
        .get()
        .ok_or_else(|| anyhow::anyhow!("LOBBYS not initialized"))?;
    let mut lobbys_lock = lobbys.lock().await;

    let lobby = lobbys_lock
        .get_mut(&lobby_id)
        .ok_or_else(|| anyhow::anyhow!("Lobby '{lobby_id}' not found"))?;
    let player = lobby
        .players
        .get_mut(&player_name)
        .ok_or_else(|| anyhow::anyhow!("Player '{player_name}' not found"))?;

    // Update last contact time for the player and convert to minimal lobby
    player.last_contact = get_current_time();

    // Return the entire state of the lobby
    let lobby = lobby.clone();
    drop(lobbys_lock);
    Ok(lobby)
}

#[allow(clippy::cast_precision_loss)]
pub async fn lobby_loop() {
    let lobbys = LOBBYS.get().unwrap();
    let mut lobbys_lock = lobbys.lock().await;

    // Iterate through lobbys to update or remove
    lobbys_lock.retain(|lobby_id, lobby| {
        let current_time = get_current_time();

        // Remove inactive players and check if key player is active
        lobby.players.retain(|player_id, player| {
            if get_time_diff(player.last_contact) > IDLE_KICK_TIME {
                // Log player kicking due to inactivity
                println!("Kicking player '{player_id}' due to idle");
                false
            } else {
                true
            }
        });
        let key_player_active = lobby.players.contains_key(&lobby.key_player);

        // Remove lobby if key player is inactive or no players are left
        if !key_player_active || lobby.players.is_empty() {
            println!("Removing lobby '{lobby_id}' due to no key player or no players");
            false
        } else {
            // Update lobby state if lobby is started
            if lobby.started {
                let elapsed_time_update = get_time_diff(lobby.last_update);

                // Distribute coins if the elapsed time has crossed a multiple of COINS_EVERY_X_SECONDS
                let previous_coin_multiple = lobby.elapsed_time / COINS_EVERY_X_SECONDS;
                let current_coin_multiple =
                    (lobby.elapsed_time + elapsed_time_update) / COINS_EVERY_X_SECONDS;
                if current_coin_multiple.trunc() > previous_coin_multiple.trunc() {
                    for player in lobby.players.values_mut() {
                        player.coins += 1;
                        player.messages.push(PlayerMessage::CoinGiven);
                    }
                }

                // Submit a question to the queue if the elapsed time has crossed a multiple of SUBMIT_QUESTION_EVERY_X_SECONDS
                let previous_question_multiple =
                    lobby.elapsed_time / SUBMIT_QUESTION_EVERY_X_SECONDS;
                let current_question_multiple =
                    (lobby.elapsed_time + elapsed_time_update) / SUBMIT_QUESTION_EVERY_X_SECONDS;
                if current_question_multiple.trunc() > previous_question_multiple.trunc() {
                    let lobby_id_clone = lobby_id.clone();
                    tokio::spawn(async move {
                        let _result = items::ask_top_question(lobby_id_clone).await;
                    });
                }

                // Add more items to the lobby's item queue if it's low
                let time_since_last_add_to_queue = get_time_diff(lobby.last_add_to_queue);
                if lobby.items_queue.len() < 3 && time_since_last_add_to_queue > 5.0 {
                    lobby.last_add_to_queue = current_time;
                    let lobby_id_clone = lobby_id.clone();
                    let history_clone = lobby.items_history.clone();
                    tokio::spawn(async move {
                        items::add_item_to_queue(lobby_id_clone, history_clone, 0).await;
                    });
                }

                // Update the elapsed time and last update time for the lobby
                lobby.elapsed_time += elapsed_time_update;
                lobby.last_update = current_time;
            }
            true
        }
    });
}
