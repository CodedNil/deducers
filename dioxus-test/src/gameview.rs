use dioxus::prelude::*;

use crate::Lobby;

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
            let ping = 0.0;
            cx.render(rsx! {
                div { display: "flex", height: "calc(100vh - 40px)", gap: "1rem",
                    div { class: "background-box", flex: "1.5" }
                    div { flex: "1", display: "flex", flex_direction: "column", gap: "1rem",
                        div { flex: "1", display: "flex", flex_direction: "column", gap: "0.5rem",

                            div {
                                // Lobby info
                                class: "background-box",
                                display: "flex",
                                gap: "1rem",
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
                                p { font_weight: "bold",
                                    "Ping "
                                    span { font_weight: "normal", "{ping}ms" }
                                }
                                button { onclick: move |_| (disconnect)(), "Disconnect" }
                            }

                            div {
                                // Leaderboard
                                class: "background-box",
                                flex_grow: "1",
                                display: "flex",
                                flex_direction: "column",
                                gap: "1rem",
                                justify_content: "space-between"
                            }
                        }
                        div { class: "background-box", flex: "1.5" }
                        div { class: "background-box", flex: "1" }
                    }
                }
            })
        },
    )
}
