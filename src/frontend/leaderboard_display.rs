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
            div { class: "header-box", flex: "2", "Player" }
            div { class: "header-box", flex: "1", "Score" }
        }
        sorted_players.iter().map(|player| {
            let row_color = match player {
                _ if player.quizmaster => "rgb(120, 110, 20)",
                _ if player.score == sorted_players[0].score => "rgb(80, 80, 60)",
                _ if player.name == *player_name => "rgb(60, 80, 80)",
                _ => "rgb(60, 60, 80)",
            };
            let (row_player, row_score) = (player.name.clone(), player.score.to_string());
            rsx! {
                div { class: "table-row",
                    div { class: "body-box", background_color: row_color, flex: "2", "{row_player}" }
                    div {
                        class: "body-box",
                        background_color: row_color,
                        flex: "1",
                        gap: "5px",
                        if player.quizmaster { "👑" } else { &row_score },
                        if *is_keyplayer && row_player != *player_name {
                            rsx! { button {
                                onclick: move |_| {
                                    kick_player(lobby_id, player_name, &row_player);
                                },
                                padding: "2px",
                                "💥"
                            }}
                        }
                    }
                }
            }
        })
    })
}
