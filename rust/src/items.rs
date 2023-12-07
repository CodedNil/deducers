use crate::game_state::Item;
use godot::{
    engine::{Control, HBoxContainer, Label, ResourceLoader, VBoxContainer},
    prelude::*,
};

#[allow(clippy::cast_possible_wrap, clippy::cast_possible_truncation)]
pub fn update(ui_root: &Gd<Control>, items: &Vec<Item>) {
    let mut header_container = ui_root
        .get_node_as::<HBoxContainer>("MarginContainer/ScrollContainer/VBoxContainer/Header");

    // Add new items if needed
    let children_to_add = (items.len() + 1) as i32 - header_container.get_child_count();
    if children_to_add > 0 {
        for _ in 0..children_to_add {
            let item_scene = ResourceLoader::singleton()
                .load("res://Items/ItemsHeader.tscn".into())
                .unwrap()
                .cast::<PackedScene>();
            let new_item = item_scene.instantiate().unwrap();
            header_container.add_child(new_item);
        }
    }

    // Remove excess items if needed
    let children_to_remove = header_container.get_child_count() - (items.len() + 1) as i32;
    if children_to_remove > 0 {
        for _ in 0..children_to_remove {
            header_container
                .get_child(header_container.get_child_count() - 1)
                .unwrap()
                .queue_free();
        }
    }

    // Update headers
    let mut index = 1;
    for item in items {
        let header_node = header_container.get_child(index).unwrap();
        header_node
            .get_node_as::<Label>("Label")
            .set_text(item.id.to_string().into());

        index += 1;
    }

    let mut items_container =
        ui_root.get_node_as::<VBoxContainer>("MarginContainer/ScrollContainer/VBoxContainer");
}
