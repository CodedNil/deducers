use crate::game_state::Player;
use godot::{
    engine::{Button, Control, Label, ResourceLoader, VBoxContainer},
    prelude::*,
};
use std::collections::HashMap;

#[allow(clippy::cast_possible_wrap, clippy::cast_possible_truncation)]
pub fn update(
    ui_root: &Gd<Control>,
    players: &HashMap<String, Player>,
    player_name: &String,
    is_host: bool,
) {
    let mut items_container = ui_root.get_node_as::<VBoxContainer>(
        "LeaderboardColorRect/MarginContainer/ScrollContainer/VBoxContainer",
    );

    // Add new items if needed
    let children_to_add = (players.len() + 1) as i32 - items_container.get_child_count();
    if children_to_add > 0 {
        for _ in 0..children_to_add {
            let item_scene = ResourceLoader::singleton()
                .load("res://LeaderboardItem.tscn".into())
                .unwrap()
                .cast::<PackedScene>();
            let new_item = item_scene.instantiate().unwrap();
            items_container.add_child(new_item);
        }
    }

    // Remove excess items if needed
    let children_to_remove = items_container.get_child_count() - (players.len() + 1) as i32;
    if children_to_remove > 0 {
        for _ in 0..children_to_remove {
            items_container
                .get_child(items_container.get_child_count() - 1)
                .unwrap()
                .queue_free();
        }
    }

    // Update leaderboard items with player scores
    let mut index = 1;
    for player in players.values() {
        let item = items_container.get_child(index).unwrap();
        item.get_node_as::<Label>("ColorName/PlayerName")
            .set_text(player.name.clone().into());
        item.get_node_as::<Label>("ColorScore/HBoxContainer/PlayerScore")
            .set_text(player.score.to_string().into());

        // Show kick button if host and not self
        item.get_node_as::<Button>("ColorScore/HBoxContainer/KickButton")
            .set_visible(is_host && (&player.name != player_name));

        index += 1;
    }
}
