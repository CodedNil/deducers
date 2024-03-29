use crate::{
    backend::{connect_player, get_current_time, get_lobby_info, get_state, Lobby, LobbyState, Player, PlayerMessage},
    frontend::{gamesettings::GameSettings, gameview::GameView},
    CLIENT_UPDATE_INTERVAL, LOBBY_ID_PATTERN, MAX_LOBBY_ID_LENGTH, MAX_PLAYER_NAME_LENGTH, PLAYER_NAME_PATTERN,
};
use dioxus::prelude::*;
use std::time::Duration;
use strum_macros::Display;
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

#[derive(Clone, Copy, PartialEq, Default, Display)]
enum RevealType {
    Victory,
    Correct,
    #[default]
    Incorrect,
}

impl RevealType {
    pub const fn to_color(self) -> &'static str {
        match self {
            Self::Victory => "rgb(120, 110, 20)",
            Self::Correct => "rgb(60, 130, 50)",
            Self::Incorrect => "rgb(140, 80, 0)",
        }
    }
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
    pub expiry: f64,
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

    let error_message = use_state(cx, ErrorDialog::default);

    let tutorial_open = use_state(cx, || false);

    let item_reveal_message = use_ref(cx, ItemRevealMessage::default);
    if item_reveal_message.read().show && item_reveal_message.read().expiry < get_current_time() {
        item_reveal_message.with_mut(|message| {
            message.show = false;
        });
    }

    let sounds_to_play: &UseRef<Vec<SoundsQueue>> = use_ref(cx, Vec::new);
    if sounds_to_play.read().iter().any(|sound| sound.expiry <= get_current_time()) {
        sounds_to_play.with_mut(|sounds| {
            sounds.retain(|sound| sound.expiry > get_current_time());
        });
    }

    let alert_popup = use_state(cx, AlertPopup::default);
    if alert_popup.get().shown && alert_popup.get().expiry < get_current_time() {
        alert_popup.set(AlertPopup::default());
    }

    let last_update = use_state(cx, get_current_time);
    if get_current_time() - last_update.get() > CLIENT_UPDATE_INTERVAL {
        last_update.set(get_current_time());
        if *is_connected.get() {
            if let Ok((lobby, messages)) = get_state(lobby_id, player_name) {
                let mut new_sounds = Vec::new();
                for message in messages {
                    let sound = match message {
                        PlayerMessage::ItemAdded => "item_added;0.5",
                        PlayerMessage::QuestionAsked => "question_added;0.5",
                        PlayerMessage::GameStart => "game_start;0.3",
                        PlayerMessage::CoinGiven => "coin_added;0.5",
                        PlayerMessage::ItemGuessed(player_name, item_id, item_name) => {
                            if !(item_reveal_message.read().show && item_reveal_message.read().revealtype == RevealType::Victory) {
                                item_reveal_message.set(ItemRevealMessage::new(
                                    5.0,
                                    format!("{player_name} guessed item {item_id} correctly as {item_name}!"),
                                    RevealType::Correct,
                                ));
                            }
                            "guess_correct;0.5"
                        }
                        PlayerMessage::GuessIncorrect => "guess_incorrect;0.5",
                        PlayerMessage::ItemRemoved(item_id, item_name) => {
                            if !(item_reveal_message.read().show && item_reveal_message.read().revealtype == RevealType::Victory) {
                                item_reveal_message.set(ItemRevealMessage::new(
                                    5.0,
                                    format!("Item {item_id} was removed from the game, it was {item_name}!"),
                                    RevealType::Incorrect,
                                ));
                            }
                            "guess_incorrect;0.5"
                        }
                        PlayerMessage::Winner(win_message) => {
                            item_reveal_message.set(ItemRevealMessage::new(30.0, win_message.clone(), RevealType::Victory));
                            "guess_correct;0.5"
                        }
                        PlayerMessage::QuestionRejected(message) => {
                            alert_popup.set(AlertPopup::message(format!("Question '{message}' rejected by quizmaster")));
                            "guess_incorrect;0.5"
                        }
                        PlayerMessage::AlertPopup(message) => {
                            alert_popup.set(AlertPopup::message(message.clone()));
                            ""
                        }
                        PlayerMessage::PlayerKicked => {
                            error_message.set(ErrorDialog {
                                show: true,
                                str: "You were kicked from the lobby".to_string(),
                            });
                            ""
                        }
                    };
                    if !sound.is_empty() {
                        new_sounds.push(SoundsQueue {
                            expiry: get_current_time() + 5.0,
                            sound: String::from(sound),
                        });
                    }
                }
                if !new_sounds.is_empty() {
                    sounds_to_play.with_mut(|sounds| {
                        sounds.extend(new_sounds);
                    });
                }
                lobby_state.set(Some(lobby));
            } else {
                lobby_state.set(None);
                is_connected.set(false);
            }
        } else {
            lobby_info.set(get_lobby_info());
        }
    }
    // Force a refresh every CLIENT_UPDATE_INTERVAL
    let count = use_state(cx, || 0);
    use_effect(cx, (), |()| {
        let count = count.clone();
        async move {
            loop {
                count.modify(|c| c + 1);
                sleep(Duration::from_secs_f64(CLIENT_UPDATE_INTERVAL)).await;
            }
        }
    });

    let render_error_dialog = rsx! {
        div { class: "dialog {error_message.get().show}", background_color: "rgb(100, 20, 20)",
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
        lobby_state.get().as_ref().map_or_else(
            || cx.render(rsx! { div {} }),
            |lobby| {
                let sounds_str = sounds_to_play
                    .read()
                    .iter()
                    .map(|sound| format!("{};{}", sound.expiry.round(), sound.sound))
                    .collect::<Vec<_>>()
                    .join(",");
                let reveal_message = item_reveal_message.read().clone();
                cx.render(rsx! {
                    GameView {
                        player_name: player_name.get().clone(),
                        lobby_id: lobby_id.get().clone(),
                        key_player: lobby.key_player.clone(),
                        started: matches!(lobby.state, LobbyState::Play | LobbyState::Starting),
                        elapsed_time: lobby.elapsed_time.round() as usize,
                        settings: lobby.settings.clone(),
                        questions_queue: lobby.questions_queue.clone(),
                        questions_queue_active: lobby.questions_queue_active(),
                        questions_queue_countdown: lobby.questions_queue_countdown.round() as usize,
                        quizmaster_queue: lobby.quizmaster_queue.clone(),
                        players: lobby.players.values().map(Player::reduce).collect(),
                        items: lobby.items.clone(),
                        questions: lobby.questions.clone(),
                        chat_messages: lobby.chat_messages.clone(),
                        alert_popup_message: alert_popup.get().message.clone()
                    }
                    render_error_dialog,
                    div { class: "dialog {reveal_message.show}", background_color: reveal_message.revealtype.to_color(), "{reveal_message.str}" }
                    if player_name == &lobby.key_player && !matches!(lobby.state, LobbyState::Play | LobbyState::Starting) {
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
                input {
                    r#type: "text",
                    placeholder: "Player Name",
                    pattern: PLAYER_NAME_PATTERN,
                    maxlength: MAX_PLAYER_NAME_LENGTH as i64,
                    oninput: move |e| {
                        player_name.set(e.value.trim().to_owned());
                    }
                }
                form {
                    class: "background-box",
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
                div { class: "background-box",
                    for lobby in lobby_info.get().iter().filter(|lobby| !lobby.started) {
                        div { display: "flex", flex_direction: "row", align_items: "center", gap: "5px",
                            div { "{lobby.id}: {lobby.players_count} Players" }
                            button {
                                onclick: move |_| {
                                    lobby_id.set(lobby.id.clone());
                                    lobby_state.set(None);
                                    if let Err(error) = connect_player(&lobby.id, player_name) {
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
            }
            render_error_dialog,
            tutorial(tutorial_open)
        })
    }
}
