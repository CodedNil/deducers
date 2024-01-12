use crate::backend::Item;
use dioxus::prelude::*;
use std::collections::HashSet;
use strum::EnumProperty;

#[component]
pub fn ItemDisplay(cx: Scope, player_name: String, is_quizmaster: bool, items: Vec<Item>) -> Element {
    let mut questions_found = HashSet::new();
    let mut active_questions: Vec<_> = items
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
            let answers = items
                .iter()
                .map(|item| item.questions.iter().find(|&q| q.id == question.id).map(|q| q.answer))
                .collect();
            (question_text, font_style, answers)
        })
        .collect();
    active_questions.resize_with(20, || (String::new(), "normal", vec![None; items.len()]));

    cx.render(rsx! {
        div { class: "table-row",
            div { class: "header-box", flex: "1", "Question" }
            for item in items {
                div { class: "header-box", width: if *is_quizmaster { "unset" } else { "20px" }, flex: "unset", text_align: "center",
                    if *is_quizmaster { format!("{}: {}", item.id, item.name) } else { item.id.to_string() }
                }
            }
        }
        for (question_string , font_style , answers) in active_questions {
            div { class: "table-row", flex: "1",
                div { class: "body-box", flex: "1", justify_content: "start", div { font_style: font_style, "{question_string}" } }
                for answer in answers {
                    div { class: "body-box", width: "20px", text_align: "center", background_color: answer.map_or("rgb(60, 60, 80)", |answer| answer.get_str("color").unwrap_or_default()),
                        if answer.is_none() && question_string.is_empty() { "⭐" } else { "" }
                    }
                }
            }
        }
    })
}
