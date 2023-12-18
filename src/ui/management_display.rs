use crate::{
    backend::{
        items::player_guess_item,
        question_queue::{player_convert_score, player_submit_question},
    },
    get_current_time, Lobby, ANONYMOUS_QUESTION_COST, GUESS_ITEM_COST, GUESS_ITEM_PATTERN, MAX_GUESS_ITEM_LENGTH, MAX_QUESTION_LENGTH,
    QUESTION_PATTERN, SCORE_TO_COINS_RATIO, SUBMIT_QUESTION_COST,
};
use anyhow::Error;
use dioxus::prelude::*;

#[allow(clippy::too_many_lines, clippy::cast_possible_wrap)]
pub fn render<'a>(
    cx: Scope<'a>,
    player_name: &'a String,
    lobby_id: &'a str,
    lobby: &Lobby,
    alert_popup: &'a UseState<Option<(f64, String)>>,
) -> Element<'a> {
    let question_submission: &UseState<String> = use_state(cx, String::new);
    let question_anonymous: &UseState<bool> = use_state(cx, || false);
    let guess_item_submission: &UseState<String> = use_state(cx, String::new);
    let guess_item_key: &UseState<usize> = use_state(cx, || 1);

    let submit_cost = SUBMIT_QUESTION_COST + if *question_anonymous.get() { ANONYMOUS_QUESTION_COST } else { 0 };
    let players_coins = lobby.players.get(player_name).unwrap().coins;

    let handle_error = |error: Error| {
        alert_popup.set(Some((get_current_time() + 5.0, error.to_string())));
    };

    cx.render(rsx! {
        div { align_self: "center", font_size: "larger", "{players_coins}🪙 Available" }
        form {
            display: "flex",
            gap: "5px",
            onsubmit: move |_| {
                let lobby_id = lobby_id.to_string();
                let player_name = player_name.clone();
                let question_submission = question_submission.get().clone();
                let question_anonymous = question_anonymous.clone();
                let alert_popup = alert_popup.clone();
                cx.spawn(async move {
                    match player_submit_question(
                            lobby_id,
                            player_name,
                            question_submission,
                            *question_anonymous.get(),
                        )
                        .await
                    {
                        Ok(()) => {
                            question_anonymous.set(false);
                        }
                        Err(error) => {
                            alert_popup.set(Some((get_current_time() + 5.0, format!("{error}"))));
                        }
                    };
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
                    let lobby_id = lobby_id.to_string();
                    let player_name = player_name.clone();
                    let alert_popup = alert_popup.clone();
                    cx.spawn(async move {
                        player_convert_score(lobby_id, player_name)
                            .await
                            .map_err(|error| {
                                alert_popup.set(Some((get_current_time() + 5.0, format!("{error}"))));
                            })
                            .ok();
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
                let lobby_id = lobby_id.to_string();
                let player_name = player_name.clone();
                let item_choice = *guess_item_key.get();
                let item_guess = guess_item_submission.get().clone();
                let alert_popup = alert_popup.clone();
                cx.spawn(async move {
                    player_guess_item(lobby_id, player_name, item_choice, item_guess)
                        .await
                        .map_err(|error| {
                            alert_popup.set(Some((get_current_time() + 5.0, format!("{error}"))));
                        })
                        .ok();
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
