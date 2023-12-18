use super::gameview::AlertPopup;
use crate::{
    backend::{
        items::player_guess_item,
        question_queue::{convert_score, submit_question},
    },
    lobby_utils::Lobby,
    ANONYMOUS_QUESTION_COST, GUESS_ITEM_COST, GUESS_ITEM_PATTERN, MAX_GUESS_ITEM_LENGTH, MAX_QUESTION_LENGTH, QUESTION_PATTERN,
    SCORE_TO_COINS_RATIO, SUBMIT_QUESTION_COST,
};
use dioxus::prelude::*;

#[allow(clippy::too_many_lines, clippy::cast_possible_wrap)]
pub fn render<'a>(
    cx: Scope<'a>,
    player_name: &'a String,
    lobby_id: &'a str,
    lobby: &Lobby,
    alert_popup: &'a UseState<AlertPopup>,
) -> Element<'a> {
    let question_submission: &UseState<String> = use_state(cx, String::new);
    let question_anonymous: &UseState<bool> = use_state(cx, || false);
    let guess_item_submission: &UseState<String> = use_state(cx, String::new);
    let guess_item_key: &UseState<usize> = use_state(cx, || 1);

    let submit_cost = SUBMIT_QUESTION_COST + if *question_anonymous.get() { ANONYMOUS_QUESTION_COST } else { 0 };
    let players_coins = lobby.players.get(player_name).unwrap().coins;

    cx.render(rsx! {
        div { align_self: "center", font_size: "larger", "{players_coins}🪙 Available" }
        form {
            display: "flex",
            gap: "5px",
            onsubmit: move |_| {
                let (lobby_id, player_name) = (lobby_id.to_string(), player_name.clone());
                let input = question_submission.get().clone();
                let anon = question_anonymous.clone();
                let alert_popup = alert_popup.clone();
                cx.spawn(async move {
                    if let Err(error)
                        = submit_question(lobby_id, player_name, input, *anon.get()).await
                    {
                        alert_popup.set(AlertPopup::error(&error));
                    } else {
                        anon.set(false);
                    }
                });
            },
            input {
                r#type: "text",
                placeholder: "Question To Ask",
                flex: "1",
                pattern: QUESTION_PATTERN,
                maxlength: MAX_QUESTION_LENGTH as i64,
                oninput: move |e| {
                    question_submission.set(e.value.clone());
                }
            }
            button { r#type: "submit", "Submit Question {submit_cost}🪙" }
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
            button {
                onclick: move |_| {
                    let (lobby_id, player_name) = (lobby_id.to_string(), player_name.clone());
                    let alert_popup = alert_popup.clone();
                    cx.spawn(async move {
                        if let Err(error) = convert_score(lobby_id, player_name).await {
                            alert_popup.set(AlertPopup::error(&error));
                        }
                    });
                },
                flex: "1",
                "Convert Leaderboard Score To {SCORE_TO_COINS_RATIO}🪙"
            }
        }
        form {
            display: "flex",
            gap: "5px",
            onsubmit: move |_| {
                let (lobby_id, player_name) = (lobby_id.to_string(), player_name.clone());
                let item_choice = *guess_item_key.get();
                let item_guess = guess_item_submission.get().clone();
                let alert_popup = alert_popup.clone();
                cx.spawn(async move {
                    if let Err(error)
                        = player_guess_item(lobby_id, player_name, item_choice, item_guess).await
                    {
                        alert_popup.set(AlertPopup::error(&error));
                    }
                });
            },
            input {
                r#type: "text",
                placeholder: "Guess Item",
                flex: "1",
                maxlength: MAX_GUESS_ITEM_LENGTH as i64,
                pattern: GUESS_ITEM_PATTERN,
                oninput: move |e| {
                    guess_item_submission.set(e.value.clone());
                }
            }
            select {
                onchange: move |event| {
                    if let Ok(selected_key) = event.value.parse::<usize>() {
                        guess_item_key.set(selected_key);
                    }
                },
                lobby.items.iter().map(|item| {
                    rsx! {
                        option { "{item.id}" }
                    }
                })
            }
            button { r#type: "submit", "Submit Guess {GUESS_ITEM_COST}🪙" }
        }
    })
}
