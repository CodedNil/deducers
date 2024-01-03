use crate::{
    backend::{connect_player, get_current_time, get_lobby_info, get_state, Lobby, Player, PlayerMessage},
    frontend::{gamesettings::GameSettings, gameview::GameView},
    LOBBY_ID_PATTERN, MAX_LOBBY_ID_LENGTH, MAX_PLAYER_NAME_LENGTH, PLAYER_NAME_PATTERN,
};
use dioxus::prelude::*;
use std::{
    sync::{atomic, Arc},
    time::Duration,
};
use strum::EnumProperty;
use strum_macros::{Display, EnumProperty};
use tokio::time::sleep;

mod gamesettings;
mod gameview;
mod items_display;
mod leaderboard_display;
mod management_display;
mod question_queue_display;
mod quizmaster;

#[derive(Clone, Default)]
struct ItemRevealMessage {
    show: bool,
    expiry: f64,
    str: String,
    revealtype: RevealType,
}

#[derive(Clone, Copy, PartialEq, Default, Display, EnumProperty)]
enum RevealType {
    #[strum(props(color = "rgb(120, 110, 20)"))]
    Victory,
    #[strum(props(color = "rgb(60, 130, 50)"))]
    Correct,
    #[default]
    #[strum(props(color = "rgb(140, 80, 0)"))]
    Incorrect,
}

impl ItemRevealMessage {
    fn new(expiry: f64, str: String, revealtype: RevealType) -> Self {
        Self {
            show: true,
            expiry: get_current_time() + expiry,
            str,
            revealtype,
        }
    }
}

#[derive(Default)]
pub struct ExpiringMsg {
    pub message: String,
    pub shown: bool,
    pub expiry: f64,
}

impl ExpiringMsg {
    fn five(message: &str) -> Self {
        Self {
            message: message.to_owned(),
            shown: true,
            expiry: get_current_time() + 5.0,
        }
    }

    fn inf(message: &str) -> Self {
        Self {
            message: message.to_owned(),
            shown: true,
            expiry: -1.0,
        }
    }
}

pub fn tutorial(tutorial_open: &UseState<bool>) -> LazyNodes<'_, '_> {
    rsx! {
        div { class: "dialog {tutorial_open.get()}", align_items: "normal",
            div {
                "Welcome to the intriguing world of Deducers! Here's how you can become a master deducer in this multiplayer twist on 20 Questions:"
            }
            div {
                strong { "The Game Board:" }
                " At the start, two items will be in play, listed under columns '1' and '2'. The names of these items are a mystery, represented by simple words like 'Bird', 'Mountain', or 'Phone'."
            }
            div {
                strong { "Collect Coins:" }
                " You'll earn coins passively as time goes by. Keep an eye on your coin balance!"
            }
            div {
                strong { "Submit Questions:" }
                " Use your coins to ask questions that will help you deduce the items. Think strategically! For a higher coin cost submit questions masked, other players won't see your question, only the answer."
            }
            div {
                strong { "Question Queue:" }
                " Your submitted questions enter a queue. Every 10 seconds, the question with the most votes is asked. Vote wisely to uncover the clues you need."
            }
            div {
                strong { "Revealing Answers:" }
                " As questions are asked, each item will reveal its answers as 'Yes', 'No', 'Maybe', or 'Unknown'. These clues are vital to your deduction process."
            }
            div {
                strong { "Make Your Guess:" }
                " If you think you've cracked it, spend coins to guess the item. The sooner you guess an item correctly, the more points you get."
            }
            div {
                strong { "New Items:" }
                " After every 5th question, a new item appears, keeping the game fresh and exciting. Keep track of all items and use your questions to reveal their secrets."
            }
            div { "Happy deducing, and may the most astute player win!" }
            button {
                onclick: move |_| {
                    tutorial_open.set(false);
                },
                "Dismiss"
            }
        }
    }
}

pub fn app(cx: Scope) -> Element {
    let player_name = use_state(cx, String::new);
    let lobby_id = use_state(cx, String::new);
    let is_connected = use_state(cx, || false);

    let lobby_state = use_state(cx, || None::<Lobby>);
    let lobby_info = use_state(cx, Vec::new);

    let error_message = use_state(cx, ExpiringMsg::default);

    let tutorial_open = use_state(cx, || false);

    let item_reveal_message = use_ref(cx, ItemRevealMessage::default);
    if item_reveal_message.read().show && item_reveal_message.read().expiry < get_current_time() {
        item_reveal_message.with_mut(|message| {
            message.show = false;
        });
    }

    let sounds_to_play: &UseRef<Vec<ExpiringMsg>> = use_ref(cx, Vec::new);
    if sounds_to_play.read().iter().any(|sound| sound.expiry <= get_current_time()) {
        sounds_to_play.with_mut(|sounds| {
            sounds.retain(|sound| sound.expiry > get_current_time());
        });
    }

    let alert_popup = use_state(cx, ExpiringMsg::default);
    if alert_popup.get().shown && alert_popup.get().expiry < get_current_time() {
        alert_popup.set(ExpiringMsg::default());
    }

    let messages_to_process = use_state(cx, Vec::new);
    if !messages_to_process.get().is_empty() {
        let mut new_sounds = Vec::new();
        for message in messages_to_process.get() {
            match message {
                PlayerMessage::ItemAdded => new_sounds.push(ExpiringMsg::inf("item_added")),
                PlayerMessage::QuestionAsked => new_sounds.push(ExpiringMsg::inf("question_added")),
                PlayerMessage::GameStart => new_sounds.push(ExpiringMsg::inf("game_start")),
                PlayerMessage::CoinGiven => new_sounds.push(ExpiringMsg::inf("coin_added")),
                PlayerMessage::ItemGuessed(player_name, item_id, item_name) => {
                    if !(item_reveal_message.read().show && item_reveal_message.read().revealtype == RevealType::Victory) {
                        item_reveal_message.set(ItemRevealMessage::new(
                            5.0,
                            format!("{player_name} guessed item {item_id} correctly as {item_name}!"),
                            RevealType::Correct,
                        ));
                    }
                    new_sounds.push(ExpiringMsg::inf("guess_correct"));
                }
                PlayerMessage::GuessIncorrect => new_sounds.push(ExpiringMsg::inf("guess_incorrect")),
                PlayerMessage::ItemRemoved(item_id, item_name) => {
                    if !(item_reveal_message.read().show && item_reveal_message.read().revealtype == RevealType::Victory) {
                        item_reveal_message.set(ItemRevealMessage::new(
                            5.0,
                            format!("Item {item_id} was removed from the game, it was {item_name}!"),
                            RevealType::Incorrect,
                        ));
                    }
                    new_sounds.push(ExpiringMsg::inf("guess_incorrect"));
                }
                PlayerMessage::Winner(win_message) => {
                    item_reveal_message.set(ItemRevealMessage::new(30.0, win_message.clone(), RevealType::Victory));
                    new_sounds.push(ExpiringMsg::inf("guess_correct"));
                }
                PlayerMessage::QuestionRejected(message) => {
                    alert_popup.set(ExpiringMsg::five(&format!("Question '{message}' rejected by quizmaster")));
                    new_sounds.push(ExpiringMsg::inf("guess_incorrect"));
                }
                PlayerMessage::AlertPopup(message) => {
                    alert_popup.set(ExpiringMsg::five(message));
                }
                PlayerMessage::PlayerKicked => {
                    error_message.set(ExpiringMsg::inf("You were kicked from the lobby"));
                }
            };
        }
        if !new_sounds.is_empty() {
            sounds_to_play.with_mut(|sounds| {
                sounds.extend(new_sounds);
            });
        }
        messages_to_process.set(Vec::new());
    }

    // Get lobby state every x seconds if connected or lobby info if not connected
    let cancel_signal = use_state(cx, || Arc::new(atomic::AtomicBool::new(false)));
    use_effect(cx, is_connected, |is_connected| {
        // Set the cancellation signal for the previous loop
        let cancel_signal = cancel_signal.clone();
        cancel_signal.store(true, atomic::Ordering::SeqCst);
        let new_cancel_signal = Arc::new(atomic::AtomicBool::new(false));
        cancel_signal.set(Arc::<atomic::AtomicBool>::clone(&new_cancel_signal));

        let (lobby_state, lobby_info) = (lobby_state.clone(), lobby_info.clone());
        let messages_to_process = messages_to_process.clone();
        let (lobby_id, player_name) = (lobby_id.get().clone(), player_name.get().clone());
        async move {
            while !new_cancel_signal.load(atomic::Ordering::SeqCst) {
                if *is_connected.get() {
                    if let Ok((lobby, messages)) = get_state(&lobby_id, &player_name) {
                        messages_to_process.set(messages);
                        lobby_state.set(Some(lobby));
                    } else {
                        is_connected.set(false);
                        break;
                    }
                } else {
                    lobby_info.set(get_lobby_info());
                }
                sleep(Duration::from_millis(500)).await;
            }
        }
    });

    let render_error_dialog = rsx! {
        div { class: "dialog {error_message.get().shown}", background_color: "rgb(100, 20, 20)",
            "{error_message.get().message}"
            button {
                onclick: move |_| {
                    error_message
                        .set(ExpiringMsg {
                            shown: false,
                            expiry: -1.0,
                            message: error_message.get().message.clone(),
                        });
                },
                "OK"
            }
        }
    };

    if *is_connected.get() {
        lobby_state.get().as_ref().map_or_else(
            || cx.render(rsx! { div { "Loading" } }),
            |lobby| {
                let sounds_str = sounds_to_play
                    .read()
                    .iter()
                    .map(|sound| format!("{};{}", sound.expiry.round(), sound.message))
                    .collect::<Vec<_>>()
                    .join(",");
                let reveal_message = item_reveal_message.read().clone();
                cx.render(rsx! {
                    GameView {
                        player_name: player_name.get().clone(),
                        lobby_id: lobby_id.get().clone(),
                        key_player: lobby.key_player.clone(),
                        started: lobby.started || lobby.starting,
                        elapsed_time: lobby.elapsed_time.round() as usize,
                        settings: lobby.settings.clone(),
                        questions_queue: lobby.questions_queue.clone(),
                        questions_queue_active: lobby.questions_queue_active(),
                        questions_queue_countdown: lobby.questions_queue_countdown.round() as usize,
                        quizmaster_queue: lobby.quizmaster_queue.clone(),
                        players: lobby.players.values().map(Player::reduce).collect(),
                        items: lobby.items.clone(),
                        chat_messages: lobby.chat_messages.clone(),
                        alert_popup_message: alert_popup.get().message.clone()
                    }
                    render_error_dialog,
                    div { class: "dialog {reveal_message.show}", background_color: reveal_message.revealtype.get_str("color").unwrap_or_default(), "{reveal_message.str}" }
                    if player_name == &lobby.key_player && !{lobby.started || lobby.starting} {
                        rsx! { GameSettings {
                            player_name: player_name.get().clone(),
                            lobby_id: lobby_id.get().clone(),
                            settings: lobby.settings.clone(),
                            items_queue: lobby.items_queue.clone(),
                        }}
                    }
                    div { id: "sounds", visibility: "collapse", position: "absolute", "{sounds_str}" }
                })
            },
        )
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
                    for lobby in lobby_info.get().iter().filter(|lobby| lobby.started) {
                        div { display: "flex", flex_direction: "row", align_items: "center", gap: "5px",
                            div { "{lobby.id}: {lobby.players_count} Players" }
                            button {
                                onclick: move |_| {
                                    lobby_id.set(lobby.id.clone());
                                    lobby_state.set(None);
                                    if let Err(error) = connect_player(&lobby.id, player_name) {
                                        error_message.set(ExpiringMsg::inf(&format!("Failed to connect to lobby: {error}")));
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
                    class: "background-box",
                    onsubmit: move |_| {
                        lobby_state.set(None);
                        if let Err(error) = connect_player(lobby_id, player_name) {
                            error_message.set(ExpiringMsg::inf(&format!("Failed to connect to lobby: {error}")));
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
                            player_name.set(e.value.trim().to_owned());
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
