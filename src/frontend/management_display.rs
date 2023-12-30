use crate::{
    backend::{
        items::player_guess_item_wrapped,
        question_queue::{convert_score, submit_question_wrapped},
        Item, LobbySettings, PlayerReduced, QueuedQuestionQuizmaster,
    },
    frontend::quizmaster::QuizmasterDisplay,
    ITEM_NAME_PATTERN, MAX_ITEM_NAME_LENGTH, MAX_QUESTION_LENGTH, QUESTION_PATTERN,
};
use dioxus::prelude::*;

#[allow(clippy::cast_possible_wrap)]
#[component]
pub fn Management(
    cx: Scope,
    player_name: String,
    lobby_id: String,
    key_player: String,
    settings: LobbySettings,
    players: Vec<PlayerReduced>,
    items: Vec<Item>,
    quizmaster_queue: Vec<QueuedQuestionQuizmaster>,
) -> Element {
    let question_masked = use_state(cx, || false);

    let submit_cost = settings.submit_question_cost + if *question_masked.get() { settings.masked_question_cost } else { 0 };
    let players_coins = players.iter().find(|p| &p.name == player_name).map_or(0, |p| p.coins);

    if player_name == key_player && settings.player_controlled {
        return cx
            .render(rsx! {QuizmasterDisplay { player_name: player_name, lobby_id: lobby_id, quizmaster_queue: quizmaster_queue.clone() }});
    }

    cx.render(rsx! {
        div { align_self: "center", font_size: "larger", "{players_coins}🪙 Available" }
        form {
            onsubmit: move |form_data| {
                if let Some(question) = form_data.values.get("question").and_then(|m| m.first()) {
                    let question = question.to_owned();
                    let (lobby_id, player_name) = (lobby_id.to_owned(), player_name.to_owned());
                    let masked = question_masked.clone();
                    cx.spawn(async move {
                        submit_question_wrapped(
                                &lobby_id,
                                &player_name,
                                question.clone(),
                                *masked.get(),
                            )
                            .await;
                        masked.set(false);
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
                let key = form_data
                    .values
                    .get("key")
                    .and_then(|m| m.first())
                    .and_then(|k| k.parse::<usize>().ok());
                if let (Some(guess), Some(key)) = (guess, key) {
                    player_guess_item_wrapped(lobby_id, player_name, key, guess);
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
