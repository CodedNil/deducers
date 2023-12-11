use crate::networking::DeducersMain;
use crate::{game_state::QueuedQuestion, networking::AsyncResult};
use godot::{
    engine::{Button, CheckBox, Control, Label, LineEdit, ResourceLoader, VBoxContainer},
    prelude::*,
};

impl DeducersMain {
    pub fn submit_question(&mut self) {
        if !self.connected || !self.server_started {
            self.show_management_info("Cannot submit question, game has not started yet".to_string(), 2000);
            return;
        }

        // Get question text and options
        let mut question_line = self
            .base
            .get_node_as::<LineEdit>("GameUI/HBoxContainer/VBoxContainer/Management/MarginContainer/VBoxContainer/QuestionSubmit/QuestionTextEdit");
        let question = question_line.get_text().to_string();
        let mut anonymous_checkbox = self
            .base
            .get_node_as::<CheckBox>("GameUI/HBoxContainer/VBoxContainer/Management/MarginContainer/VBoxContainer/AnonymousCheckbox");
        let anonymous = anonymous_checkbox.is_pressed();

        // Clear question text and options
        question_line.set_text("".into());
        anonymous_checkbox.set_pressed(false);

        // Check if question is empty
        if question.trim().is_empty() {
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
        let http_client_clone = self.http_client.clone();
        let tx = self.result_sender.clone();
        self.runtime.spawn(async move {
            match http_client_clone.post(&url).send().await {
                Ok(response) => {
                    if response.error_for_status_ref().is_err() {
                        godot_print!("Error submitting question: {:?}", response);
                        // Handle the error case, where the status code indicates a failure.
                        let error_message = (response.text().await).map_or_else(|_| "Error submitting question".to_string(), |custom_message| custom_message);

                        tx.lock().await.send(AsyncResult::QuestionSubmitError(error_message)).await.unwrap();
                    }
                }
                Err(_) => {
                    tx.lock()
                        .await
                        .send(AsyncResult::QuestionSubmitError("Error submitting question".into()))
                        .await
                        .unwrap();
                }
            }
        });
    }

    pub fn question_queue_vote_pressed(&mut self, button_id: u32) {
        self.play_button_pressed_sound();

        // Find the question text from button_id
        let ui_root = self.base.get_node_as::<Control>("GameUI/HBoxContainer/VBoxContainer/QuestionQueue");
        let items_container = ui_root.get_node_as::<VBoxContainer>("MarginContainer/ScrollContainer/VBoxContainer");

        let mut question_text = String::new();
        for i in 2..items_container.get_child_count() {
            let item = items_container.get_child(i).unwrap();
            let vote_button = item.get_node_as::<Button>("ColorRect3/HBoxContainer/VoteButton");
            let id = vote_button.get_meta("button_id".into()).to::<u32>();
            if id == button_id {
                question_text = item.get_node_as::<Label>("ColorRect2/QuestionLabel").get_text().to_string();
                break;
            }
        }

        if question_text.is_empty() {
            godot_print!("Failed to find question text for button id: {:?}", button_id);
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

        let http_client_clone = self.http_client.clone();
        self.runtime.spawn(async move {
            match http_client_clone.post(&url).send().await {
                Ok(_) => {}
                Err(error) => {
                    godot_print!("Error voting for question {error}");
                }
            }
        });
    }

    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
    pub fn questions_queue_update(&mut self, questions_queue: &Vec<QueuedQuestion>) {
        let ui_root = self.base.get_node_as::<Control>("GameUI/HBoxContainer/VBoxContainer/QuestionQueue");
        let mut items_container = ui_root.get_node_as::<VBoxContainer>("MarginContainer/ScrollContainer/VBoxContainer");

        // Add new items if needed
        let children_to_add = (questions_queue.len() + 2) as i32 - items_container.get_child_count();
        if children_to_add > 0 {
            for _ in 0..children_to_add {
                let item_scene = ResourceLoader::singleton()
                    .load("res://QuestionQueueItem.tscn".into())
                    .unwrap()
                    .cast::<PackedScene>();
                let new_item = item_scene.instantiate().unwrap();

                // Set vote button callback
                let mut vote_button = new_item.get_node_as::<Button>("ColorRect3/HBoxContainer/VoteButton");
                // Generate random id for button
                let button_id = rand::random::<u32>();
                vote_button.set_meta("button_id".into(), button_id.to_variant());
                let tx = self.result_sender.clone();
                let runtime = self.runtime.handle().clone();
                vote_button.connect(
                    "pressed".into(),
                    Callable::from_fn("question_vote_pressed", move |_| {
                        let tx_clone = tx.clone();
                        runtime.spawn(async move {
                            tx_clone.lock().await.send(AsyncResult::QuestionVoted(button_id)).await.unwrap();
                        });
                        Ok(Variant::nil())
                    }),
                );

                items_container.add_child(new_item);
            }
        }

        // Remove excess items if needed
        let children_to_remove = items_container.get_child_count() - (questions_queue.len() + 2) as i32;
        if children_to_remove > 0 {
            for _ in 0..children_to_remove {
                items_container.get_child(items_container.get_child_count() - 1).unwrap().queue_free();
            }
        }

        // Create sorted vector of questions by votes
        let mut questions_queue_sorted = questions_queue.clone();
        questions_queue_sorted.sort_by(|a, b| b.votes.cmp(&a.votes));

        // Update queue items
        let mut index = 2;
        for question in questions_queue_sorted {
            let question_text = question.question.clone().unwrap_or_else(|| "ANONYMOUS".into());

            let item = items_container.get_child(index).unwrap();
            item.get_node_as::<Label>("ColorRect1/PlayerLabel").set_text(question.player.clone().into());
            item.get_node_as::<Label>("ColorRect2/QuestionLabel").set_text(question_text.into());
            item.get_node_as::<Label>("ColorRect3/HBoxContainer/VotesLabel")
                .set_text(question.votes.to_string().into());

            index += 1;
        }
    }
}
