use crate::backend::{
    items::{quizmaster_change_answer, quizmaster_reject, quizmaster_submit},
    Answer, QueuedQuestionQuizmaster,
};
use dioxus::prelude::*;
use strum::{EnumProperty, IntoEnumIterator};

#[component]
pub fn QuizmasterDisplay(cx: Scope, player_name: String, lobby_id: String, quizmaster_queue: Vec<QueuedQuestionQuizmaster>) -> Element {
    cx.render(rsx! {
        div { class: "table-row",
            div { class: "header-box", flex: "1", "Player" }
            div { class: "header-box", flex: "3", "Question" }
        }
        for question in quizmaster_queue.iter() {
            div { class: "header-box", display: "flex", flex_direction: "column", gap: "5px", text_transform: "none",
                div { display: "flex", gap: "5px",
                    div { class: "body-box", "{question.player}" }
                    div { class: "body-box", flex: "1", "{question.question}" }
                    button {
                        onclick: move |_| {
                            quizmaster_submit(lobby_id, player_name, &question.question);
                        },
                        background_color: "rgb(20, 100, 20)",
                        "Submit"
                    }
                    button {
                        onclick: move |_| {
                            quizmaster_reject(lobby_id, player_name, &question.question);
                        },
                        background_color: "rgb(100, 20, 20)",
                        "Reject"
                    }
                }
                div { display: "flex", gap: "5px",
                    for item in question.items.iter() {
                        div {
                            class: "body-box",
                            flex: "1",
                            display: "flex",
                            flex_direction: "column",
                            gap: "5px",
                            background_color: item.answer.get_str("color").unwrap_or_default(),
                            div { "{item.name}: {item.answer.to_string()}" }
                            div { display: "flex", width: "100%",
                                for answer in Answer::iter() {
                                    button {
                                        class: "body-box",
                                        padding: "8px",
                                        flex: "1",
                                        border: "1px solid white",
                                        background_color: answer.get_str("color").unwrap_or_default(),
                                        onclick: move |_| {
                                            quizmaster_change_answer(lobby_id, player_name, &question.question, item.id, answer);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    })
}
