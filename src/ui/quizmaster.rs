use crate::lobby_utils::Lobby;
use dioxus::prelude::*;

pub fn render<'a>(cx: Scope<'a>, player_name: &'a String, lobby_id: &'a str, lobby: &Lobby) -> Element<'a> {
    let questions = lobby.quizmaster_queue.clone();

    cx.render(rsx! {
        div { class: "table-row",
            rsx! {
                div { class: "table-header-box", flex: "1", "Player" }
                div { class: "table-header-box", flex: "3", "Question" }
            }
        }
        questions.iter().map(|question| {
            rsx! {
                div { class: "table-row",
                    div { class: "table-body-box", flex: "1", "{question.player}" }
                    div { class: "table-body-box", flex: "3", "{question.question}" }
                }
            }
        })
    })
}
