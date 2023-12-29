use crate::{
    backend::{alter_lobby_settings, AlterLobbySetting, Difficulty, LobbySettings},
    ITEM_NAME_PATTERN, MAX_ITEM_NAME_LENGTH, MAX_LOBBY_ITEMS,
};
use dioxus::prelude::*;
use std::{collections::HashMap, rc::Rc};
use strum::IntoEnumIterator;

#[component]
pub fn GameSettings(cx: Scope, player_name: String, lobby_id: String, settings: LobbySettings, items_queue: Vec<String>) -> Element {
    let advanced_settings_toggle = use_state(cx, || false);
    let player_controlled = settings.player_controlled;
    let game_time = calculate_game_time(
        settings.item_count,
        settings.submit_question_every_x_seconds,
        settings.add_item_every_x_questions,
    );

    let alter_setting = {
        move |setting: AlterLobbySetting| {
            let _result = alter_lobby_settings(lobby_id, player_name, setting);
        }
    };

    cx.render(rsx! {
        div { class: "dialog floating", display: "flex", gap: "20px", top: "50%",
            label { font_weight: "bold", font_size: "larger", "Lobby Settings" }
            label { font_size: "large", "Estimated game length {game_time}" }
            div { display: "flex", flex_direction: "column", gap: "5px",
                standard_settings(*settings, alter_setting),
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
                        item_settings(items_queue.clone(), alter_setting)
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
                        advanced_settings(*settings, alter_setting)
                    }
                }
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
fn standard_settings<'a>(settings: LobbySettings, alter_setting: impl Fn(AlterLobbySetting) + 'a) -> LazyNodes<'a, 'a> {
    let alter_setting = Rc::new(alter_setting);
    rsx! {
        div { display: "flex", gap: "5px",
            "Difficulty:"
            Difficulty::iter().map(|variant| {
                let alter_setting = Rc::clone(&alter_setting);
                rsx! {
                    button {
                        class: if settings.difficulty == variant { "highlighted" } else { "" },
                        onclick: {
                            let alter_setting = Rc::clone(&alter_setting);
                            move |_| {
                                alter_setting(AlterLobbySetting::Difficulty(variant));
                            }
                        },
                        "{variant}"
                    }
                }
            })
        }
        if !settings.player_controlled {
            rsx! { label {
                "Item Count: "
                input {
                    r#type: "number",
                    min: "1",
                    max: MAX_LOBBY_ITEMS as i64,
                    value: "{settings.item_count}",
                    width: "50px",
                    oninput: {
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
fn item_settings<'a>(items_queue: Vec<String>, alter_setting: impl Fn(AlterLobbySetting) + 'a) -> LazyNodes<'a, 'a> {
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
                        let alter_setting = Rc::clone(&alter_setting);
                        move |_| {
                            alter_setting(AlterLobbySetting::RefreshAllItems);
                        }
                    },
                    "↻"
                }
            }
            items_queue.into_iter().map(|item| {
                let item1 = item.clone();
                let alter_setting = Rc::clone(&alter_setting);
                rsx! {
                    div { display: "flex", flex_direction: "row", gap: "5px",
                        class: "table-body-box",
                        item.clone()
                        button {
                            padding: "2px",
                            padding_top: "0px",
                            background_color: "rgb(20, 100, 150)",
                            onclick: {
                                let alter_setting = Rc::clone(&alter_setting);
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
                                let alter_setting = Rc::clone(&alter_setting);
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
                    let alter_setting = Rc::clone(&alter_setting);
                    move |form_data| {
                        if let Some(item_name)
                            = form_data.values.get("item_name").and_then(|m| m.first())
                        {
                            alter_setting(AlterLobbySetting::AddItem(item_name.clone()));
                        }
                    }
                },
                input {
                    r#type: "text",
                    placeholder: "Add item",
                    name: "item_name",
                    maxlength: MAX_ITEM_NAME_LENGTH as i64,
                    pattern: ITEM_NAME_PATTERN,
                    "data-clear-on-submit": "true"
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
            key: key.to_owned(),
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

fn advanced_settings<'a>(settings: LobbySettings, alter_setting: impl Fn(AlterLobbySetting) + 'a) -> LazyNodes<'a, 'a> {
    let alter_setting = Rc::new(alter_setting);
    let setting_details = vec![
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
        ("starting_coins", settings.starting_coins),
        ("coin_every_x_seconds", settings.coin_every_x_seconds),
        ("submit_question_every_x_seconds", settings.submit_question_every_x_seconds),
        ("add_item_every_x_questions", settings.add_item_every_x_questions),
        ("submit_question_cost", settings.submit_question_cost),
        ("masked_question_cost", settings.masked_question_cost),
        ("guess_item_cost", settings.guess_item_cost),
        ("question_min_votes", settings.question_min_votes),
        ("score_to_coins_ratio", settings.score_to_coins_ratio),
    ]
    .iter()
    .copied()
    .collect();

    rsx! {
        setting_details.into_iter().map(|setting| {
            let alter_setting = Rc::clone(&alter_setting);
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
