use crate::{
    backend::{
        alert_popup,
        items::player_guess_item,
        question_queue::{convert_score, submit_question},
        Item, LobbySettings,
    },
    ITEM_NAME_PATTERN, MAX_ITEM_NAME_LENGTH, MAX_QUESTION_LENGTH, QUESTION_PATTERN,
};
use dioxus::prelude::*;

#[component]
pub fn Management(
    cx: Scope,
    player_name: String,
    lobby_id: String,
    settings: LobbySettings,
    players_coins: usize,
    items: Vec<Item>,
) -> Element {
    let question_masked = use_state(cx, || false);
    let submit_cost = settings.submit_question_cost + if *question_masked.get() { settings.masked_question_cost } else { 0 };
    cx.render(rsx! {
        div { align_self: "center", font_size: "larger", "{players_coins}🪙 Available" }
        form {
            onsubmit: move |form_data| {
                if let Some(question) = form_data.values.get("question").and_then(|m| m.first()) {
                    let question = question.to_owned();
                    let (lobby_id, player_name) = (lobby_id.to_owned(), player_name.to_owned());
                    let masked = *question_masked.get();
                    question_masked.set(false);
                    cx.spawn(async move {
                        if let Err(error)
                            = submit_question(&lobby_id, &player_name, question.clone(), masked)
                                .await
                        {
                            alert_popup(
                                &lobby_id,
                                &player_name,
                                &format!("Question rejected {error}"),
                            );
                        }
                    });
                }
            },
            input {
                r#type: "text",
                placeholder: "Question To Ask",
                name: "question",
                flex: "1",
                pattern: QUESTION_PATTERN,
                maxlength: MAX_QUESTION_LENGTH as i64,
                "data-clear-on-submit": "true"
            }
            button { r#type: "submit", "Submit Question {submit_cost}🪙" }
        }
        div { display: "flex", gap: "5px", justify_content: "center",
            input {
                r#type: "checkbox",
                checked: "{question_masked}",
                onclick: move |_| {
                    question_masked.set(!question_masked.get());
                }
            }
            "Masked +{settings.masked_question_cost}🪙"
        }
        div { display: "flex", gap: "5px",
            button {
                onclick: move |_| {
                    convert_score(lobby_id, player_name);
                },
                flex: "1",
                "Convert Leaderboard Score To {settings.score_to_coins_ratio}🪙"
            }
        }
        form {
            onsubmit: move |form_data| {
                let guess = form_data.values.get("guess").and_then(|m| m.first());
                let item_choice = form_data
                    .values
                    .get("key")
                    .and_then(|m| m.first())
                    .and_then(|k| k.parse::<usize>().ok());
                if let (Some(guess), Some(item_choice)) = (guess, item_choice) {
                    player_guess_item(lobby_id, player_name, item_choice, guess);
                }
            },
            input {
                r#type: "text",
                placeholder: "Guess Item",
                name: "guess",
                flex: "1",
                maxlength: MAX_ITEM_NAME_LENGTH as i64,
                pattern: ITEM_NAME_PATTERN,
                "data-clear-on-submit": "true"
            }
            select { name: "key",
                items.iter().map(|item| {
                    rsx! {
                        option { "{item.id}" }
                    }
                })
            }
            button { r#type: "submit", "Submit Guess {settings.guess_item_cost}🪙" }
        }
    })
}
