use crate::{
    ui::{items, leaderboard, management, question_queue},
    Lobby,
};
use dioxus::prelude::*;

#[allow(clippy::too_many_lines)]
pub fn game_view<'a>(
    cx: Scope<'a>,
    disconnect: Box<dyn Fn() + 'a>,
    player_name: &String,
    lobby_id: &String,
    lobby: &Lobby,
) -> Element<'a> {
    let time = lobby.elapsed_time.round();
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
                items::render(cx, player_name, lobby)
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
                                rsx!(button { "Start" })
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
                        leaderboard::render(cx, player_name, lobby)
                    }
                }
                div {
                    // Management
                    class: "background-box",
                    flex: "1",
                    display: "flex",
                    flex_direction: "column",
                    gap: "5px",
                    management::render(cx, player_name, lobby)
                }
                div {
                    // Item Queue
                    class: "background-box",
                    flex: "1",
                    display: "flex",
                    flex_direction: "column",
                    gap: "5px",
                    question_queue::render(cx, player_name, lobby)
                }
            }
        }
    })
}
