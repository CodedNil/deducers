use crate::backend::Item;
use dioxus::prelude::*;
use strum::EnumProperty;

#[derive(PartialEq)]
struct TempQuestion {
    id: usize,
    question: String,
    masked: bool,
}

#[component]
pub fn ItemDisplay(cx: Scope, player_name: String, is_quizmaster: bool, items: Vec<Item>) -> Element {
    let mut active_questions = vec![];
    for item in items {
        for question in &item.questions {
            let question_string = if question.masked {
                let question_player_name = items
                    .iter()
                    .flat_map(|item| &item.questions)
                    .find(|item_question| item_question.id == question.id)
                    .map(|item_question| item_question.player.clone())
                    .unwrap_or_default();

                if &question_player_name == player_name {
                    question.text.clone()
                } else {
                    format!("MASKED - {question_player_name}")
                }
            } else {
                question.text.clone()
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

    cx.render(rsx! {
        div { class: "table-row",
            div { class: "header-box", flex: "1", "Question" }
            items.iter().map(|item| {
                let (content, width) = if *is_quizmaster { (item.name.clone(), "unset") } else { (item.id.to_string(), "20px") };
                rsx! {
                    div { class: "header-box", width: width, flex: "unset", text_align: "center", content }
                }
            })
        }
        (0..20usize).map(|question_index| {
            let is_blank = question_index >= active_questions.len();
            let (question_id, question_string) = if is_blank {
                (question_index + 1, String::new())
            } else {
                let question = &active_questions[question_index];
                (question.id, question.question.clone())
            };
            rsx! {
                div { class: "table-row", flex: "1",
                    div {
                        class: "body-box",
                        flex: "1",
                        justify_content: "start",
                        div { font_weight: "bold", width: "20px", "{question_index + 1}" },
                        div { question_string }
                    }
                    items.iter().map(|item| {
                        let answer = if is_blank { None } else { item.questions.iter().find(|&q| q.id == question_id).map(|q| q.answer) };
                        let box_fill = if answer.is_none() { "⭐" } else { "" };
                        let answer_color = answer.map_or("rgb(60, 60, 80)", |answer| answer.get_str("color").unwrap());
                        rsx! {
                            div { class: "body-box", width: "20px", text_align: "center", background_color: answer_color, box_fill }
                        }
                    })
                }
            }
        })
    })
}
