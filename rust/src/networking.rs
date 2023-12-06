use crate::leaderboard;
use chrono::{DateTime, Utc};
use godot::{
    engine::{ColorRect, Control, IControl, Label, LineEdit},
    prelude::*,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, time::Duration};

#[derive(GodotClass)]
#[class(base=Control)]
struct DeducersMain {
    #[base]
    base: Base<Control>,
    http_client: ureq::Agent,
    server_ip: String,
    player_name: String,
    room_name: String,
    connected: bool,
    is_host: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Server {
    id: String,
    started: bool,
    key_player: String,
    players: HashMap<String, Player>,
    questions_queue: Vec<QueuedQuestion>,
    items: Vec<Item>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Player {
    pub name: String,
    last_contact: DateTime<Utc>,
    pub score: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct QueuedQuestion {
    player: String,
    question: String,
    votes: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Item {
    name: String,
    questions: Vec<Question>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Question {
    player: String,
    question: String,
    answer: Answer,
    anonymous: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
enum Answer {
    Yes,
    No,
    Sometimes,
    Depends,
    Irrelevant,
}

#[derive(Deserialize)]
enum GameStateResponse {
    ServerState(Server),
    Error(String),
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
                match serde_json::from_str::<GameStateResponse>(
                    &response.into_string().unwrap_or_default(),
                ) {
                    Ok(GameStateResponse::ServerState(server)) => {
                        godot_print!("Server data: {:?}", server);

                        // Set fields
                        self.server_ip = server_ip_text;
                        self.player_name = player_name_text;
                        self.room_name = room_name_text;
                        self.connected = true;
                        if server.key_player == self.player_name {
                            self.is_host = true;
                        }
                        self.process_join_server(&server);
                        self.process_game_state(&server);
                    }
                    Ok(GameStateResponse::Error(err_msg)) => {
                        godot_print!("Error in game state response: {:?}", err_msg);
                    }
                    Err(e) => {
                        godot_print!("Failed to parse response, error: {:?}", e);
                    }
                }
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

    fn show_alert(&mut self, message: String) {
        self.base
            .get_node_as::<Label>("AlertDialog/MarginContainer/VBoxContainer/Label")
            .set_text(message.into());
        self.base.get_node_as::<ColorRect>("AlertDialog").show();
    }

    #[func]
    fn on_error_dialog_ok_pressed(&mut self) {
        self.base.get_node_as::<ColorRect>("AlertDialog").hide();
    }

    #[func]
    fn on_start_server_pressed(&mut self) {}

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

    fn process_join_server(&mut self, server: &Server) {
        // Hide connect ui
        self.base.get_node_as::<Control>("ConnectUI").hide();

        // Set lobby id
        self.base
            .get_node_as::<Label>("GameUI/HBoxContainer/VBoxContainer/Leaderboard/LobbyStatus/MarginContainer/HBoxContainer/LobbyId")
            .set_text(format!("Lobby ID: {}", self.room_name.clone()).into());

        // Set start button visibility
        self.base
            .get_node_as::<Control>("GameUI/HBoxContainer/VBoxContainer/Leaderboard/LobbyStatus/MarginContainer/HBoxContainer/StartButton")
            .set_visible(self.is_host && !server.started);
    }

    fn process_game_state(&mut self, server: &Server) {
        leaderboard::update(
            &self
                .base
                .get_node_as::<Control>("GameUI/HBoxContainer/VBoxContainer/Leaderboard"),
            &server.players,
            &self.player_name,
            self.is_host,
        );
    }
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
            is_host: false,
        }
    }

    fn ready(&mut self) {
        // Show connect ui
        self.base.get_node_as::<Control>("ConnectUI").show();
    }
}