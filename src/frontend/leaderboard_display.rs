use crate::backend::{kick_player, PlayerReduced};
use dioxus::prelude::*;

#[derive(Props, PartialEq, Eq)]
pub struct Props {
    pub player_name: String,
    pub lobby_id: String,
    pub players: Vec<PlayerReduced>,
    pub is_keyplayer: bool,
}

#[allow(non_snake_case)]
pub fn Leaderboard(cx: Scope<Props>) -> Element {
    let (player_name, lobby_id) = (cx.props.player_name.clone(), cx.props.lobby_id.clone());
    let mut sorted_players = cx.props.players.clone();
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
            let can_kick = cx.props.is_keyplayer && player.name != *player_name;
            let lobby_id = lobby_id.clone();
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
                                        let _result = kick_player(&lobby_id, &row_player);
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
