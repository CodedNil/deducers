use crate::{
    lobby_utils::{
        connect_player, disconnect_player, get_current_time, get_lobby_info, get_state, start_lobby, Lobby, LobbyInfo, PlayerMessage,
    },
    ui::gameview::game_view,
    LOBBY_ID_PATTERN, MAX_LOBBY_ID_LENGTH, MAX_PLAYER_NAME_LENGTH, PLAYER_NAME_PATTERN,
};
use dioxus::prelude::*;
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

#[derive(Default)]
struct ItemRevealMessage {
    show: bool,
    expiry: f64,
    correct: bool,
    str: String,
}

impl ItemRevealMessage {
    fn render(&self) -> LazyNodes<'_, '_> {
        let item_reveal_correct_class = if self.correct { "correct" } else { "incorrect" };
        rsx! {div { class: "dialog floating item-reveal {item_reveal_correct_class}", top: if self.show { "20%" } else { "-100%" }, "{self.str}" }}
    }
}

#[derive(Default)]
struct ErrorDialog {
    show: bool,
    str: String,
}

#[derive(Default, Clone, PartialEq)]
struct SoundsQueue {
    expiry: f64,
    sound: String,
}

#[allow(clippy::too_many_lines, clippy::cast_possible_wrap)]
pub fn app(cx: Scope) -> Element {
    let player_name: &UseState<String> = use_state(cx, String::new);
    let lobby_id: &UseState<String> = use_state(cx, String::new);
    let is_connected: &UseState<bool> = use_state(cx, || false);

    let lobby_state: &UseState<Option<Lobby>> = use_state(cx, || None::<Lobby>);
    let lobby_info: &UseState<Vec<LobbyInfo>> = use_state(cx, Vec::new);

    let error_message: &UseState<ErrorDialog> = use_state(cx, ErrorDialog::default);

    // Hide the item reveal message after 5 seconds
    let item_reveal_message: &UseState<ItemRevealMessage> = use_state(cx, ItemRevealMessage::default);
    if item_reveal_message.get().show && item_reveal_message.get().expiry > get_current_time() {
        item_reveal_message.set(ItemRevealMessage {
            show: false,
            expiry: 0.0,
            correct: item_reveal_message.get().correct,
            str: item_reveal_message.get().str.clone(),
        });
    }

    // Remove expired sounds
    let sounds_to_play: &UseState<Vec<SoundsQueue>> = use_state(cx, Vec::new);
    let sounds_to_play_vec = sounds_to_play.get().clone();
    let new_sounds_to_play_vec = sounds_to_play_vec
        .iter()
        .filter(|sound| sound.expiry > get_current_time())
        .cloned()
        .collect();
    if new_sounds_to_play_vec != sounds_to_play_vec {
        sounds_to_play.set(new_sounds_to_play_vec);
    }

    // Process players messages
    let process_messages = {
        move |messages: Vec<PlayerMessage>,
              sounds_to_play: &UseState<Vec<SoundsQueue>>,
              item_reveal_message: &UseState<ItemRevealMessage>| {
            let mut new_sounds = Vec::new();
            for message in messages {
                let sound = match message {
                    PlayerMessage::ItemAdded => "item_added",
                    PlayerMessage::QuestionAsked => "question_added",
                    PlayerMessage::GameStart => "game_start",
                    PlayerMessage::CoinGiven => "coin_added",
                    PlayerMessage::ItemGuessed(player_name, item_id, item_name) => {
                        item_reveal_message.set(ItemRevealMessage {
                            show: true,
                            expiry: get_current_time() + 5.0,
                            correct: true,
                            str: format!("{player_name} guessed item {item_id} correctly as {item_name}!"),
                        });
                        "guess_correct"
                    }
                    PlayerMessage::GuessIncorrect => "guess_incorrect",
                    PlayerMessage::ItemRemoved(item_id, item_name) => {
                        item_reveal_message.set(ItemRevealMessage {
                            show: true,
                            expiry: get_current_time() + 5.0,
                            correct: false,
                            str: format!("Item {item_id} was removed from the game, it was {item_name}!"),
                        });
                        "guess_incorrect"
                    }
                };
                new_sounds.push(SoundsQueue {
                    expiry: get_current_time() + 5.0,
                    sound: String::from(sound),
                });
            }
            if !new_sounds.is_empty() {
                let mut old_sounds = sounds_to_play.get().clone();
                old_sounds.extend(new_sounds);
                sounds_to_play.set(old_sounds);
            }
        }
    };

    // Get lobby state every x seconds if connected or lobby info if not connected
    let cancel_signal = use_state(cx, || Arc::new(Mutex::new(false)));
    use_effect(cx, is_connected, |is_connected| {
        let cancel_signal = cancel_signal.clone();

        // Set the cancellation signal for the previous loop
        {
            let mut cancel = cancel_signal.get().lock().unwrap();
            *cancel = true;
        }
        let new_cancel_signal = Arc::new(Mutex::new(false));
        cancel_signal.set(new_cancel_signal.clone());

        let lobby_state = lobby_state.clone();
        let lobby_info = lobby_info.clone();
        let lobby_id = lobby_id.clone();
        let player_name = player_name.clone();
        let error_message = error_message.clone();
        let sounds_to_play = sounds_to_play.clone();
        let item_reveal_message = item_reveal_message.clone();
        async move {
            loop {
                if *new_cancel_signal.lock().unwrap() {
                    break;
                }

                if *is_connected.get() {
                    match get_state(lobby_id.get().to_string(), player_name.get().to_string()).await {
                        Ok((lobby, messages)) => {
                            process_messages(messages.clone(), &sounds_to_play, &item_reveal_message);
                            lobby_state.set(Some(lobby));
                        }
                        Err(error) => {
                            error_message.set(ErrorDialog {
                                show: true,
                                str: format!("Disconnected from lobby: {error}"),
                            });
                            is_connected.set(false);
                            break;
                        }
                    }
                } else if let Ok(lobbys) = get_lobby_info().await {
                    lobby_info.set(lobbys);
                }
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        }
    });

    let disconnect = Box::new(move || {
        let player_name = player_name.clone();
        let lobby_id = lobby_id.clone();
        is_connected.set(false);
        lobby_state.set(None);

        cx.spawn(async move {
            let _result = disconnect_player(lobby_id.get().to_string(), player_name.get().to_string()).await;
        });
    });

    let start = Box::new(move || {
        let player_name = player_name.clone();
        let lobby_id = lobby_id.clone();

        cx.spawn(async move {
            let _result = start_lobby(lobby_id.get().to_string(), player_name.get().to_string()).await;
        });
    });

    let render_error_dialog = rsx! {
        div { class: "dialog floating error", top: if error_message.get().show { "50%" } else { "-100%" },
            "{error_message.get().str}"
            button {
                onclick: move |_| {
                    error_message
                        .set(ErrorDialog {
                            show: false,
                            str: error_message.get().str.clone(),
                        });
                },
                "OK"
            }
        }
    };

    if *is_connected.get() {
        if let Some(lobby) = lobby_state.get() {
            let sounds_str = sounds_to_play
                .iter()
                .map(|sound| format!("{};{}", sound.expiry.round(), sound.sound))
                .collect::<Vec<String>>()
                .join(",");
            cx.render(rsx! {
                game_view(cx, player_name, lobby_id, lobby, disconnect, start),
                render_error_dialog,
                item_reveal_message.render(),
                div { id: "sounds", visibility: "collapse", position: "absolute", "{sounds_str}" }
            })
        } else {
            cx.render(rsx! { div { "Loading..." } })
        }
    } else {
        cx.render(rsx! {
            div {
                display: "flex",
                flex_direction: "column",
                align_items: "center",
                gap: "10px",
                justify_content: "center",
                height: "calc(100vh - 40px)",
                div { class: "background-box", display: "flex", flex_direction: "column", gap: "5px",
                    for lobby in lobby_info.get() {
                        div { display: "flex", flex_direction: "row", align_items: "center", gap: "5px",
                            div { "{lobby.id}: {lobby.players_count} Players" }
                            button {
                                onclick: move |_| {
                                    lobby_id.set(lobby.id.clone());
                                    let player_name = player_name.clone();
                                    let lobby_id = lobby.id.clone();
                                    let is_connected = is_connected.clone();
                                    let error_message = error_message.clone();
                                    lobby_state.set(None);
                                    cx.spawn(async move {
                                        if let Err(error)
                                            = connect_player(lobby_id.clone(), player_name.get().to_string()).await
                                        {
                                            error_message
                                                .set(ErrorDialog {
                                                    show: true,
                                                    str: format!("Failed to connect to lobby: {error}"),
                                                });
                                        } else {
                                            is_connected.set(true);
                                        }
                                    });
                                },
                                "Connect"
                            }
                        }
                    }
                }
                form {
                    class: "dialog",
                    onsubmit: move |_| {
                        let player_name = player_name.clone();
                        let lobby_id = lobby_id.clone();
                        let is_connected = is_connected.clone();
                        let error_message = error_message.clone();
                        lobby_state.set(None);
                        cx.spawn(async move {
                            if let Err(error)
                                = connect_player(lobby_id.get().to_string(), player_name.get().to_string())
                                    .await
                            {
                                error_message
                                    .set(ErrorDialog {
                                        show: true,
                                        str: format!("Failed to connect to lobby: {error}"),
                                    });
                            } else {
                                is_connected.set(true);
                            }
                        });
                    },
                    input {
                        r#type: "text",
                        placeholder: "Player Name",
                        pattern: PLAYER_NAME_PATTERN,
                        maxlength: MAX_PLAYER_NAME_LENGTH as i64,
                        oninput: move |e| {
                            player_name.set(e.value.clone());
                        }
                    }
                    input {
                        r#type: "text",
                        placeholder: "Lobby Id",
                        pattern: LOBBY_ID_PATTERN,
                        maxlength: MAX_LOBBY_ID_LENGTH as i64,
                        oninput: move |e| {
                            lobby_id.set(e.value.clone());
                        }
                    }
                    button { r#type: "submit", "Connect" }
                }
            }
            render_error_dialog
        })
    }
}
