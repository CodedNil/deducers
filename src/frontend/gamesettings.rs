use crate::{
    backend::{alter_lobby_settings, start_lobby, AlterLobbySetting, Difficulty, LobbySettings},
    ITEM_NAME_PATTERN, MAX_ITEM_NAME_LENGTH, MAX_LOBBY_ITEMS, QUESTION_PATTERN,
};
use dioxus::prelude::*;
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
            alter_lobby_settings(lobby_id, player_name, setting);
        }
    };

    cx.render(rsx! {
        div { class: "dialog true", display: "flex", gap: "20px", max_height: "80vh", overflow_y: "auto",
            label { font_weight: "bold", font_size: "larger", "Lobby Settings" }
            label { font_size: "large", "Estimated game length {game_time}" }
            button {
                onclick: move |_| {
                    start_lobby(lobby_id, player_name);
                },
                "Start"
            }
            div { display: "flex", flex_direction: "column", gap: "5px", align_items: "center",
                StandardSettings {
                    player_name: player_name.clone(),
                    lobby_id: lobby_id.clone(),
                    settings: settings.clone()
                }
                div { display: "flex", flex_direction: "row", gap: "5px",
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
                            rsx! { ItemSettings {
                                player_name: player_name.clone(),
                                lobby_id: lobby_id.clone(),
                                items_queue: items_queue.clone(),
                                settings: settings.clone(),
                            }}
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
                            rsx! { AdvancedSettings {
                                player_name: player_name.clone(),
                                lobby_id: lobby_id.clone(),
                                settings: settings.clone(),
                            }}
                        }
                    }
                }
            }
        }
    })
}

// Function to estimate game time in seconds
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

#[component]
pub fn StandardSettings(cx: Scope, player_name: String, lobby_id: String, settings: LobbySettings) -> Element {
    let alter_setting = {
        move |setting: AlterLobbySetting| {
            alter_lobby_settings(lobby_id, player_name, setting);
        }
    };
    cx.render(rsx! {
        div { display: "flex", gap: "5px",
            label {
                "Theme: "
                input {
                    r#type: "text",
                    placeholder: "None",
                    maxlength: 20,
                    pattern: QUESTION_PATTERN,
                    oninput: {
                        move |e| {
                            alter_setting(AlterLobbySetting::Theme(e.value.clone()));
                        }
                    }
                }
            }
        }
        div { display: "flex", gap: "5px",
            "Difficulty:"
            for variant in Difficulty::iter() {
                button {
                    class: if settings.difficulty == variant { "highlighted" } else { "" },
                    onclick: {
                        move |_| {
                            alter_setting(AlterLobbySetting::Difficulty(variant));
                        }
                    },
                    "{variant}"
                }
            }
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
    })
}

#[component]
pub fn ItemSettings(cx: Scope, player_name: String, lobby_id: String, items_queue: Vec<String>, settings: LobbySettings) -> Element {
    let alter_setting = {
        move |setting: AlterLobbySetting| {
            alter_lobby_settings(lobby_id, player_name, setting);
        }
    };
    let mut items_queue = items_queue.clone();
    while items_queue.len() < settings.item_count {
        items_queue.push("Loading...".to_owned());
    }
    cx.render(rsx! {
        div { display: "flex", flex_direction: "column", gap: "5px",
            div {
                "Items "
                button {
                    padding: "2px",
                    padding_top: "0px",
                    background_color: "rgb(20, 100, 150)",
                    onclick: {
                        move |_| {
                            alter_setting(AlterLobbySetting::RefreshAllItems);
                        }
                    },
                    "↻"
                }
            }
            for item in items_queue {
                div { display: "flex", flex_direction: "row", gap: "5px", class: "body-box",
                    "{item}"
                    if item != "Loading..." {
                        let item1 = item.clone();
                        rsx! {
                            button {
                                padding: "2px",
                                padding_top: "0px",
                                background_color: "rgb(20, 100, 150)",
                                onclick: {
                                    move |_| {
                                        alter_setting(AlterLobbySetting::RefreshItem(item.clone()));
                                    }
                                },
                                "↻"
                            }
                            button {
                                padding: "2px",
                                padding_top: "0px",
                                background_color: "rgb(100, 20, 20)",
                                onclick: {
                                    move |_| {
                                        alter_setting(AlterLobbySetting::RemoveItem(item1.clone()));
                                    }
                                },
                                "-"
                            }
                        }
                    }
                }
            }
            form {
                onsubmit: {
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
    })
}

struct SettingDetail {
    key: String,
    display_name: String,
    min: usize,
    max: usize,
    value: usize,
}

impl SettingDetail {
    fn new(key: &str, min: usize, max: usize, value: usize) -> Self {
        Self {
            key: key.to_owned(),
            display_name: key.chars().next().unwrap().to_uppercase().to_string() + &key[1..].replace('_', " "),
            min,
            max,
            value,
        }
    }
}

#[component]
pub fn AdvancedSettings(cx: Scope, player_name: String, lobby_id: String, settings: LobbySettings) -> Element {
    let setting_details = vec![
        SettingDetail::new("starting_coins", 1, 100, settings.starting_coins),
        SettingDetail::new("coin_every_x_seconds", 1, 20, settings.coin_every_x_seconds),
        SettingDetail::new("submit_question_every_x_seconds", 1, 30, settings.submit_question_every_x_seconds),
        SettingDetail::new("add_item_every_x_questions", 1, 20, settings.add_item_every_x_questions),
        SettingDetail::new("submit_question_cost", 1, 100, settings.submit_question_cost),
        SettingDetail::new("masked_question_cost", 1, 100, settings.masked_question_cost),
        SettingDetail::new("guess_item_cost", 1, 100, settings.guess_item_cost),
        SettingDetail::new("question_min_votes", 1, 20, settings.question_min_votes),
        SettingDetail::new("score_to_coins_ratio", 1, 100, settings.score_to_coins_ratio),
    ];

    cx.render(rsx! {
        for setting in setting_details.into_iter() {
            label {
                "{setting.display_name}: "
                input {
                    r#type: "number",
                    min: "{setting.min}",
                    max: "{setting.max}",
                    value: "{setting.value}",
                    max_width: "50px",
                    oninput: {
                        move |e| {
                            alter_lobby_settings(
                                lobby_id,
                                player_name,
                                AlterLobbySetting::Advanced(
                                    setting.key.clone(),
                                    e.value.parse::<usize>().unwrap_or(1),
                                ),
                            );
                        }
                    }
                }
            }
        }
    })
}
