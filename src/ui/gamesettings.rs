use crate::{
    lobby_utils::{alter_lobby_settings, AlterLobbySetting, Difficulty, Lobby},
    MAX_LOBBY_ITEMS,
};
use dioxus::prelude::*;

#[allow(clippy::too_many_lines, clippy::cast_possible_wrap)]
pub fn render<'a>(
    cx: Scope<'a>,
    lobby_settings_open: &'a UseState<bool>,
    player_name: &'a str,
    lobby_id: &'a str,
    lobby: &Lobby,
) -> Element<'a> {
    let difficulty = lobby.settings.difficulty.clone();
    let item_count = lobby.settings.item_count.to_string();
    let player_controlled = lobby.settings.player_controlled.to_string();
    let server_items = lobby.items.clone();

    cx.render(rsx! {
        div { class: "dialog floating", display: "flex", gap: "20px", top: if *lobby_settings_open.get() { "50%" } else { "-100%" },
            "Lobby Settings"
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
            label {
                "Item Count: "
                input {
                    r#type: "number",
                    min: "1",
                    max: MAX_LOBBY_ITEMS as i64,
                    value: "{item_count}",
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
            }
            label {
                "Player Controlled: "
                input { r#type: "checkbox", checked: "{player_controlled}",
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
            button {
                onclick: move |_| {
                    lobby_settings_open.set(!lobby_settings_open.get());
                },
                "Close"
            }
        }
    })
}
