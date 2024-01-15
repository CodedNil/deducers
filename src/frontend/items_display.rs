use crate::backend::{Item, Question};
use dioxus::prelude::*;
use std::collections::{HashMap, HashSet};

#[component]
pub fn ItemDisplay(cx: Scope, player_name: String, is_quizmaster: bool, items: Vec<Item>, questions: Vec<Question>) -> Element {
    let questions_by_id: HashMap<usize, &Question> = questions.iter().map(|q| (q.id, q)).collect();
    let mut questions_found = HashSet::new();

    let mut active_questions: Vec<_> = items
        .iter()
        .flat_map(|item| &item.answers)
        .filter_map(|(question_id, _)| questions_by_id.get(question_id))
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
                .map(|item| item.answers.iter().find(|&(id, _)| id == &question.id).map(|(_, a)| a))
                .collect();
            (question.id, question_text, font_style, answers)
        })
        .collect();
    active_questions.sort_by(|(id_a, _, _, _), (id_b, _, _, _)| id_a.cmp(id_b));
    active_questions.resize_with(20, || (0, String::new(), "normal", vec![None; items.len()]));

    cx.render(rsx! {
        div { class: "table-row",
            div { class: "header-box", flex: "1", "Question" }
            for item in items {
                div { class: "header-box", width: if *is_quizmaster { "unset" } else { "20px" }, flex: "unset", text_align: "center",
                    if *is_quizmaster { format!("{}: {}", item.id, item.name) } else { item.id.to_string() }
                }
            }
        }
        for (_ , question_string , font_style , answers) in active_questions {
            div { class: "table-row", flex: "1",
                div { class: "body-box", flex: "1", justify_content: "start", div { font_style: font_style, "{question_string}" } }
                for answer in answers {
                    div { class: "body-box", width: "20px", text_align: "center", background_color: answer.map_or("rgb(60, 60, 80)", |answer| answer.to_color()),
                        if answer.is_none() && question_string.is_empty() { "⭐" } else { "" }
                    }
                }
            }
        }
    })
}
