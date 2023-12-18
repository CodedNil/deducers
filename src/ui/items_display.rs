use crate::Lobby;
use dioxus::prelude::*;

#[derive(PartialEq)]
struct TempQuestion {
    id: usize,
    question: String,
    anonymous: bool,
}

pub fn render<'a>(cx: Scope<'a>, player_name: &String, lobby: &Lobby) -> Element<'a> {
    // Get list of questions that are active
    let mut active_questions = vec![];
    for item in &lobby.items {
        for question in &item.questions {
            let question_string = if question.anonymous {
                let question_player_name = lobby
                    .items
                    .iter()
                    .flat_map(|item| &item.questions)
                    .find(|item_question| item_question.id == question.id)
                    .map(|item_question| item_question.player.clone())
                    .unwrap_or_default();

                if question_player_name == *player_name {
                    question.question.clone()
                } else {
                    format!("ANONYMOUS - {player_name}")
                }
            } else {
                question.question.clone()
            };

            let structed = TempQuestion {
                id: question.id,
                question: question_string,
                anonymous: question.anonymous,
            };
            if !active_questions.contains(&structed) {
                active_questions.push(structed);
            }
        }
    }

    cx.render(rsx! {
        div { class: "table-row",
            rsx! {
                div { class: "table-header-box", flex: "1", "Question" }
                lobby.items.iter().map(|item| {
                    rsx! {
                        div { class: "table-header-box", width: "20px", flex: "unset", text_align: "center", "{item.id}" }
                    }
                })
            }
        }
        (0..20usize).map(|question_index| {
            let num_blanks = 20 - active_questions.len();
            let (question_id, question_string) = if question_index < 20 - num_blanks {
                let question = active_questions.get(question_index).unwrap();
                (question.id, question.question.clone())
            } else {
                (question_index + 1, String::new())
            };
            rsx! {
                div { class: "table-row", flex: "1",
                    div {
                        class: "table-body-box",
                        flex: "1",
                        display: "flex",
                        justify_content: "start",
                        gap: "5px",
                        div { font_weight: "bold", width: "20px", "{question_index + 1}" },
                        div { "{question_string}" }
                    }
                    lobby.items.iter().map(|item| {
                        let answer_type = if question_index < 20 - num_blanks {
                            item.questions.iter()
                                .find(|answer_question| answer_question.id == question_id)
                                .map_or(String::new(), |answer_question| format!("{:?}", answer_question.answer).to_lowercase())
                        } else {
                            String::new()
                        };
                        let class_name = format!("table-body-box {answer_type}").trim().to_string();
                        let box_fill = if answer_type.is_empty() { "⭐" } else { "" };
                        rsx! {
                            div { class: "{class_name}", width: "20px", flex: "unset", text_align: "center", "{box_fill}" }
                        }
                    })
                }
            }
        })
    })
}
