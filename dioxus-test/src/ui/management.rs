use crate::{
    Lobby, ANONYMOUS_QUESTION_COST, GUESS_ITEM_COST, SCORE_TO_COINS_RATIO, SUBMIT_QUESTION_COST,
};
use dioxus::prelude::*;

pub fn render<'a>(cx: Scope<'a>, player_name: &String, lobby: &Lobby) -> Element<'a> {
    let question_submission: &UseState<String> = use_state(cx, String::new);
    let question_anonymous: &UseState<bool> = use_state(cx, || false);
    let guess_item_submission: &UseState<String> = use_state(cx, String::new);
    let guess_item_key: &UseState<usize> = use_state(cx, || 0);

    let submit_cost = SUBMIT_QUESTION_COST
        + if *question_anonymous.get() {
            ANONYMOUS_QUESTION_COST
        } else {
            0
        };

    let players_coins = lobby.players.get(player_name).unwrap().coins;

    cx.render(rsx! {
        div { align_self: "center", font_size: "larger", "{players_coins}🪙 Available" }
        div { display: "flex", gap: "5px",
            input {
                value: "{question_submission}",
                placeholder: "Question To Ask",
                flex: "1",
                oninput: move |e| {
                    let input = e.value.clone();
                    let filtered_input: String = input
                        .chars()
                        .filter(|c| c.is_alphanumeric())
                        .take(20)
                        .collect();
                    question_submission.set(filtered_input);
                }
            }
            button { "Submit Question {submit_cost}🪙" }
        }
        div { display: "flex", gap: "5px", justify_content: "center",
            input {
                r#type: "checkbox",
                checked: "{question_anonymous}",
                onclick: move |_| {
                    question_anonymous.set(!question_anonymous.get());
                }
            }
            "Anonymous +{ANONYMOUS_QUESTION_COST}🪙"
        }
        div { display: "flex", gap: "5px",
            button { flex: "1", "Convert Leaderboard Score To {SCORE_TO_COINS_RATIO}🪙" }
        }
        div { display: "flex", gap: "5px",
            input {
                value: "{guess_item_submission}",
                placeholder: "Guess Item",
                flex: "1",
                oninput: move |e| {
                    let input = e.value.clone();
                    let filtered_input: String = input
                        .chars()
                        .filter(|c| c.is_alphanumeric())
                        .take(20)
                        .collect();
                    guess_item_submission.set(filtered_input);
                }
            }
            select {
                option { "Item 1" }
                option { "Item 2" }
                option { "Item 3" }
            }
            button { "Submit Guess {GUESS_ITEM_COST}🪙" }
        }
    })
}
