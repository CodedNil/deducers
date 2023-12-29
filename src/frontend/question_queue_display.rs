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
    let questions_queue = questions_queue.clone();
    let mut sorted_questions = questions_queue;
    sorted_questions.sort_by(|a, b| {
        if a.votes == b.votes {
            a.question.cmp(&b.question)
        } else {
            b.votes.cmp(&a.votes)
        }
    });

    cx.render(rsx! {
        div { align_self: "center",
            if *questions_queue_active {
                format!("Top Question Submitted in {questions_queue_countdown} Seconds")
            } else {
                format!("Top Question Submitted After {} Votes", settings.question_min_votes)
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
            let (player_name, lobby_id) = (player_name.clone(), lobby_id.clone());
            rsx! {
                div { class: "table-row",
                    div { class: "{row_class}", flex: "1", "{question.player}" }
                    div { class: "{row_class}", flex: "3", "{question_text}" }
                    div {
                        class: "{row_class}",
                        flex: "1",
                        "{question.votes}",
                        if !is_quizmaster {
                            rsx! { button {
                                onclick: move |_| {
                                    let _result = vote_question(&lobby_id, &player_name, &question_string);
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
