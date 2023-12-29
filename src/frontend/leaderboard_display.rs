use crate::backend::{kick_player, PlayerReduced};
use dioxus::prelude::*;

#[component]
pub fn Leaderboard(cx: Scope, player_name: String, lobby_id: String, players: Vec<PlayerReduced>, is_keyplayer: bool) -> Element {
    let mut sorted_players = players.clone();
    sorted_players.sort_by(|a, b| {
        if a.score == b.score {
            a.name.cmp(&b.name)
        } else {
            b.score.cmp(&a.score)
        }
    });

    cx.render(rsx! {
        div { class: "table-row",
            rsx! {
                div { class: "table-header-box", flex: "2", "Player" }
                div { class: "table-header-box", flex: "1", "Score" }
            }
        }
        sorted_players.iter().map(|player| {
            let row_color = if player.quizmaster {
                "rgb(120, 110, 20)"
            } else if player.score == sorted_players[0].score {
                "rgb(80, 80, 60)"
            } else if player.name == *player_name {
                "rgb(60, 80, 80)"
            } else {
                "rgb(60, 60, 80)"
            };
            let row_player = player.name.clone();
            let can_kick = *is_keyplayer && player.name != *player_name;
            let lobby_id = lobby_id.clone();
            rsx! {
                div { class: "table-row",
                    div { class: "table-body-box", background_color: row_color, flex: "2", "{row_player}" }
                    if player.quizmaster {
                        rsx! { div { class: "table-body-box", background_color: row_color, flex: "1", "👑" } }
                    } else if can_kick {
                        rsx! {
                            div {
                                class: "table-body-box",
                                background_color: row_color,
                                flex: "1",
                                gap: "5px",
                                "{player.score}",
                                button {
                                    onclick: move |_| {
                                        kick_player(&lobby_id, &row_player);
                                    },
                                    padding: "2px",
                                    "💥"
                                }
                            }
                        }
                    } else {
                        rsx! { div { class: "table-body-box", background_color: row_color, flex: "1", "{player.score}" } }
                    }
                }
            }
        })
    })
}
