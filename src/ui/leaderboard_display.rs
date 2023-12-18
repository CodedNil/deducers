use crate::{
    connection::kick_player,
    lobby_utils::{Lobby, Player},
};
use dioxus::prelude::*;

pub fn render<'a>(cx: Scope<'a>, player_name: &String, lobby_id: &'a String, lobby: &Lobby) -> Element<'a> {
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

    let kick_player = {
        move |row_player: String| {
            let lobby_id = lobby_id.to_string();

            cx.spawn(async move {
                let _result = kick_player(lobby_id, row_player).await;
            });
        }
    };

    cx.render(rsx! {
        div { class: "table-row",
            rsx! {
                div { class: "table-header-box", flex: "2", "Player" }
                div { class: "table-header-box", flex: "1", "Score" }
            }
        }
        sorted_players.iter().map(|player| {
            let row_class = if player.score == sorted_players[0].score {
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
                    if can_kick {
                        rsx! {
                            div {
                                class: row_class,
                                flex: "1",
                                gap: "5px",
                                "{player.score}",
                                button {
                                    onclick: move |_| {
                                        kick_player(row_player.clone());
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
