use crate::{
    backend::items::quizmaster_change_answer,
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
            let question_string = question.question.clone();
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
                        button { background_color: "rgb(20, 100, 20)", "Submit" }
                    }
                    div {
                        display: "flex",
                        gap: "5px",
                        question.items.iter().map(|item| {
                            let question_string1 = question_string.clone();
                            let question_string2 = question_string.clone();
                            let item1 = item.clone();
                            rsx! {
                                button {
                                    class: "table-body-box {item.answer.to_string().to_lowercase()}",
                                    flex: "1",
                                    display: "flex",
                                    flex_direction: "column",
                                    gap: "5px",
                                    onclick: move |_| {
                                        let lobby_id = lobby_id.to_string();
                                        let player_name = player_name.to_string();
                                        let question = question_string1.clone();
                                        let new_answer = item1.answer.next();
                                        cx.spawn(async move {
                                            let _response = quizmaster_change_answer(lobby_id, player_name, question, item1.id, new_answer).await;
                                        });
                                    },
                                    div {
                                        "{item.name}: {item.answer.to_string()}"
                                    },
                                    div {
                                        display: "flex",
                                        width: "100%",
                                        Answer::variants().iter().map(|answer| {
                                            let question_string2 = question_string2.clone();
                                            let answer = answer.clone();
                                            rsx! {
                                                button {
                                                    class: "table-body-box {answer.to_string().to_lowercase()} smallanswerbutton",
                                                    onclick: move |_| {
                                                        let lobby_id = lobby_id.to_string();
                                                        let player_name = player_name.to_string();
                                                        let question = question_string2.clone();
                                                        let new_answer = answer.clone();
                                                        cx.spawn(async move {
                                                            let _response = quizmaster_change_answer(lobby_id, player_name, question, item1.id, new_answer).await;
                                                        });
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
