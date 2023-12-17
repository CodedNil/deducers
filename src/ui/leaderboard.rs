use crate::{Lobby, Player};
use dioxus::prelude::*;

pub fn render<'a>(cx: Scope<'a>, player_name: &String, lobby: &Lobby) -> Element<'a> {
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
            rsx! {
                div { class: "table-row",
                    div { class: row_class, flex: "2", "{player.name}" }
                    div { class: row_class, flex: "1", "{player.score}" }
                }
            }
        })
    })
}
