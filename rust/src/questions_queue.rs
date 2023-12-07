use crate::game_state::QueuedQuestion;
use crate::networking::DeducersMain;
use godot::{
    engine::{Button, CheckBox, Control, Label, LineEdit, ResourceLoader, VBoxContainer},
    prelude::*,
};

impl DeducersMain {
    pub fn submit_question(&mut self) {
        if !self.connected || !self.server_started {
            self.show_management_info(
                "Cannot submit question, game has not started yet".to_string(),
                2000,
            );
            return;
        }

        // Get question text and options
        let question = self
        .base
        .get_node_as::<LineEdit>("GameUI/HBoxContainer/VBoxContainer/Management/MarginContainer/VBoxContainer/QuestionSubmit/QuestionTextEdit")
        .get_text()
        .to_string();
        let anonymous = self
        .base
        .get_node_as::<CheckBox>("GameUI/HBoxContainer/VBoxContainer/Management/MarginContainer/VBoxContainer/AnonymouseCheckbox").is_pressed();

        // Check if question is empty
        if question.is_empty() {
            self.show_management_info("Question cannot be empty".to_string(), 2000);
            return;
        }

        // Serialize the options into JSON, and url encode it
        let encoded_question = urlencoding::encode(&question);
        let options = serde_json::json!({ "anonymous": anonymous }).to_string();
        let encoded_options = urlencoding::encode(&options);

        // Construct the URL for the post request
        let url = format!(
            "http://{server_ip}/server/{room_name}/submitquestion/{player_name}/{question}/{options}",
            server_ip = self.server_ip,
            room_name = self.room_name,
            player_name = self.player_name,
            question = encoded_question,
            options = encoded_options
        );

        match self.http_client.post(&url).call() {
            Ok(result) => {
                // Clear question text
                self.base
                .get_node_as::<LineEdit>("GameUI/HBoxContainer/VBoxContainer/Management/MarginContainer/VBoxContainer/QuestionSubmit/QuestionTextEdit")
                .set_text("".into());
                // Untick anonymous checkbox
                self.base
                .get_node_as::<CheckBox>("GameUI/HBoxContainer/VBoxContainer/Management/MarginContainer/VBoxContainer/AnonymouseCheckbox")
                .set_pressed(false);

                // Print result
                godot_print!("Result: {:?}", result.into_string());
            }
            Err(error) => {
                if let ureq::Error::Status(_, response) = error {
                    if let Ok(text) = response.into_string() {
                        self.show_management_info(text, 2000);
                    } else {
                        godot_print!("Error submitting question");
                    }
                } else {
                    godot_print!("Error submitting question: {error}");
                };
            }
        }
    }

    pub fn question_queue_vote_clicked(&mut self, button_id: u32) {
        // Find the question text from button_id
        let ui_root = self
            .base
            .get_node_as::<Control>("GameUI/HBoxContainer/VBoxContainer/QuestionQueue");
        let items_container =
            ui_root.get_node_as::<VBoxContainer>("MarginContainer/ScrollContainer/VBoxContainer");

        let mut question_text = String::new();
        for i in 2..items_container.get_child_count() {
            let item = items_container.get_child(i).unwrap();
            let vote_button = item.get_node_as::<Button>("ColorRect3/HBoxContainer/VoteButton");
            let id = vote_button.get_meta("button_id".into()).to::<u32>();
            if id == button_id {
                question_text = item
                    .get_node_as::<Label>("ColorRect2/QuestionLabel")
                    .get_text()
                    .to_string();
                break;
            }
        }

        if question_text.is_empty() {
            godot_print!(
                "Failed to find question text for button id: {:?}",
                button_id
            );
            return;
        }

        // Construct the URL for the post request
        let encoded_question = urlencoding::encode(&question_text);
        let url = format!(
            "http://{server_ip}/server/{room_name}/votequestion/{player_name}/{question}",
            server_ip = self.server_ip,
            room_name = self.room_name,
            player_name = self.player_name,
            question = encoded_question,
        );

        match self.http_client.post(&url).call() {
            Ok(result) => {
                godot_print!("Voted for question: {:?}", result.into_string());
            }
            Err(error) => {
                godot_print!("Error voting for question: {error}");
            }
        }
    }

    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
    pub fn questions_queue_update(&mut self, questions_queue: &Vec<QueuedQuestion>) {
        let ui_root = self
            .base
            .get_node_as::<Control>("GameUI/HBoxContainer/VBoxContainer/QuestionQueue");
        let mut items_container =
            ui_root.get_node_as::<VBoxContainer>("MarginContainer/ScrollContainer/VBoxContainer");

        // Add new items if needed
        let children_to_add =
            (questions_queue.len() + 2) as i32 - items_container.get_child_count();
        if children_to_add > 0 {
            for _ in 0..children_to_add {
                let item_scene = ResourceLoader::singleton()
                    .load("res://QuestionQueueItem.tscn".into())
                    .unwrap()
                    .cast::<PackedScene>();
                let new_item = item_scene.instantiate().unwrap();

                // Set vote button callback
                let mut vote_button =
                    new_item.get_node_as::<Button>("ColorRect3/HBoxContainer/VoteButton");
                // Generate random id for button
                let button_id = rand::random::<u32>();
                vote_button.set_meta("button_id".into(), button_id.to_variant());

                items_container.add_child(new_item);
            }
        }

        // Remove excess items if needed
        let children_to_remove =
            items_container.get_child_count() - (questions_queue.len() + 2) as i32;
        if children_to_remove > 0 {
            for _ in 0..children_to_remove {
                items_container
                    .get_child(items_container.get_child_count() - 1)
                    .unwrap()
                    .queue_free();
            }
        }

        // Create sorted vector of questions by votes
        let mut questions_queue_sorted = questions_queue.clone();
        questions_queue_sorted.sort_by(|a, b| b.votes.cmp(&a.votes));

        // Update queue items
        let mut index = 2;
        for question in questions_queue_sorted {
            let item = items_container.get_child(index).unwrap();
            item.get_node_as::<Label>("ColorRect1/PlayerLabel")
                .set_text(question.player.clone().into());
            item.get_node_as::<Label>("ColorRect2/QuestionLabel")
                .set_text(question.question.clone().into());
            item.get_node_as::<Label>("ColorRect3/HBoxContainer/VotesLabel")
                .set_text(question.votes.to_string().into());

            index += 1;
        }
    }
}
