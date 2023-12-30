use crate::{
    backend::{
        add_chat_message, disconnect_player, start_lobby, ChatMessage, Item, LobbySettings, PlayerReduced, QueuedQuestion,
        QueuedQuestionQuizmaster,
    },
    frontend::{
        items_display::ItemDisplay, leaderboard_display::Leaderboard, management_display::Management,
        question_queue_display::QuestionQueueDisplay, quizmaster::QuizmasterDisplay,
    },
    MAX_CHAT_LENGTH,
};
use dioxus::prelude::*;

#[allow(clippy::cast_possible_wrap)]
#[component]
pub fn GameView(
    cx: Scope,
    player_name: String,
    lobby_id: String,
    key_player: String,
    started: bool,
    elapsed_time: usize,
    settings: LobbySettings,
    questions_queue: Vec<QueuedQuestion>,
    questions_queue_active: bool,
    questions_queue_countdown: usize,
    quizmaster_queue: Vec<QueuedQuestionQuizmaster>,
    players: Vec<PlayerReduced>,
    items: Vec<Item>,
    chat_messages: Vec<ChatMessage>,
    alert_popup_message: String,
) -> Element {
    let is_keyplayer = player_name == key_player;
    let is_quizmaster = is_keyplayer && settings.player_controlled;

    cx.render(rsx! {
        div { display: "flex", height: "calc(100vh - 40px)", gap: "20px",
            div {
                // Items
                class: "background-box",
                flex: "1.5",
                overflow_y: "auto",
                ItemDisplay { player_name: player_name.to_owned(), is_quizmaster: is_quizmaster, items: items.clone() }
            }
            div { flex: "1", display: "flex", flex_direction: "column", gap: "20px",
                div { display: "flex", flex_direction: "column", gap: "5px",
                    div {
                        // Lobby info
                        class: "background-box",
                        flex_direction: "row",
                        gap: "20px",
                        justify_content: "space-between",
                        align_items: "center",

                        div { font_weight: "bold",
                            "Lobby "
                            span { font_weight: "normal", "{lobby_id}" }
                        }
                        div { font_weight: "bold",
                            "Time "
                            span { font_weight: "normal", "{elapsed_time}s" }
                        }
                        div { display: "flex", gap: "5px",
                            if is_keyplayer && !started {
                                rsx! { button { onclick: move |_| {
                                    start_lobby(lobby_id, player_name);
                                }, "Start" } }
                            }
                            button {
                                onclick: move |_| {
                                    disconnect_player(lobby_id, player_name);
                                },
                                "Disconnect"
                            }
                        }
                    }

                    div {
                        // Leaderboard
                        class: "background-box",
                        Leaderboard {
                            player_name: player_name.to_owned(),
                            lobby_id: lobby_id.to_owned(),
                            players: players.clone(),
                            is_keyplayer: is_keyplayer
                        }
                    }
                }
                div { display: "flex", flex_direction: "column", gap: "5px",
                    div {
                        // Management
                        class: "background-box",
                        if *started {
                            if player_name == key_player && settings.player_controlled {
                                rsx! {
                                    QuizmasterDisplay {
                                        player_name: player_name.clone(),
                                        lobby_id: lobby_id.clone(),
                                        quizmaster_queue: quizmaster_queue.clone()
                                    }
                                }
                            } else {
                                rsx! {
                                    Management {
                                        player_name: player_name.to_owned(),
                                        lobby_id: lobby_id.to_owned(),
                                        settings: *settings,
                                        players_coins: players.iter().find(|p| &p.name == player_name).map_or(0, |p| p.coins),
                                        items: items.clone(),
                                    }
                                }
                            }
                        } else {
                            rsx! { div { align_self: "center", font_size: "larger", "Waiting for game to start" } }
                        }
                    }
                    if !alert_popup_message.is_empty() {
                        rsx! {
                            div {
                                class: "background-box",
                                background_color: "rgb(100, 20, 20)",
                                "{alert_popup_message}"
                            }
                        }
                    }
                }
                div {
                    // Questions Queue
                    class: "background-box",
                    min_height: "150px",
                    overflow_y: "auto",
                    QuestionQueueDisplay {
                        player_name: player_name.to_owned(),
                        lobby_id: lobby_id.to_owned(),
                        questions_queue: questions_queue.clone(),
                        questions_queue_active: *questions_queue_active,
                        questions_queue_countdown: *questions_queue_countdown,
                        settings: *settings,
                        is_quizmaster: is_quizmaster
                    }
                }
                div {
                    // Chat
                    class: "background-box",
                    flex: "1",
                    min_height: "150px",
                    overflow_y: "auto",
                    div { class: "header-box", "Chat" }
                    div { flex: "1", display: "flex", flex_direction: "column", gap: "3px", overflow_y: "auto",
                        chat_messages.iter().rev().map(|message| {
                            rsx! {
                                div { class: "body-box", "{message.player}: {message.message}" }
                            }
                        })
                    }
                    form {
                        onsubmit: move |form_data| {
                            if let Some(message) = form_data.values.get("message").and_then(|m| m.first()) {
                                add_chat_message(lobby_id, player_name, message);
                            }
                        },
                        input {
                            placeholder: "Message",
                            name: "message",
                            maxlength: MAX_CHAT_LENGTH as i64,
                            flex: "1",
                            "data-clear-on-submit": "true"
                        }
                        button { r#type: "submit", "Send" }
                    }
                }
            }
        }
    })
}
