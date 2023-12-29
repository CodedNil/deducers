use crate::backend::{
    items::{quizmaster_change_answer, quizmaster_reject, quizmaster_submit},
    Answer, QueuedQuestionQuizmaster,
};
use dioxus::prelude::*;
use strum::{EnumProperty, IntoEnumIterator};

#[component]
pub fn QuizmasterDisplay<'a>(
    cx: Scope,
    player_name: &'a str,
    lobby_id: &'a str,
    quizmaster_queue: Vec<QueuedQuestionQuizmaster>,
) -> Element<'a> {
    cx.render(rsx! {
        div { class: "table-row",
            rsx! {
                div { class: "table-header-box", flex: "1", "Player" }
                div { class: "table-header-box", flex: "3", "Question" }
            }
        }
        quizmaster_queue.iter().map(|question| {
            let question_string1 = question.question.clone();
            let question_string2 = question.question.clone();
            rsx! {
                div {
                    class: "table-header-box",
                    display: "flex",
                    flex_direction: "column",
                    gap: "5px",
                    text_transform: "none",
                    div {
                        display: "flex",
                        gap: "5px",
                        div { class: "table-body-box", "{question.player}" }
                        div { class: "table-body-box", flex: "1", "{question.question}" }
                        button {
                            onclick: move |_| {
                                quizmaster_submit(lobby_id, player_name, &question_string1);
                            },
                            background_color: "rgb(20, 100, 20)", "Submit" }
                        button {
                            onclick: move |_| {
                                quizmaster_reject(lobby_id, player_name, &question_string2);
                            },
                            background_color: "rgb(100, 20, 20)", "Reject" }
                    }
                    div {
                        display: "flex",
                        gap: "5px",
                        question.items.iter().map(|item| {
                            let item = item.clone();
                            let question_string = question_string1.clone();
                            let answer_color = item.answer.get_str("color").unwrap().to_owned();
                            rsx! {
                                div {
                                    class: "table-body-box",
                                    flex: "1",
                                    display: "flex",
                                    flex_direction: "column",
                                    gap: "5px",
                                    background_color: "{answer_color}",
                                    div {
                                        "{item.name}: {item.answer.to_string()}"
                                    },
                                    div {
                                        display: "flex",
                                        width: "100%",
                                        Answer::iter().map(|answer| {
                                            let question_string = question_string.clone();
                                            let answer_color = answer.get_str("color").unwrap().to_owned();
                                            rsx! {
                                                button {
                                                    class: "table-body-box smallanswerbutton",
                                                    background_color: "{answer_color}",
                                                    onclick: move |_| {
                                                        quizmaster_change_answer(lobby_id, player_name, &question_string, item.id, answer);
                                                    },
                                                }
                                            }
                                        })
                                    }
                                }
                            }
                        })
                    }
                }
            }
        })
    })
}
