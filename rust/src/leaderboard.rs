use crate::{
    game_state::Player,
    networking::{AsyncResult, DeducersMain},
};
use godot::{
    engine::{Button, Control, Label, ResourceLoader},
    prelude::*,
};
use std::collections::HashMap;

impl DeducersMain {
    #[allow(clippy::cast_possible_wrap, clippy::cast_possible_truncation)]
    pub fn update_leaderboard(&mut self, players: &HashMap<String, Player>, player_name: &String, is_host: bool) {
        let ui_root = self.base.get_node_as::<Control>("GameUI/HBoxContainer/VBoxContainer/Leaderboard");
        let mut items_container = ui_root.get_node_as::<Control>("LeaderboardColorRect/MarginContainer/ScrollContainer/VBoxContainer");

        // Add new items if needed
        let children_to_add = (players.len() + 1) as i32 - items_container.get_child_count();
        if children_to_add > 0 {
            for _ in 0..children_to_add {
                let item_scene = ResourceLoader::singleton()
                    .load("res://LeaderboardItem.tscn".into())
                    .unwrap()
                    .cast::<PackedScene>();
                let new_item = item_scene.instantiate().unwrap();

                // Set kick button callback
                let mut kick_button = new_item.get_node_as::<Button>("ColorScore/HBoxContainer/KickButton");
                // Generate random id for button
                let button_id = rand::random::<u32>();
                kick_button.set_meta("button_id".into(), button_id.to_variant());
                let tx = self.result_sender.clone();
                let runtime = self.runtime.handle().clone();
                kick_button.connect(
                    "pressed".into(),
                    Callable::from_fn("kick_pressed", move |_| {
                        let tx_clone = tx.clone();
                        runtime.spawn(async move {
                            tx_clone.lock().await.send(AsyncResult::KickPlayer(button_id)).await.unwrap();
                        });
                        Ok(Variant::nil())
                    }),
                );

                items_container.add_child(new_item);
            }
        }

        // Remove excess items if needed
        let children_to_remove = items_container.get_child_count() - (players.len() + 1) as i32;
        if children_to_remove > 0 {
            for _ in 0..children_to_remove {
                items_container.get_child(items_container.get_child_count() - 1).unwrap().queue_free();
            }
        }

        // Update leaderboard items with player scores
        for (index, player) in players.values().enumerate() {
            let item = items_container.get_child(index as i32 + 1).unwrap();
            item.get_node_as::<Label>("ColorName/PlayerName").set_text(player.name.clone().into());
            item.get_node_as::<Label>("ColorScore/HBoxContainer/PlayerScore")
                .set_text(player.score.to_string().into());

            // Show kick button if host and not self
            item.get_node_as::<Button>("ColorScore/HBoxContainer/KickButton")
                .set_visible(is_host && (&player.name != player_name));
        }
    }

    pub fn kick_player_pressed(&mut self, button_id: u32) {
        // Find the player name from button_id
        let ui_root = self.base.get_node_as::<Control>("GameUI/HBoxContainer/VBoxContainer/Leaderboard");
        let items_container = ui_root.get_node_as::<Control>("LeaderboardColorRect/MarginContainer/ScrollContainer/VBoxContainer");

        let mut player_name = String::new();
        for i in 2..items_container.get_child_count() {
            let item = items_container.get_child(i).unwrap();
            let kick_button = item.get_node_as::<Button>("ColorScore/HBoxContainer/KickButton");
            let id = kick_button.get_meta("button_id".into()).to::<u32>();
            if id == button_id {
                player_name = item.get_node_as::<Label>("ColorName/PlayerName").get_text().to_string();
                break;
            }
        }

        if player_name.is_empty() {
            godot_print!("Failed to find player name for button id: {:?}", button_id);
            return;
        }

        // Construct the URL for the post request
        let url = format!(
            "http://{server_ip}/server/{room_name}/kickplayer/{player_name}/{kick_player}",
            server_ip = self.server_ip,
            room_name = self.room_name,
            player_name = self.player_name,
            kick_player = player_name
        );

        let http_client_clone = self.http_client.clone();
        self.runtime.spawn(async move {
            match http_client_clone.post(&url).send().await {
                Ok(_) => {}
                Err(error) => {
                    godot_print!("Error kicking player {error}");
                }
            }
        });
    }
}
