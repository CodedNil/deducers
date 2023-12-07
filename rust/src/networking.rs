use chrono::{DateTime, Utc};
use godot::{
    engine::{Button, ColorRect, Control, IControl, Label, LineEdit},
    prelude::*,
};
use std::time::Duration;

const UPDATE_TIME: f64 = 0.5;

#[derive(GodotClass)]
#[class(base=Control)]
pub struct DeducersMain {
    #[base]
    pub base: Base<Control>,
    pub http_client: ureq::Agent,
    pub server_ip: String,
    pub player_name: String,
    pub room_name: String,
    pub connected: bool,
    pub server_started: bool,
    pub is_host: bool,
    time_since_update: f64,
    management_info_text_clear_time: Option<DateTime<Utc>>,
}

#[godot_api]
impl DeducersMain {
    #[func]
    fn on_connect_button_pressed(&mut self) {
        // Get nodes
        let server_ip_text = self
            .base
            .get_node_as::<LineEdit>("ConnectUI/ColorRect/VBoxContainer/ServerIp")
            .get_text()
            .to_string();
        let room_name_text = self
            .base
            .get_node_as::<LineEdit>("ConnectUI/ColorRect/VBoxContainer/HBoxContainer/RoomName")
            .get_text()
            .to_string();
        let player_name_text = self
            .base
            .get_node_as::<LineEdit>("ConnectUI/ColorRect/VBoxContainer/HBoxContainer/PlayerName")
            .get_text()
            .to_string();

        // Make post request to connect
        let url =
            format!("http://{server_ip_text}/server/{room_name_text}/connect/{player_name_text}");
        let result = self.http_client.post(&url).call();

        match result {
            Ok(response) => {
                self.process_join_server(
                    &response.into_string().unwrap_or_default(),
                    server_ip_text,
                    room_name_text,
                    player_name_text,
                );
            }
            Err(error) => {
                let error_message = if let ureq::Error::Status(_, response) = error {
                    response
                        .into_string()
                        .unwrap_or_else(|_| "Failed to read error message".to_string())
                } else {
                    format!("Connection error: {error}")
                };

                godot_print!("Error: {error_message}");
                self.show_alert(
                    format!("Could not connect to server:\n{error_message}").to_string(),
                );
            }
        }
    }

    pub fn show_alert(&mut self, message: String) {
        self.base
            .get_node_as::<Label>("AlertDialog/MarginContainer/VBoxContainer/Label")
            .set_text(message.into());
        self.base.get_node_as::<ColorRect>("AlertDialog").show();
    }

    pub fn show_management_info(&mut self, message: String, duration: i64) {
        // Set message text, then wait duration and if message text is still the same, clear it
        self.base
            .get_node_as::<Label>("GameUI/HBoxContainer/VBoxContainer/Management/MarginContainer/VBoxContainer/ManagementInfoLabel").set_text(message.into());
        // Set mana
        self.management_info_text_clear_time =
            Some(Utc::now() + chrono::Duration::milliseconds(duration));
    }

    #[func]
    fn on_error_dialog_ok_pressed(&mut self) {
        self.base.get_node_as::<ColorRect>("AlertDialog").hide();
    }

    #[func]
    fn on_start_server_pressed(&mut self) {
        let url = format!(
            "http://{server_ip}/server/{room_name}/start/{player_name}",
            server_ip = self.server_ip,
            room_name = self.room_name,
            player_name = self.player_name
        );
        match self.http_client.post(&url).call() {
            Ok(_) => {
                self.base
                    .get_node_as::<Button>("GameUI/HBoxContainer/VBoxContainer/Leaderboard/LobbyStatus/MarginContainer/HBoxContainer/StartButton")
                    .hide();
            }
            Err(error) => {
                godot_print!("Error starting server {error}");
            }
        }
    }

    #[func]
    fn on_leave_server_pressed(&mut self) {
        // Make post request to disconnect
        let url = format!(
            "http://{server_ip}/server/{room_name}/disconnect/{player_name}",
            server_ip = self.server_ip,
            room_name = self.room_name,
            player_name = self.player_name
        );
        match self.http_client.post(&url).call() {
            Ok(_) => {}
            Err(error) => {
                godot_print!("Error disconnecting from server {error}");
            }
        }

        // Show connect ui
        self.base.get_node_as::<Control>("ConnectUI").show();

        self.connected = false;
    }

    #[func]
    fn on_submit_question_pressed(&mut self) {
        self.submit_question();
    }

    #[func]
    fn on_convert_score_pressed(&mut self) {}
}

#[godot_api]
impl IControl for DeducersMain {
    fn init(base: Base<Control>) -> Self {
        Self {
            base,
            http_client: ureq::builder()
                .timeout_connect(Duration::from_secs(5))
                .build(),
            server_ip: String::new(),
            player_name: String::new(),
            room_name: String::new(),
            connected: false,
            server_started: false,
            is_host: false,
            time_since_update: 0.0,
            management_info_text_clear_time: None,
        }
    }

    fn ready(&mut self) {
        // Show connect ui
        self.base.get_node_as::<Control>("ConnectUI").show();
    }

    fn process(&mut self, delta: f64) {
        if self.connected {
            self.time_since_update += delta;
            if self.time_since_update >= UPDATE_TIME {
                self.time_since_update = 0.0;

                self.refresh_game_state();
            }
        }
        // Clear management info text if it's time
        if let Some(clear_time) = self.management_info_text_clear_time {
            if clear_time < Utc::now() {
                self.base
                    .get_node_as::<Label>("GameUI/HBoxContainer/VBoxContainer/Management/MarginContainer/VBoxContainer/ManagementInfoLabel")
                    .set_text("".into());
                self.management_info_text_clear_time = None;
            }
        }
    }
}
