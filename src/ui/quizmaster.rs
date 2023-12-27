use crate::{
    backend::items::{quizmaster_change_answer, quizmaster_reject, quizmaster_submit},
    lobby_utils::{Answer, Lobby},
};
use dioxus::prelude::*;

pub fn render<'a>(cx: Scope<'a>, player_name: &'a str, lobby_id: &'a str, lobby: &Lobby) -> Element<'a> {
    let questions = lobby.quizmaster_queue.clone();

    cx.render(rsx! {
        div { class: "table-row",
            rsx! {
                div { class: "table-header-box", flex: "1", "Player" }
                div { class: "table-header-box", flex: "3", "Question" }
            }
        }
        questions.iter().map(|question| {
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
                                let _response = quizmaster_submit(lobby_id, player_name, &question_string1);
                            },
                            background_color: "rgb(20, 100, 20)", "Submit" }
                        button {
                            onclick: move |_| {
                                let _response = quizmaster_reject(lobby_id, player_name, &question_string2);
                            },
                            background_color: "rgb(100, 20, 20)", "Reject" }
                    }
                    div {
                        display: "flex",
                        gap: "5px",
                        question.items.iter().map(|item| {
                            let item = item.clone();
                            let question_string = question_string1.clone();
                            rsx! {
                                button {
                                    class: "table-body-box {item.answer.to_string().to_lowercase()}",
                                    flex: "1",
                                    display: "flex",
                                    flex_direction: "column",
                                    gap: "5px",
                                    onclick: move |_| {
                                        let _response = quizmaster_change_answer(lobby_id, player_name, &question_string, item.id, item.answer.next());
                                    },
                                    div {
                                        "{item.name}: {item.answer.to_string()}"
                                    },
                                    div {
                                        display: "flex",
                                        width: "100%",
                                        Answer::variants().iter().map(|answer| {
                                            let question_string = question_string.clone();
                                            let answer = answer.clone();
                                            rsx! {
                                                button {
                                                    class: "table-body-box {answer.to_string().to_lowercase()} smallanswerbutton",
                                                    onclick: move |_| {
                                                        let _response = quizmaster_change_answer(lobby_id, player_name, &question_string, item.id, answer.clone());
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
