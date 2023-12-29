use crate::backend::{kick_player, Lobby, Player};
use dioxus::prelude::*;

pub fn render<'a>(cx: Scope<'a>, player_name: &'a str, lobby_id: &'a str, lobby: &Lobby) -> Element<'a> {
    let players = lobby.players.values().collect::<Vec<&Player>>();
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
            let row_class = if player.quizmaster {
                "table-body-box quizmaster"
            } else if player.score == sorted_players[0].score {
                "table-body-box winner"
            } else if player.name == *player_name {
                "table-body-box self"
            } else {
                "table-body-box"
            };
            let row_player = player.name.clone();
            let can_kick = lobby.key_player == *player_name && player.name != *player_name;
            rsx! {
                div { class: "table-row",
                    div { class: row_class, flex: "2", "{row_player}" }
                    if player.quizmaster {
                        rsx! { div { class: row_class, flex: "1", "👑" } }
                    } else if can_kick {
                        rsx! {
                            div {
                                class: row_class,
                                flex: "1",
                                gap: "5px",
                                "{player.score}",
                                button {
                                    onclick: move |_| {
                                        let _result = kick_player(lobby_id, &row_player);
                                    },
                                    padding: "2px",
                                    "💥"
                                }
                            }
                        }
                    } else {
                        rsx! { div { class: row_class, flex: "1", "{player.score}" } }
                    }
                }
            }
        })
    })
}
