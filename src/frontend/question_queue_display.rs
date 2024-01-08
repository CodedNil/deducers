use crate::backend::{question_queue::vote_question, LobbySettings, QueuedQuestion};
use dioxus::prelude::*;

#[component]
pub fn QuestionQueueDisplay(
    cx: Scope,
    player_name: String,
    lobby_id: String,
    is_quizmaster: bool,
    questions_queue: Vec<QueuedQuestion>,
    questions_queue_active: bool,
    questions_queue_countdown: usize,
    settings: LobbySettings,
) -> Element {
    cx.render(rsx! {
        div { align_self: "center",
            if *questions_queue_active {
                format!("Top Question Submitted in {questions_queue_countdown} Seconds")
            } else {
                format!("Top Question Submitted After {} Votes", settings.question_min_votes)
            }
        }
        div { class: "table-row",
            div { class: "header-box", flex: "1", "Player" }
            div { class: "header-box", flex: "3", "Question" }
            div { class: "header-box", flex: "1", "Votes" }
        }
        questions_queue.iter().map(|question| {
            let row_class = format!("body-box{}", if question.player == *player_name { " self" } else { "" });
            let question_text = if question.masked {
                if question.player != *player_name && !is_quizmaster {
                    "MASKED".to_owned()
                } else {
                    format!("MASKED - {}", question.question)
                }
            } else {
                question.question.clone()
            };
            cx.render(rsx! {
                div { class: "table-row",
                    div { class: "{row_class}", flex: "1", "{question.player}" }
                    div { class: "{row_class}", flex: "3", "{question_text}" }
                    div { class: "{row_class}", flex: "1",
                        "{question.votes}"
                        if !is_quizmaster {
                            rsx! { button {
                                onclick: move |_| {
                                    vote_question(lobby_id, player_name, &question.question);
                                },
                                padding: "2px",
                                "🪙"
                            }}
                        }
                    }
                }
            })
        })
    })
}
