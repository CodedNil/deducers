use crate::game_state::Item;
use godot::{
    engine::{ColorRect, Control, Label, OptionButton, ResourceLoader},
    prelude::*,
};

#[allow(
    clippy::cast_possible_wrap,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
pub fn set_guess_list(ui_root: &Gd<Control>, items: &Vec<Item>) {
    let mut guess_item_choice =
        ui_root.get_node_as::<OptionButton>("MarginContainer/VBoxContainer/GuessItem/ItemChoice");

    // Add or remove items as needed
    let item_count = guess_item_choice.get_item_count();
    match items.len().cmp(&(item_count as usize)) {
        std::cmp::Ordering::Less => {
            for _ in items.len() as i32..item_count {
                guess_item_choice.remove_item(items.len() as i32);
            }
        }
        std::cmp::Ordering::Greater => {
            for index in item_count..items.len() as i32 {
                guess_item_choice.add_item(items[index as usize].id.to_string().into());
            }
        }
        std::cmp::Ordering::Equal => {}
    }
    // Set item names
    for (index, item) in items.iter().enumerate() {
        guess_item_choice.set_item_text(index as i32, item.id.to_string().into());
    }
}

#[allow(
    clippy::cast_possible_wrap,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
pub fn update(ui_root: &Gd<Control>, items: &Vec<Item>) {
    let mut header_container =
        ui_root.get_node_as::<Control>("MarginContainer/ScrollContainer/VBoxContainer/Header");

    // Manage header items, add and remove as needed
    manage_items(
        &mut header_container,
        items.len() as i32 + 1,
        "res://Items/ItemsHeader.tscn",
    );

    // Update headers
    for (index, item) in items.iter().enumerate() {
        let header_node = header_container.get_child(index as i32 + 1).unwrap();
        header_node
            .get_node_as::<Label>("Label")
            .set_text(item.id.to_string().into());
    }

    let items_container =
        ui_root.get_node_as::<Control>("MarginContainer/ScrollContainer/VBoxContainer");

    // Get list of questions that are active
    let mut active_questions = vec![];
    for item in items {
        for question in &item.questions {
            if !active_questions.contains(&(question.id, question.question.clone())) {
                active_questions.push((question.id, question.question.clone()));
            }
        }
    }

    // Update items with question text
    for question_index in 0..20 {
        let num_blanks = 20 - active_questions.len() as i32;

        let mut child = items_container
            .get_child(question_index + 1)
            .unwrap()
            .cast::<Control>();

        // Set question id and text
        child
            .get_node_as::<Label>("ColorRect/MarginContainer/HBoxContainer/Index")
            .set_text(
                if question_index < 20 - num_blanks {
                    format!("{}: ", question_index + 1)
                } else {
                    String::new()
                }
                .into(),
            );
        let question_string = if question_index < 20 - num_blanks {
            active_questions
                .get(question_index as usize)
                .unwrap()
                .1
                .clone()
                .unwrap_or("ANONYMOUS".into())
        } else {
            String::new()
        };
        let question_id = if question_index < 20 - num_blanks {
            active_questions.get(question_index as usize).unwrap().0
        } else {
            0
        };
        child
            .get_node_as::<Label>("ColorRect/MarginContainer/HBoxContainer/Question")
            .set_text(question_string.clone().into());

        // Make the right number of answer boxes available
        manage_items(
            &mut child,
            items.len() as i32 + 1,
            "res://Items/ItemsAnswerBox.tscn",
        );

        // Colour the answer boxes
        for (item_index, item) in items.iter().enumerate() {
            // Get answer if it exists
            let mut answer: Option<crate::game_state::Answer> = None;
            if question_index < 20 - num_blanks {
                for answer_question in &item.questions {
                    if answer_question.id == question_id {
                        answer = Some(answer_question.answer.clone());
                        break;
                    }
                }
            }

            let mut color_rect = child
                .get_child(item_index as i32 + 1)
                .unwrap()
                .cast::<ColorRect>();
            let mut star_image = color_rect.get_child(0).unwrap().cast::<Control>();
            if let Some(answer) = answer {
                color_rect.set_color(answer.to_color());
                star_image.set_visible(false);
            } else {
                color_rect.set_color(Color::from_rgb(0.2, 0.2, 0.2));
                star_image.set_visible(true);
            }
        }
    }
}

fn manage_items(container: &mut Gd<Control>, count: i32, item_scene_path: &str) {
    // Add new items if needed
    let children_to_add = count - container.get_child_count();
    if children_to_add > 0 {
        for _ in 0..children_to_add {
            let item_scene = ResourceLoader::singleton()
                .load(item_scene_path.into())
                .unwrap()
                .cast::<PackedScene>();
            let new_item = item_scene.instantiate().unwrap();
            container.add_child(new_item);
        }
    }

    // Remove excess items if needed
    let children_to_remove = container.get_child_count() - count;
    if children_to_remove > 0 {
        for _ in 0..children_to_remove {
            container
                .get_child(container.get_child_count() - 1)
                .unwrap()
                .queue_free();
        }
    }
}
