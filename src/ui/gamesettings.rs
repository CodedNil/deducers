use crate::{
    lobby_utils::{alter_lobby_settings, AlterLobbySetting, Difficulty, Lobby},
    ITEM_NAME_PATTERN, MAX_ITEM_NAME_LENGTH, MAX_LOBBY_ITEMS,
};
use dioxus::prelude::*;
use std::{collections::HashMap, rc::Rc};

pub fn render<'a>(
    cx: Scope<'a>,
    lobby_settings_open: &'a UseState<bool>,
    player_name: &'a str,
    lobby_id: &'a str,
    lobby: &'a Lobby,
) -> Element<'a> {
    let advanced_settings_toggle: &UseState<bool> = use_state(cx, || false);
    let player_controlled = lobby.settings.player_controlled;
    let game_time = calculate_game_time(
        lobby.settings.item_count,
        lobby.settings.submit_question_every_x_seconds,
        lobby.settings.add_item_every_x_questions,
    );
    let add_item_submission: &UseState<String> = use_state(cx, String::new);

    let settings_open = if player_name == lobby.key_player {
        *lobby_settings_open.get()
    } else {
        false
    };

    let alter_setting = {
        move |setting: AlterLobbySetting| {
            let lobby_id = lobby_id.to_string();
            let player_name = player_name.to_string();
            cx.spawn(async move {
                let _result = alter_lobby_settings(&lobby_id, &player_name, setting).await;
            });
        }
    };

    cx.render(rsx! {
        div { class: "dialog floating", display: "flex", gap: "20px", top: if settings_open { "50%" } else { "-100%" },
            label { font_weight: "bold", font_size: "larger", "Lobby Settings" }
            label { font_size: "large", "Estimated game length {game_time}" }
            div { display: "flex", flex_direction: "column", gap: "5px",
                standard_settings(lobby, alter_setting),
                div { class: "dark-box",
                    label {
                        "Host as Quizmaster: "
                        input {
                            r#type: "checkbox",
                            checked: "{player_controlled}",
                            oninput: move |e| {
                                alter_setting(
                                    AlterLobbySetting::PlayerControlled(e.value.parse::<bool>().unwrap_or(false)),
                                );
                            }
                        }
                    }
                    if player_controlled {
                        item_settings(lobby, add_item_submission, alter_setting)
                    }
                }
                div { class: "dark-box",
                    label {
                        "Advanced options: "
                        input {
                            r#type: "checkbox",
                            checked: "{advanced_settings_toggle}",
                            oninput: move |e| {
                                advanced_settings_toggle.set(e.value.parse::<bool>().unwrap_or(false));
                            }
                        }
                    }
                    if *advanced_settings_toggle.get() {
                        advanced_settings(lobby, alter_setting)
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
fn calculate_game_time(items_count: usize, question_every_x_seconds: usize, item_every_x_questions: usize) -> String {
    // Initial items are added at the start, so we subtract them from the total count
    let remaining_items = if items_count > 2 { items_count - 2 } else { 0 };

    // Calculate the number of questions after which all items are added
    let final_item_added_at_question = if items_count > 2 {
        remaining_items * item_every_x_questions
    } else {
        0
    };

    // Total questions required to remove all items (20 questions per item)
    let total_questions_to_complete = 20 + final_item_added_at_question;

    // Calculate total game time in seconds
    let game_time = total_questions_to_complete * question_every_x_seconds;

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

#[allow(clippy::cast_possible_wrap)]
fn standard_settings<'a>(lobby: &Lobby, alter_setting: impl Fn(AlterLobbySetting) + 'a) -> LazyNodes<'a, 'a> {
    let difficulty = lobby.settings.difficulty.clone();
    let item_count = lobby.settings.item_count;
    let player_controlled = lobby.settings.player_controlled;
    let alter_setting = Rc::new(alter_setting);
    rsx! {
        div { display: "flex", gap: "5px",
            "Difficulty:"
            button {
                class: if difficulty == Difficulty::Easy { "highlighted" } else { "" },
                onclick: {
                    let alter_setting = alter_setting.clone();
                    move |_| {
                        alter_setting(AlterLobbySetting::Difficulty(Difficulty::Easy));
                    }
                },
                "Easy"
            }
            button {
                class: if difficulty == Difficulty::Medium { "highlighted" } else { "" },
                onclick: {
                    let alter_setting = alter_setting.clone();
                    move |_| {
                        alter_setting(AlterLobbySetting::Difficulty(Difficulty::Medium));
                    }
                },
                "Medium"
            }
            button {
                class: if difficulty == Difficulty::Hard { "highlighted" } else { "" },
                onclick: {
                    let alter_setting = alter_setting.clone();
                    move |_| {
                        alter_setting(AlterLobbySetting::Difficulty(Difficulty::Hard));
                    }
                },
                "Hard"
            }
        }
        if !player_controlled {
            let alter_setting = alter_setting.clone();
            rsx! { label {
                "Item Count: "
                input {
                    r#type: "number",
                    min: "1",
                    max: MAX_LOBBY_ITEMS as i64,
                    value: "{item_count}",
                    width: "50px",
                    oninput: {
                        let alter_setting = alter_setting.clone();
                        move |e| {
                            alter_setting(AlterLobbySetting::ItemCount(e.value.parse::<usize>().unwrap_or(1)));
                        }
                    },
                }
            }}
        }
    }
}

#[allow(clippy::cast_possible_wrap)]
fn item_settings<'a>(
    lobby: &Lobby,
    add_item_submission: &UseState<String>,
    alter_setting: impl Fn(AlterLobbySetting) + 'a,
) -> LazyNodes<'a, 'a> {
    let add_item_submission1 = add_item_submission.clone();
    let add_item_submission2 = add_item_submission.clone();
    let server_items = lobby.items_queue.clone();
    let alter_setting = Rc::new(alter_setting);
    rsx! {
        div { display: "flex", flex_direction: "column", gap: "5px",
            div {
                "Items "
                button {
                    padding: "2px",
                    padding_top: "0px",
                    background_color: "rgb(20, 100, 150)",
                    onclick: {
                        let alter_setting = alter_setting.clone();
                        move |_| {
                            alter_setting(AlterLobbySetting::RefreshAllItems);
                        }
                    },
                    "↻"
                }
            }
            server_items.into_iter().map(|item| {
                let item1 = item.clone();
                let alter_setting = alter_setting.clone();
                rsx! {
                    div { display: "flex", flex_direction: "row", gap: "5px",
                        class: "table-body-box",
                        item.clone()
                        button {
                            padding: "2px",
                            padding_top: "0px",
                            background_color: "rgb(20, 100, 150)",
                            onclick: {
                                let alter_setting = alter_setting.clone();
                                move |_| {
                                    alter_setting(AlterLobbySetting::RefreshItem(item.clone()));
                                }
                            },
                            "↻"
                        }
                        button {
                            class: "smallbutton",
                            background_color: "rgb(100, 20, 20)",
                            onclick: {
                                let alter_setting = alter_setting.clone();
                                move |_| {
                                    alter_setting(AlterLobbySetting::RemoveItem(item1.clone()));
                                }
                            },
                            "-"
                        }
                    }
                }
            }),
            form {
                display: "flex",
                gap: "5px",
                onsubmit: {
                    let alter_setting = alter_setting.clone();
                    let submission = add_item_submission1.get().clone();
                    move |_| {
                        alter_setting(AlterLobbySetting::AddItem(submission.clone()));
                    }
                },
                input {
                    r#type: "text",
                    placeholder: "Add item",
                    maxlength: MAX_ITEM_NAME_LENGTH as i64,
                    pattern: ITEM_NAME_PATTERN,
                    "data-clear-on-submit": "true",
                    oninput: move |e| {
                        add_item_submission2.set(e.value.clone());
                    }
                }
                button { background_color: "rgb(20, 100, 20)", r#type: "submit", "+" }
            }
        }
    }
}

struct SettingDetail {
    key: String,
    display_name: String,
    min: usize,
    max: usize,
}

impl SettingDetail {
    fn new(key: &str, min: usize, max: usize) -> Self {
        Self {
            key: key.to_string(),
            display_name: key
                .split('_')
                .map(|word| word.chars().next().unwrap().to_uppercase().to_string() + &word.chars().skip(1).collect::<String>())
                .collect::<Vec<String>>()
                .join(" "),
            min,
            max,
        }
    }
}

fn advanced_settings<'a>(lobby: &'a Lobby, alter_setting: impl Fn(AlterLobbySetting) + 'a) -> LazyNodes<'a, 'a> {
    let alter_setting = Rc::new(alter_setting);
    let settings = vec![
        SettingDetail::new("starting_coins", 1, 100),
        SettingDetail::new("coin_every_x_seconds", 1, 20),
        SettingDetail::new("submit_question_every_x_seconds", 1, 30),
        SettingDetail::new("add_item_every_x_questions", 1, 20),
        SettingDetail::new("submit_question_cost", 1, 100),
        SettingDetail::new("masked_question_cost", 1, 100),
        SettingDetail::new("guess_item_cost", 1, 100),
        SettingDetail::new("question_min_votes", 1, 20),
        SettingDetail::new("score_to_coins_ratio", 1, 100),
    ];

    let setting_values: HashMap<&str, usize> = [
        ("starting_coins", lobby.settings.starting_coins),
        ("coin_every_x_seconds", lobby.settings.coin_every_x_seconds),
        ("submit_question_every_x_seconds", lobby.settings.submit_question_every_x_seconds),
        ("add_item_every_x_questions", lobby.settings.add_item_every_x_questions),
        ("submit_question_cost", lobby.settings.submit_question_cost),
        ("masked_question_cost", lobby.settings.masked_question_cost),
        ("guess_item_cost", lobby.settings.guess_item_cost),
        ("question_min_votes", lobby.settings.question_min_votes),
        ("score_to_coins_ratio", lobby.settings.score_to_coins_ratio),
    ]
    .iter()
    .copied()
    .collect();

    rsx! {
        settings.into_iter().map(|setting| {
            let alter_setting = alter_setting.clone();
            setting_values.get(setting.key.as_str()).map_or_else(|| rsx! { div {} }, |&setting_value|
                rsx! {
                    label {
                        "{setting.display_name}: "
                        input {
                            r#type: "number",
                            min: "{setting.min}",
                            max: "{setting.max}",
                            value: "{setting_value}",
                            max_width: "50px",
                            oninput: {
                                move |e| {
                                    alter_setting(AlterLobbySetting::Advanced(setting.key.clone(), e.value.parse::<usize>().unwrap_or(1)));
                                }
                            }
                        }
                    }
                }
            )
        })
    }
}
