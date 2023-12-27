use crate::{
    lobby_utils::{connect_player, disconnect_player, get_current_time, get_lobby_info, get_state, Lobby, PlayerMessage},
    ui::{gamesettings, gameview},
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
    victory: bool,
    correct: bool,
    str: String,
}

impl ItemRevealMessage {
    fn render(&self) -> LazyNodes<'_, '_> {
        let item_reveal_correct_class = if self.victory {
            "victory"
        } else if self.correct {
            "correct"
        } else {
            "incorrect"
        };
        rsx! {
            div {
                class: "dialog floating item-reveal {item_reveal_correct_class}",
                top: if self.show { "20%" } else { "-100%" },
                "{self.str}"
            }
        }
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

#[derive(Default)]
pub struct AlertPopup {
    pub shown: bool,
    expiry: f64,
    pub message: String,
}

impl AlertPopup {
    pub fn message(message: String) -> Self {
        Self {
            shown: true,
            expiry: get_current_time() + 5.0,
            message,
        }
    }
}

pub fn tutorial(tutorial_open: &UseState<bool>) -> LazyNodes<'_, '_> {
    rsx! {
        div { class: "dialog floating tutorial", top: if *tutorial_open.get() { "50%" } else { "-100%" },
            p {
                "Welcome to the intriguing world of Deducers! Here's how you can become a master deducer in this multiplayer twist on 20 Questions:"
            }
            p {
                strong { "The Game Board:" }
                " At the start, two items will be in play, listed under columns '1' and '2'. The names of these items are a mystery, represented by simple words like 'Bird', 'Mountain', or 'Phone'."
            }
            p {
                strong { "Collect Coins:" }
                " You'll earn coins passively as time goes by. Keep an eye on your coin balance!"
            }
            p {
                strong { "Submit Questions:" }
                " Use your coins to ask questions that will help you deduce the items. Think strategically! For a higher coin cost submit questions masked, other players won't see your question, only the answer."
            }
            p {
                strong { "Question Queue:" }
                " Your submitted questions enter a queue. Every 10 seconds, the question with the most votes is asked. Vote wisely to uncover the clues you need."
            }
            p {
                strong { "Revealing Answers:" }
                " As questions are asked, each item will reveal its answers as 'Yes', 'No', 'Maybe', or 'Unknown'. These clues are vital to your deduction process."
            }
            p {
                strong { "Make Your Guess:" }
                " If you think you've cracked it, spend coins to guess the item. The sooner you guess an item correctly, the more points you get."
            }
            p {
                strong { "New Items:" }
                " After every 5th question, a new item appears, keeping the game fresh and exciting. Keep track of all items and use your questions to reveal their secrets."
            }
            p { "Happy deducing, and may the most astute player win!" }
            button {
                onclick: move |_| {
                    tutorial_open.set(false);
                },
                "OK"
            }
        }
    }
}

#[allow(clippy::too_many_lines, clippy::cast_possible_wrap)]
pub fn app(cx: Scope) -> Element {
    let player_name = use_state(cx, String::new);
    let lobby_id = use_state(cx, String::new);
    let is_connected = use_state(cx, || false);

    let lobby_state = use_state(cx, || None::<Lobby>);
    let lobby_info = use_state(cx, Vec::new);
    let lobby_settings_open = use_state(cx, || true);

    let error_message = use_state(cx, ErrorDialog::default);

    let tutorial_open = use_state(cx, || false);

    // Hide the item reveal message after 5 seconds
    let item_reveal_message: &UseState<ItemRevealMessage> = use_state(cx, ItemRevealMessage::default);
    if item_reveal_message.get().show && item_reveal_message.get().expiry < get_current_time() {
        item_reveal_message.set(ItemRevealMessage {
            show: false,
            expiry: 0.0,
            victory: item_reveal_message.get().victory,
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

    let alert_popup: &UseState<AlertPopup> = use_state(cx, AlertPopup::default);
    if alert_popup.get().shown && alert_popup.get().expiry < get_current_time() {
        alert_popup.set(AlertPopup::default());
    }

    // Process players messages
    let process_messages = {
        move |messages: Vec<PlayerMessage>,
              sounds_to_play: &UseState<Vec<SoundsQueue>>,
              item_reveal_message: &UseState<ItemRevealMessage>,
              alert_popup: &UseState<AlertPopup>| {
            let mut new_sounds = Vec::new();
            for message in messages {
                let sound = match message {
                    PlayerMessage::ItemAdded => "item_added",
                    PlayerMessage::QuestionAsked => "question_added",
                    PlayerMessage::GameStart => "game_start",
                    PlayerMessage::CoinGiven => "coin_added",
                    PlayerMessage::ItemGuessed(player_name, item_id, item_name) => {
                        if !(item_reveal_message.get().show && item_reveal_message.get().victory) {
                            item_reveal_message.set(ItemRevealMessage {
                                show: true,
                                expiry: get_current_time() + 5.0,
                                victory: false,
                                correct: true,
                                str: format!("{player_name} guessed item {item_id} correctly as {item_name}!"),
                            });
                        }
                        "guess_correct"
                    }
                    PlayerMessage::GuessIncorrect => "guess_incorrect",
                    PlayerMessage::ItemRemoved(item_id, item_name) => {
                        if !(item_reveal_message.get().show && item_reveal_message.get().victory) {
                            item_reveal_message.set(ItemRevealMessage {
                                show: true,
                                expiry: get_current_time() + 5.0,
                                victory: false,
                                correct: false,
                                str: format!("Item {item_id} was removed from the game, it was {item_name}!"),
                            });
                        }
                        "guess_incorrect"
                    }
                    PlayerMessage::Winner(players) => {
                        let win_message = if players.len() > 1 {
                            format!("The tied winners are {}!", players.join(", "))
                        } else if players.is_empty() {
                            String::from("The game has ended with no winner!")
                        } else {
                            format!("The winner is {}!", players[0])
                        };
                        item_reveal_message.set(ItemRevealMessage {
                            show: true,
                            expiry: get_current_time() + 30.0,
                            victory: true,
                            correct: true,
                            str: win_message,
                        });
                        "guess_correct"
                    }
                    PlayerMessage::QuestionRejected(message) => {
                        alert_popup.set(AlertPopup::message(format!("Question '{message}' rejected by quizmaster")));
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
        let alert_popup = alert_popup.clone();
        async move {
            loop {
                if *new_cancel_signal.lock().unwrap() {
                    break;
                }

                if *is_connected.get() {
                    match get_state(&lobby_id.get().clone(), &player_name.get().clone()) {
                        Ok((lobby, messages)) => {
                            process_messages(messages.clone(), &sounds_to_play, &item_reveal_message, &alert_popup);
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
                } else {
                    lobby_info.set(get_lobby_info());
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
        let _result = disconnect_player(&lobby_id.get().clone(), &player_name.get().clone());
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
                gameview::render(cx, player_name, lobby_id, lobby, disconnect, lobby_settings_open, alert_popup),
                render_error_dialog,
                item_reveal_message.render(),
                rsx! { gamesettings::render(cx, lobby_settings_open, player_name, lobby_id, lobby) },
                div { id: "sounds", visibility: "collapse", position: "absolute", "{sounds_str}" }
            })
        } else {
            cx.render(rsx! { div { "Loading" } })
        }
    } else {
        let is_lobby_valid = lobby_info.get().iter().any(|lobby| lobby.id == *lobby_id.get());
        cx.render(rsx! {
            div {
                display: "flex",
                flex_direction: "column",
                align_items: "center",
                gap: "10px",
                height: "calc(100vh - 40px)",
                img { src: "/assets/deducers_banner2.png", width: "400px", padding: "20px" }
                div { class: "background-box",
                    for lobby in lobby_info.get() {
                        div { display: "flex", flex_direction: "row", align_items: "center", gap: "5px",
                            div { "{lobby.id}: {lobby.players_count} Players" }
                            button {
                                onclick: move |_| {
                                    lobby_id.set(lobby.id.clone());
                                    lobby_state.set(None);
                                    if let Err(error) = connect_player(lobby_id, player_name) {
                                        error_message
                                            .set(ErrorDialog {
                                                show: true,
                                                str: format!("Failed to connect to lobby: {error}"),
                                            });
                                    } else {
                                        is_connected.set(true);
                                    }
                                },
                                "Join"
                            }
                        }
                    }
                }
                button {
                    onclick: move |_| {
                        tutorial_open.set(true);
                    },
                    "Learn How To Play"
                }
                form {
                    class: "dialog",
                    onsubmit: move |_| {
                        lobby_state.set(None);
                        if let Err(error) = connect_player(lobby_id, player_name) {
                            error_message
                                .set(ErrorDialog {
                                    show: true,
                                    str: format!("Failed to connect to lobby: {error}"),
                                });
                        } else {
                            is_connected.set(true);
                        }
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
                    button { r#type: "submit",
                        if is_lobby_valid { "Join" } else { "Create Lobby" }
                    }
                }
            }
            render_error_dialog,
            tutorial(tutorial_open)
        })
    }
}
