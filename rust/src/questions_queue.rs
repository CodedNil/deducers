use crate::networking::DeducersMain;
use godot::{
    engine::{CheckBox, LineEdit},
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
}
