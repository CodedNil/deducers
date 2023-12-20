use crate::{
    lobby_utils::{alter_lobby_settings, AlterLobbySetting, Difficulty, Lobby},
    ITEM_NAME_PATTERN, MAX_ITEM_NAME_LENGTH, MAX_LOBBY_ITEMS,
};
use dioxus::prelude::*;

pub fn render<'a>(
    cx: Scope<'a>,
    lobby_settings_open: &'a UseState<bool>,
    player_name: &'a str,
    lobby_id: &'a str,
    lobby: &Lobby,
) -> Element<'a> {
    let advanced_settings: &UseState<bool> = use_state(cx, || false);
    let player_controlled = lobby.settings.player_controlled;
    let game_time = calculate_game_time(lobby.settings.item_count, lobby.settings.submit_question_every_x_seconds);

    let settings_open = if player_name == lobby.key_player {
        *lobby_settings_open.get()
    } else {
        false
    };

    cx.render(rsx! {
        div { class: "dialog floating", display: "flex", gap: "20px", top: if settings_open { "50%" } else { "-100%" },
            label { font_weight: "bold", font_size: "larger", "Lobby Settings" }
            div { display: "flex", flex_direction: "column", gap: "5px",
                standard_settings(cx, player_name, lobby_id, lobby),
                "Estimated game length {game_time}"
                if player_controlled {
                    item_settings(cx, player_name, lobby_id, lobby)
                }
                label {
                    "Advanced options: "
                    input {
                        r#type: "checkbox",
                        checked: "{advanced_settings}",
                        oninput: move |e| {
                            advanced_settings.set(e.value.parse::<bool>().unwrap_or(false));
                        }
                    }
                }
            }
            button {
                onclick: move |_| {
                    lobby_settings_open.set(!lobby_settings_open.get());
                },
                font_weight: "bold",
                "Close"
            }
        }
    })
}

// Function to estimate game time in seconds
#[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn calculate_game_time(items_count: usize, every_x_seconds: usize) -> String {
    // Initial items are added at the start, so we subtract them from the total count
    let remaining_items = if items_count > 2 { items_count - 2 } else { 0 };

    // Calculate the number of questions after which all items are added
    let final_item_added_at_question = if items_count > 2 { remaining_items * 5 } else { 0 };

    // Total questions required to remove all items (20 questions per item)
    let total_questions_to_complete = 20 + final_item_added_at_question;

    // Calculate total game time in seconds
    let game_time = total_questions_to_complete * every_x_seconds;

    if game_time < 60 {
        // If less than a minute, display just in seconds
        format!("{game_time} seconds")
    } else {
        // Calculate minutes and seconds
        let minutes = game_time / 60;
        let seconds = game_time % 60;
        format!("{minutes} minutes {seconds} seconds")
    }
}

#[allow(clippy::too_many_lines, clippy::cast_possible_wrap)]
fn standard_settings<'a>(cx: Scope<'a>, player_name: &'a str, lobby_id: &'a str, lobby: &Lobby) -> LazyNodes<'a, 'a> {
    let difficulty = lobby.settings.difficulty.clone();
    let item_count = lobby.settings.item_count;
    let player_controlled = lobby.settings.player_controlled;
    rsx! {
        div { display: "flex", gap: "5px",
            "Difficulty:"
            button {
                class: if difficulty == Difficulty::Easy { "highlighted" } else { "" },
                onclick: move |_| {
                    let lobby_id = lobby_id.to_string();
                    let player_name = player_name.to_string();
                    cx.spawn(async move {
                        let _result = alter_lobby_settings(
                                lobby_id,
                                player_name,
                                AlterLobbySetting::Difficulty(Difficulty::Easy),
                            )
                            .await;
                    });
                },
                "Easy"
            }
            button {
                class: if difficulty == Difficulty::Medium { "highlighted" } else { "" },
                onclick: move |_| {
                    let lobby_id = lobby_id.to_string();
                    let player_name = player_name.to_string();
                    cx.spawn(async move {
                        let _result = alter_lobby_settings(
                                lobby_id,
                                player_name,
                                AlterLobbySetting::Difficulty(Difficulty::Medium),
                            )
                            .await;
                    });
                },
                "Medium"
            }
            button {
                class: if difficulty == Difficulty::Hard { "highlighted" } else { "" },
                onclick: move |_| {
                    let lobby_id = lobby_id.to_string();
                    let player_name = player_name.to_string();
                    cx.spawn(async move {
                        let _result = alter_lobby_settings(
                                lobby_id,
                                player_name,
                                AlterLobbySetting::Difficulty(Difficulty::Hard),
                            )
                            .await;
                    });
                },
                "Hard"
            }
        }
        if !player_controlled {
            rsx! { label {
                "Item Count: "
                input {
                    r#type: "number",
                    min: "1",
                    max: MAX_LOBBY_ITEMS as i64,
                    value: "{item_count}",
                    width: "50px",
                    oninput: move |e| {
                        let lobby_id = lobby_id.to_string();
                        let player_name = player_name.to_string();
                        let count = e.value.parse::<usize>().unwrap_or(1);
                        cx.spawn(async move {
                            let _result = alter_lobby_settings(
                                    lobby_id,
                                    player_name,
                                    AlterLobbySetting::ItemCount(count),
                                )
                                .await;
                        });
                    }
                }
            }}
        }
        label {
            "Host as Quizmaster: "
            input {
                r#type: "checkbox",
                checked: "{player_controlled}",
                oninput: move |e| {
                    let lobby_id = lobby_id.to_string();
                    let player_name = player_name.to_string();
                    let checked = e.value.parse::<bool>().unwrap_or(false);
                    cx.spawn(async move {
                        let _result = alter_lobby_settings(
                                lobby_id,
                                player_name,
                                AlterLobbySetting::PlayerControlled(checked),
                            )
                            .await;
                    });
                }
            }
        }
    }
}

#[allow(clippy::too_many_lines, clippy::cast_possible_wrap)]
fn item_settings<'a>(cx: Scope<'a>, player_name: &'a str, lobby_id: &'a str, lobby: &Lobby) -> LazyNodes<'a, 'a> {
    let add_submission: &UseState<String> = use_state(cx, String::new);
    let server_items = lobby.items_queue.clone();

    rsx! {
        div { display: "flex", flex_direction: "column", gap: "5px",
            div { class: "table-header-box",
                "Items "
                button {
                    padding: "2px",
                    padding_top: "0px",
                    background_color: "rgb(20, 100, 150)",
                    onclick: move |_| {
                        let lobby_id = lobby_id.to_string();
                        let player_name = player_name.to_string();
                        cx.spawn(async move {
                            let _result = alter_lobby_settings(
                                    lobby_id,
                                    player_name,
                                    AlterLobbySetting::RefreshAllItems,
                                )
                                .await;
                        });
                    },
                    "↻"
                }
            }
            server_items.into_iter().map(|item| {
                let item1 = item.clone();
                let item2 = item.clone();
                rsx! {
                    div { display: "flex", flex_direction: "row", gap: "5px",
                        class: "table-body-box",
                        item.clone()
                        button {
                            padding: "2px",
                            padding_top: "0px",
                            background_color: "rgb(20, 100, 150)",
                            onclick: move |_| {
                                let lobby_id = lobby_id.to_string();
                                let player_name = player_name.to_string();
                                let item = item1.clone();
                                cx.spawn(async move {
                                    let _result = alter_lobby_settings(
                                            lobby_id,
                                            player_name,
                                            AlterLobbySetting::RefreshItem(item),
                                        )
                                        .await;
                                });
                            },
                            "↻"
                        }
                        button {
                            padding: "2px",
                            padding_top: "0px",
                            background_color: "rgb(100, 20, 20)",
                            onclick: move |_| {
                                let lobby_id = lobby_id.to_string();
                                let player_name = player_name.to_string();
                                let item = item2.clone();
                                cx.spawn(async move {
                                    let _result = alter_lobby_settings(
                                            lobby_id,
                                            player_name,
                                            AlterLobbySetting::RemoveItem(item),
                                        )
                                        .await;
                                });
                            },
                            "-"
                        }
                    }
                }
            }),
            form {
                display: "flex",
                gap: "5px",
                onsubmit: move |_| {
                    let lobby_id = lobby_id.to_string();
                    let player_name = player_name.to_string();
                    let submission = add_submission.get().clone();
                    cx.spawn(async move {
                        let _result = alter_lobby_settings(
                                lobby_id,
                                player_name,
                                AlterLobbySetting::AddItem(submission),
                            )
                            .await;
                    });
                },
                input {
                    r#type: "text",
                    placeholder: "Add item",
                    maxlength: MAX_ITEM_NAME_LENGTH as i64,
                    pattern: ITEM_NAME_PATTERN,
                    "data-clear-on-submit": "true",
                    oninput: move |e| {
                        add_submission.set(e.value.clone());
                    }
                }
                button { background_color: "rgb(20, 100, 20)", r#type: "submit", "+" }
            }
        }
    }
}
