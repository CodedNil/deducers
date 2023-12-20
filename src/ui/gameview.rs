use crate::{
    lobby_utils::{add_chat_message, get_current_time, Lobby},
    ui::{items_display, leaderboard_display, management_display, question_queue_display},
    MAX_CHAT_LENGTH,
};
use anyhow::Error;
use dioxus::prelude::*;

#[derive(Default)]
pub struct AlertPopup {
    shown: bool,
    expiry: f64,
    message: String,
}

impl AlertPopup {
    pub fn error(error: &Error) -> Self {
        Self {
            shown: true,
            expiry: get_current_time() + 5.0,
            message: error.to_string(),
        }
    }
}

#[allow(clippy::too_many_lines, clippy::cast_possible_wrap)]
pub fn render<'a>(
    cx: Scope<'a>,
    player_name: &'a String,
    lobby_id: &'a String,
    lobby: &Lobby,
    disconnect: Box<dyn Fn() + 'a>,
    start: Box<dyn Fn() + 'a>,
    lobby_settings_open: &'a UseState<bool>,
) -> Element<'a> {
    let alert_popup: &UseState<AlertPopup> = use_state(cx, AlertPopup::default);
    if alert_popup.get().shown && alert_popup.get().expiry < get_current_time() {
        alert_popup.set(AlertPopup::default());
    }
    let chat_submission: &UseState<String> = use_state(cx, String::new);

    cx.render(rsx! {
        div { display: "flex", height: "calc(100vh - 40px)", gap: "20px",
            div {
                // Items
                class: "background-box",
                flex: "1.5",
                display: "flex",
                flex_direction: "column",
                gap: "5px",
                overflow_y: "auto",
                items_display::render(cx, player_name, lobby)
            }
            div { flex: "1", display: "flex", flex_direction: "column", gap: "20px",
                div { display: "flex", flex_direction: "column", gap: "5px",

                    div {
                        // Lobby info
                        class: "background-box",
                        display: "flex",
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
                                    lobby_settings_open.set(false);
                                    start();
                                }, "Start" } }
                            }
                            if lobby.key_player == *player_name && !lobby.started {
                                rsx! { button { onclick: move |_| {
                                    lobby_settings_open.set(!lobby_settings_open.get());
                                }, "Settings" } }
                            }
                            button { onclick: move |_| (disconnect)(), "Disconnect" }
                        }
                    }

                    div {
                        // Leaderboard
                        class: "background-box",
                        display: "flex",
                        flex_direction: "column",
                        gap: "5px",
                        leaderboard_display::render(cx, player_name, lobby_id, lobby)
                    }
                }
                div { display: "flex", flex_direction: "column", gap: "5px",
                    div {
                        // Management
                        class: "background-box",
                        display: "flex",
                        flex_direction: "column",
                        gap: "5px",
                        management_display::render(cx, player_name, lobby_id, lobby, &alert_popup)
                    }
                    if alert_popup.get().shown {
                        rsx! {
                            div {
                                class: "background-box alert",
                                display: "flex",
                                flex_direction: "column",
                                gap: "5px",
                                "{alert_popup.get().message}"
                            }
                        }
                    }
                }
                div {
                    // Item Queue
                    class: "background-box",
                    display: "flex",
                    flex_direction: "column",
                    gap: "5px",
                    question_queue_display::render(cx, player_name, lobby_id, lobby)
                }
                div {
                    // Chat
                    class: "background-box",
                    flex: "1",
                    display: "flex",
                    flex_direction: "column",
                    gap: "5px",
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
                        onsubmit: move |_| {
                            let lobby_id = lobby_id.to_string();
                            let player_name = player_name.to_string();
                            let submission = chat_submission.get().clone();
                            cx.spawn(async move {
                                let _result = add_chat_message(lobby_id, player_name, submission).await;
                            });
                        },
                        input {
                            placeholder: "Message",
                            maxlength: MAX_CHAT_LENGTH as i64,
                            flex: "1",
                            "data-clear-on-submit": "true",
                            oninput: move |e| {
                                chat_submission.set(e.value.clone());
                            }
                        }
                        button { r#type: "submit", "Send" }
                    }
                }
            }
        }
    })
}
