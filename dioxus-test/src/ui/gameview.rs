use dioxus::prelude::*;

use crate::{ui::leaderboard::leaderboard, Lobby};

pub fn game_view<'a>(
    cx: Scope<'a>,
    disconnect: Box<dyn Fn() + 'a>,
    player_name: &UseState<String>,
    lobby_id: &UseState<String>,
    lobby_state: &UseState<Option<Lobby>>,
) -> Element<'a> {
    (*lobby_state.get()).as_ref().map_or_else(
        || {
            cx.render(rsx! {div {
            }
            })
        },
        |lobby| {
            let time = lobby.elapsed_time.round();
            cx.render(rsx! {
                div { display: "flex", height: "calc(100vh - 40px)", gap: "20px",
                    div { class: "background-box", flex: "1.5" }
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
                                    "Lobby Id "
                                    span { font_weight: "normal", "{lobby_id}" }
                                }
                                p { font_weight: "bold",
                                    "Time "
                                    span { font_weight: "normal", "{time}s" }
                                }
                                button { onclick: move |_| (disconnect)(), "Disconnect" }
                            }

                            div {
                                // Leaderboard
                                class: "background-box",
                                display: "flex",
                                flex_direction: "column",
                                gap: "5px",
                                max_height: "300px",
                                overflow_y: "scroll",

                                leaderboard(cx, player_name, lobby_state)
                            }
                        }
                        div {
                            // Management
                            class: "background-box",
                            flex: "1.5",
                            display: "flex",
                            flex_direction: "column",
                            gap: "5px",
                            div { "5🪙 Available" }
                            div {
                                input { value: "{player_name}", placeholder: "Question To Ask" }
                                button { "Submit Question X🪙" }
                            }
                        }
                        div { class: "background-box", flex: "1" }
                    }
                }
            })
        },
    )
}
