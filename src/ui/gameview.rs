use crate::{
    get_current_time,
    ui::{items_display, leaderboard_display, management_display, question_queue_display},
    Lobby,
};
use dioxus::prelude::*;

#[allow(clippy::too_many_lines)]
pub fn game_view<'a>(
    cx: Scope<'a>,
    player_name: &'a String,
    lobby_id: &'a String,
    lobby: &Lobby,
    disconnect: Box<dyn Fn() + 'a>,
    start: Box<dyn Fn() + 'a>,
) -> Element<'a> {
    let time = lobby.elapsed_time.round();
    let alert_popup: &UseState<Option<(f64, String)>> = use_state(cx, || None::<(f64, String)>);

    // Clear alert if time has passed
    if let Some(alert_popup_contents) = alert_popup.get() {
        if alert_popup_contents.0 < get_current_time() {
            alert_popup.set(None);
        }
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
                            span { font_weight: "normal", "{time}s" }
                        }
                        div { display: "flex", gap: "5px",
                            if lobby.key_player == *player_name && !lobby.started {
                                rsx! { button { onclick: move |_| (start)(), "Start" } }
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
                        leaderboard_display::render(cx, player_name, lobby)
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
                    if alert_popup.get().is_some() {
                        let message = alert_popup.get().clone().unwrap_or_default().1;
                        rsx! {
                            div {
                                class: "background-box alert",
                                display: "flex",
                                flex_direction: "column",
                                gap: "5px",
                                "{message}"
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
                    question_queue_display::render(cx, player_name, lobby)
                }
            }
        }
    })
}
