use crate::{
    backend::{
        items::player_guess_item,
        question_queue::{convert_score, submit_question},
        Lobby,
    },
    frontend::{quizmaster, AlertPopup},
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
    let question_submission: &UseState<String> = use_state(cx, String::new);
    let question_masked: &UseState<bool> = use_state(cx, || false);
    let guess_item_submission: &UseState<String> = use_state(cx, String::new);
    let guess_item_key: &UseState<usize> = use_state(cx, || 1);

    let submit_cost = lobby.settings.submit_question_cost
        + if *question_masked.get() {
            lobby.settings.masked_question_cost
        } else {
            0
        };
    let players_coins = lobby.players[player_name].coins;

    if !lobby.started {
        return cx.render(rsx! { div { align_self: "center", font_size: "larger", "Waiting for game to start" } });
    }

    if player_name == lobby.key_player && lobby.settings.player_controlled {
        return quizmaster::render(cx, player_name, lobby_id, lobby);
    }

    cx.render(rsx! {
        div { align_self: "center", font_size: "larger", "{players_coins}🪙 Available" }
        form {
            display: "flex",
            gap: "5px",
            onsubmit: move |_| {
                let (lobby_id, player_name) = (lobby_id.to_owned(), player_name.to_owned());
                let input = question_submission.get().clone();
                let masked = question_masked.clone();
                let alert_popup = alert_popup.clone();
                cx.spawn(async move {
                    if let Err(error)
                        = submit_question(&lobby_id, &player_name, input, *masked.get()).await
                    {
                        alert_popup.set(AlertPopup::message(error.to_string()));
                    } else {
                        masked.set(false);
                    }
                });
            },
            input {
                r#type: "text",
                placeholder: "Question To Ask",
                flex: "1",
                pattern: QUESTION_PATTERN,
                maxlength: MAX_QUESTION_LENGTH as i64,
                "data-clear-on-submit": "true",
                oninput: move |e| {
                    question_submission.set(e.value.clone());
                }
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
            "Masked +{lobby.settings.masked_question_cost}🪙"
        }
        div { display: "flex", gap: "5px",
            button {
                onclick: move |_| {
                    if let Err(error) = convert_score(lobby_id, player_name) {
                        alert_popup.set(AlertPopup::message(error.to_string()));
                    }
                },
                flex: "1",
                "Convert Leaderboard Score To {lobby.settings.score_to_coins_ratio}🪙"
            }
        }
        form {
            display: "flex",
            gap: "5px",
            onsubmit: move |_| {
                if let Err(error)
                    = player_guess_item(
                        lobby_id,
                        player_name,
                        *guess_item_key.get(),
                        guess_item_submission,
                    ) {
                    alert_popup.set(AlertPopup::message(error.to_string()));
                }
            },
            input {
                r#type: "text",
                placeholder: "Guess Item",
                flex: "1",
                maxlength: MAX_ITEM_NAME_LENGTH as i64,
                pattern: ITEM_NAME_PATTERN,
                "data-clear-on-submit": "true",
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
            button { r#type: "submit", "Submit Guess {lobby.settings.guess_item_cost}🪙" }
        }
    })
}
