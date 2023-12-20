use crate::{
    lobby_utils::{get_current_time, Lobby},
    ui::{items_display, leaderboard_display, management_display, question_queue_display},
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
                    flex: "1",
                    display: "flex",
                    flex_direction: "column",
                    gap: "5px",
                    question_queue_display::render(cx, player_name, lobby_id, lobby)
                }
            }
        }
    })
}
