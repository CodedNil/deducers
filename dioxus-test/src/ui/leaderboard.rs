use dioxus::prelude::*;

use crate::{Lobby, Player};

pub fn leaderboard<'a>(
    cx: Scope<'a>,
    player_name: &UseState<String>,
    lobby_state: &UseState<Option<Lobby>>,
) -> Element<'a> {
    (*lobby_state.get()).as_ref().map_or_else(
        || cx.render(rsx! { div { "No players available." } }),
        |lobby| {
            let players = lobby.players.values().collect::<Vec<&Player>>();
            // Sorting the players by score and name
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
                        span { class: "table-header-box", flex: "2", "Player" }
                        span { class: "table-header-box", flex: "1", "Score" }
                    }
                }
                sorted_players.iter().map(|player| {
                    let row_class = if player.score == sorted_players[0].score {
                        "table-body-box-winner"
                    } else if player.name == *player_name.get() {
                        "table-body-box-self"
                    } else {
                        "table-body-box"
                    };
                    rsx! {
                        div { class: "table-row",
                            rsx! {
                                span { class: row_class, flex: "2", "{player.name}" }
                                span { class: row_class, flex: "1", "{player.score}" }
                            }
                        }
                    }
                })
            })
        },
    )
}
