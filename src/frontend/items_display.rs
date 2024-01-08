use crate::backend::Item;
use dioxus::prelude::*;
use std::collections::HashSet;
use strum::EnumProperty;

#[component]
pub fn ItemDisplay(cx: Scope, player_name: String, is_quizmaster: bool, items: Vec<Item>) -> Element {
    let mut questions_found = HashSet::new();
    let active_questions: Vec<_> = items
        .iter()
        .flat_map(|item| &item.questions)
        .filter(|question| questions_found.insert(question.id))
        .map(|question| {
            let question_text = if question.masked {
                if &question.player != player_name && !is_quizmaster {
                    format!("MASKED - {}", question.player)
                } else {
                    format!("MASKED {} - {}", question.text, question.player)
                }
            } else {
                question.text.clone()
            };
            let font_style = if question.masked { "italic" } else { "normal" };
            (question.id, question_text, font_style)
        })
        .collect();

    cx.render(rsx! {
        div { class: "table-row",
            div { class: "header-box", flex: "1", "Question" }
            items.iter().map(|item| {
                let (content, width) = if *is_quizmaster { (format!("{}: {}", item.id, item.name), "unset") } else { (item.id.to_string(), "20px") };
                rsx! {
                    div { class: "header-box", width: width, flex: "unset", text_align: "center", content }
                }
            })
        }
        (0..20usize).map(|question_index| {
            let is_blank = question_index >= active_questions.len();
            let (question_id, question_string, font_style) = if is_blank {
                (question_index + 1, String::new(), "normal")
            } else {
                active_questions[question_index].clone()
            };
            rsx! {
                div { class: "table-row", flex: "1",
                    div {
                        class: "body-box",
                        flex: "1",
                        justify_content: "start",
                        div { font_style: font_style, question_string }
                    }
                    items.iter().map(|item| {
                        let answer = if is_blank { None } else { item.questions.iter().find(|&q| q.id == question_id).map(|q| q.answer) };
                        let box_fill = if answer.is_none() && is_blank { "⭐" } else { "" };
                        let answer_color = answer.map_or("rgb(60, 60, 80)", |answer| answer.get_str("color").unwrap_or_default());
                        rsx! {
                            div { class: "body-box", width: "20px", text_align: "center", background_color: answer_color, box_fill }
                        }
                    })
                }
            }
        })
    })
}
