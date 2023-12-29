use crate::{
    backend::{
        items::player_guess_item,
        question_queue::{convert_score, submit_question},
        Lobby,
    },
    frontend::{quizmaster::QuizmasterDisplay, AlertPopup},
    ITEM_NAME_PATTERN, MAX_ITEM_NAME_LENGTH, MAX_QUESTION_LENGTH, QUESTION_PATTERN,
};
use dioxus::prelude::*;

#[allow(clippy::cast_possible_wrap)]
pub fn render<'a>(
    cx: Scope<'a>,
    player_name: &'a str,
    lobby_id: &'a str,
    lobby: &Lobby,
    alert_popup: &'a UseState<AlertPopup>,
) -> Element<'a> {
    let question_masked: &UseState<bool> = use_state(cx, || false);

    let settings = lobby.settings;

    let submit_cost = settings.submit_question_cost + if *question_masked.get() { settings.masked_question_cost } else { 0 };
    let players_coins = lobby.players[player_name].coins;

    if player_name == lobby.key_player && settings.player_controlled {
        return cx.render(
            rsx! {QuizmasterDisplay { player_name: player_name, lobby_id: lobby_id, quizmaster_queue: lobby.quizmaster_queue.clone() }},
        );
    }

    cx.render(rsx! {
        div { align_self: "center", font_size: "larger", "{players_coins}🪙 Available" }
        form {
            display: "flex",
            gap: "5px",
            onsubmit: move |form_data| {
                if let Some(question) = form_data.values.get("question").and_then(|m| m.first()) {
                    let question = question.to_owned();
                    let (lobby_id, player_name) = (lobby_id.to_owned(), player_name.to_owned());
                    let masked = question_masked.clone();
                    let alert_popup = alert_popup.clone();
                    cx.spawn(async move {
                        if let Err(error)
                            = submit_question(
                                    &lobby_id,
                                    &player_name,
                                    question.clone(),
                                    *masked.get(),
                                )
                                .await
                        {
                            alert_popup.set(AlertPopup::message(error.to_string()));
                        } else {
                            masked.set(false);
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
                    if let Err(error) = convert_score(lobby_id, player_name) {
                        alert_popup.set(AlertPopup::message(error.to_string()));
                    }
                },
                flex: "1",
                "Convert Leaderboard Score To {settings.score_to_coins_ratio}🪙"
            }
        }
        form {
            display: "flex",
            gap: "5px",
            onsubmit: move |form_data| {
                let guess = form_data.values.get("guess").and_then(|m| m.first());
                let key = form_data
                    .values
                    .get("key")
                    .and_then(|m| m.first())
                    .and_then(|k| k.parse::<usize>().ok());
                if let (Some(guess), Some(key)) = (guess, key) {
                    if let Err(error) = player_guess_item(lobby_id, player_name, key, guess) {
                        alert_popup.set(AlertPopup::message(error.to_string()));
                    }
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
                lobby.items.iter().map(|item| {
                    rsx! {
                        option { "{item.id}" }
                    }
                })
            }
            button { r#type: "submit", "Submit Guess {settings.guess_item_cost}🪙" }
        }
    })
}
