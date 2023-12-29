use crate::{backend::question_queue::vote_question, lobby_utils::Lobby};
use dioxus::prelude::*;

pub fn render<'a>(cx: Scope<'a>, player_name: &'a str, lobby_id: &'a str, lobby: &Lobby) -> Element<'a> {
    let questions = lobby.questions_queue.clone();
    let mut sorted_questions = questions;
    sorted_questions.sort_by(|a, b| {
        if a.votes == b.votes {
            a.question.cmp(&b.question)
        } else {
            b.votes.cmp(&a.votes)
        }
    });

    cx.render(rsx! {
        div { align_self: "center",
            if lobby.question_queue_active() {
                format!("Top Question Submitted in {} Seconds", lobby.questions_queue_countdown.round())
            } else {
                format!("Top Question Submitted After {} Votes", lobby.settings.question_min_votes)
            }
        }

        div { class: "table-row",
            div { class: "table-header-box", flex: "1", "Player" }
            div { class: "table-header-box", flex: "3", "Question" }
            div { class: "table-header-box", flex: "1", "Votes" }
        }
        sorted_questions.iter().map(|question| {
            let row_class = format!("table-body-box{}", if question.player == *player_name { " self" } else { "" });
            let question_string = question.question.clone();
            let question_text = if question.masked && question.player != *player_name {
                "MASKED".to_owned()
            } else {
                question.question.clone()
            };
            rsx! {
                div { class: "table-row",
                    div { class: "{row_class}", flex: "1", "{question.player}" }
                    div { class: "{row_class}", flex: "3", "{question_text}" }
                    div {
                        class: "{row_class}",
                        flex: "1",
                        "{question.votes}",
                        if !(player_name == lobby.key_player && lobby.settings.player_controlled) {
                            rsx! { button {
                                onclick: move |_| {
                                    let _result = vote_question(lobby_id, player_name, &question_string);
                                },
                                padding: "2px",
                                "🪙"
                            }}
                        }
                    }
                }
            }
        })
    })
}
