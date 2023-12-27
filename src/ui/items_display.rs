use crate::lobby_utils::Lobby;
use dioxus::prelude::*;

#[derive(Debug, PartialEq)]
struct TempQuestion {
    id: usize,
    question: String,
    masked: bool,
}

pub fn render<'a>(cx: Scope<'a>, player_name: &str, lobby: &Lobby) -> Element<'a> {
    // Get list of questions that are active
    let mut active_questions = vec![];
    for item in &lobby.items {
        for question in &item.questions {
            let question_string = if question.masked {
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
                    format!("MASKED - {question_player_name}")
                }
            } else {
                question.question.clone()
            };

            let structed = TempQuestion {
                id: question.id,
                question: question_string,
                masked: question.masked,
            };
            if !active_questions.contains(&structed) {
                active_questions.push(structed);
            }
        }
    }

    let is_quizmaster = player_name == lobby.key_player && lobby.settings.player_controlled;

    cx.render(rsx! {
        div { class: "table-row",
            div { class: "table-header-box", flex: "1", "Question" }
            lobby.items.iter().map(|item| {
                let (content, width) = if is_quizmaster { (item.name.clone(), "unset") } else { (item.id.to_string(), "20px") };
                rsx! {
                    div { class: "table-header-box", width: width, flex: "unset", text_align: "center", content }
                }
            })
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
                        justify_content: "start",
                        div { font_weight: "bold", width: "20px", "{question_index + 1}" },
                        div { "{question_string}" }
                    }
                    lobby.items.iter().map(|item| {
                        let answer = if question_index < 20 - num_blanks {
                            item.questions.iter()
                                .find(|answer_question| answer_question.id == question_id).map(|answer_question| answer_question.answer.clone())
                        } else {
                            None
                        };
                        let box_fill = if answer.is_none() { "⭐" } else { "" };
                        let answer_color = answer.map_or("rgb(60, 60, 80)".to_string(), |answer| answer.to_color().to_string());
                        rsx! {
                            div { class: "table-body-box", width: "20px", text_align: "center", background_color: "{answer_color}", box_fill }
                        }
                    })
                }
            }
        })
    })
}
