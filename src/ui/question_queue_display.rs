use crate::{
    backend::question_queue::player_vote_question, Lobby, QueuedQuestion,
    SUBMIT_QUESTION_EVERY_X_SECONDS,
};
use dioxus::prelude::*;

pub fn render<'a>(
    cx: Scope<'a>,
    player_name: &'a String,
    lobby_id: &'a String,
    lobby: &Lobby,
) -> Element<'a> {
    let questions = lobby
        .questions_queue
        .iter()
        .collect::<Vec<&QueuedQuestion>>();
    // Sorting the questions by score and name
    let mut sorted_questions = questions.clone();
    sorted_questions.sort_by(|a, b| {
        if a.votes == b.votes {
            a.question.cmp(&b.question)
        } else {
            b.votes.cmp(&a.votes)
        }
    });

    let next_question_remaining_time = (SUBMIT_QUESTION_EVERY_X_SECONDS
        - (lobby.elapsed_time % SUBMIT_QUESTION_EVERY_X_SECONDS))
        .round();

    let vote_question = {
        move |question| {
            let lobby_id = lobby_id.to_string();
            let player_name = player_name.clone();

            cx.spawn(async move {
                let _result = player_vote_question(lobby_id, player_name, question).await;
            });
        }
    };

    cx.render(rsx! {
        div { align_self: "center", "Top Question Submitted in {next_question_remaining_time} Seconds" }

        div { class: "table-row",
            rsx! {
                div { class: "table-header-box", flex: "1", "Player" }
                div { class: "table-header-box", flex: "3", "Question" }
                div { class: "table-header-box", flex: "1", "Votes" }
            }
        }
        sorted_questions.iter().map(|question| {
            let row_class = if question.player == *player_name {
                "table-body-box self"
            } else {
                "table-body-box"
            };
            let question_string = question.question.clone();
            rsx! {
                div { class: "table-row",
                    div { class: row_class, flex: "1", "{question.player}" }
                    div { class: row_class, flex: "3", "{question.question}" }
                    div {
                        class: row_class,
                        flex: "1",
                        gap: "5px",
                        "{question.votes}",
                        button {
                            onclick: move |_| {
                                vote_question(question_string.clone());
                            },
                            padding: "2px",
                            "🪙"
                        }
                    }
                }
            }
        })
    })
}
