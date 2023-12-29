use super::connection::AlertPopup;
use crate::{
    lobby_utils::{add_chat_message, disconnect_player, start_lobby, Lobby},
    ui::{items_display, leaderboard_display, management_display, question_queue_display},
    MAX_CHAT_LENGTH,
};
use dioxus::prelude::*;

#[allow(clippy::cast_possible_wrap, clippy::too_many_arguments)]
pub fn render<'a>(
    cx: Scope<'a>,
    player_name: &'a str,
    lobby_id: &'a str,
    lobby: &Lobby,
    is_connected: &'a UseState<bool>,
    lobby_settings_open: &'a UseState<bool>,
    alert_popup: &'a UseState<AlertPopup>,
) -> Element<'a> {
    let chat_submission: &UseState<String> = use_state(cx, String::new);

    cx.render(rsx! {
        div { display: "flex", height: "calc(100vh - 40px)", gap: "20px",
            div {
                // Items
                class: "background-box",
                flex: "1.5",
                overflow_y: "auto",
                items_display::render(cx, player_name, lobby)
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
                                    lobby_settings_open.set(false);
                                    let _result = start_lobby(lobby_id, player_name);
                                }, "Start" } }
                            }
                            if lobby.key_player == *player_name && !lobby.started {
                                rsx! { button { onclick: move |_| {
                                    lobby_settings_open.set(!lobby_settings_open.get());
                                }, "Settings" } }
                            }
                            button { onclick: move |_| {
                                is_connected.set(false);
                                let _result = disconnect_player(lobby_id, player_name);
                            }, "Disconnect" }
                        }
                    }

                    div {
                        // Leaderboard
                        class: "background-box",
                        leaderboard_display::render(cx, player_name, lobby_id, lobby)
                    }
                }
                div { display: "flex", flex_direction: "column", gap: "5px",
                    div {
                        // Management
                        class: "background-box",
                        management_display::render(cx, player_name, lobby_id, lobby, &alert_popup)
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
                    // Item Queue
                    class: "background-box",
                    min_height: "200px",
                    overflow_y: "auto",
                    question_queue_display::render(cx, player_name, lobby_id, lobby)
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
                        onsubmit: move |_| {
                            let _result = add_chat_message(lobby_id, player_name, chat_submission);
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
