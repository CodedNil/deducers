use crate::{
    backend::{add_chat_message, disconnect_player, start_lobby, ChatMessage, Lobby, LobbySettings, Player, PlayerReduced, QueuedQuestion},
    frontend::{
        items_display::ItemDisplay, leaderboard_display::Leaderboard, management_display, question_queue_display::QuestionQueueDisplay,
        AlertPopup,
    },
    MAX_CHAT_LENGTH,
};
use dioxus::prelude::*;

#[derive(Props, PartialEq, Eq)]
pub struct Props {
    pub player_name: String,
    pub lobby_id: String,
    pub is_quizmaster: bool,
    pub key_player: String,
    pub started: bool,

    pub settings: LobbySettings,
    pub questions_queue: Vec<QueuedQuestion>,
    pub questions_queue_active: bool,
    pub questions_queue_countdown: usize,
    pub players: Vec<PlayerReduced>,
    pub items: Vec<String>,
    pub chat_messages: Vec<ChatMessage>,
}

#[allow(clippy::cast_possible_wrap, clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub fn render<'a>(
    cx: Scope<'a>,
    player_name: &'a str,
    lobby_id: &'a str,
    lobby: &Lobby,
    is_connected: &'a UseState<bool>,
    alert_popup: &'a UseState<AlertPopup>,
) -> Element<'a> {
    let is_keyplayer = player_name == lobby.key_player;
    let is_quizmaster = player_name == lobby.key_player && lobby.settings.player_controlled;

    cx.render(rsx! {
        div { display: "flex", height: "calc(100vh - 40px)", gap: "20px",
            div {
                // Items
                class: "background-box",
                flex: "1.5",
                overflow_y: "auto",
                ItemDisplay {
                    player_name: player_name.to_owned(),
                    is_quizmaster: is_quizmaster,
                    items: lobby.items.clone(),
                }
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

                        p { font_weight: "bold",
                            "Lobby "
                            span { font_weight: "normal", "{lobby_id}" }
                        }
                        p { font_weight: "bold",
                            "Time "
                            span { font_weight: "normal", "{lobby.elapsed_time.round()}s" }
                        }
                        div { display: "flex", gap: "5px",
                            if lobby.key_player == *player_name && !lobby.started {
                                rsx! { button { onclick: move |_| {
                                    let _result = start_lobby(lobby_id, player_name);
                                }, "Start" } }
                            }
                            button {
                                onclick: move |_| {
                                    is_connected.set(false);
                                    let _result = disconnect_player(lobby_id, player_name);
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
                            players: lobby.players.values().map(Player::reduce).collect(),
                            is_keyplayer: is_keyplayer
                        }
                    }
                }
                div { display: "flex", flex_direction: "column", gap: "5px",
                    div {
                        // Management
                        class: "background-box",
                        if lobby.started {
                            management_display::render(cx, player_name, lobby_id, lobby, alert_popup)
                        } else {
                            cx.render(rsx! { div { align_self: "center", font_size: "larger", "Waiting for game to start" } })
                        }
                    }
                    if alert_popup.get().shown {
                        rsx! {
                            div {
                                class: "background-box alert",
                                "{alert_popup.get().message}"
                            }
                        }
                    }
                }
                div {
                    // Questions Queue
                    class: "background-box",
                    min_height: "200px",
                    overflow_y: "auto",
                    QuestionQueueDisplay {
                        player_name: player_name.to_owned(),
                        lobby_id: lobby_id.to_owned(),
                        questions_queue: lobby.questions_queue.clone(),
                        questions_queue_active: lobby.questions_queue_active(),
                        questions_queue_countdown: lobby.questions_queue_countdown.round() as usize,
                        settings: lobby.settings,
                        is_quizmaster: is_quizmaster
                    }
                }
                div {
                    // Chat
                    class: "background-box",
                    flex: "1",
                    min_height: "200px",
                    overflow_y: "auto",
                    div { class: "table-header-box", "Chat" }
                    div { flex: "1", display: "flex", flex_direction: "column", gap: "3px", overflow_y: "auto",
                        lobby.chat_messages.iter().rev().map(|message| {
                            rsx! {
                                div { class: "table-body-box", "{message.player}: {message.message}" }
                            }
                        })
                    }
                    form {
                        display: "flex",
                        gap: "5px",
                        onsubmit: move |form_data| {
                            if let Some(messages) = form_data.values.get("message") {
                                if let Some(message) = messages.first() {
                                    let _result = add_chat_message(lobby_id, player_name, message);
                                }
                            }
                        },
                        input {
                            placeholder: "Message",
                            name: "message",
                            maxlength: MAX_CHAT_LENGTH as i64,
                            flex: "1",
                            "data-clear-on-submit": "true",
                        }
                        button { r#type: "submit", "Send" }
                    }
                }
            }
        }
    })
}
