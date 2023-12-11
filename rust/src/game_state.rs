use crate::{
    items,
    networking::{AsyncResult, DeducersMain, SUBMIT_QUESTION_EVERY_X_SECONDS},
};
use godot::{
    engine::{Control, Label},
    prelude::*,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::time::Instant;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServerMinimal {
    id: String,
    started: bool,
    elapsed_time: f64,
    key_player: String,
    players: HashMap<String, Player>,
    questions_queue: Vec<QueuedQuestion>,
    items: Vec<Item>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Player {
    pub name: String,
    pub score: usize,
    coins: Option<usize>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QueuedQuestion {
    pub player: String,
    pub question: Option<String>,
    pub anonymous: bool,
    pub votes: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Item {
    pub id: usize,
    pub questions: Vec<Question>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Question {
    pub player: String,
    pub id: usize,
    pub question: Option<String>,
    pub answer: Answer,
    pub anonymous: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Answer {
    Yes,
    No,
    Maybe,
}

impl Answer {
    pub const fn to_color(&self) -> Color {
        match self {
            Self::Yes => Color::from_rgb(0.25, 0.5, 0.2),
            Self::No => Color::from_rgb(0.5, 0.2, 0.2),
            Self::Maybe => Color::from_rgb(0.55, 0.35, 0.0),
        }
    }
}

#[derive(Deserialize)]
enum GameStateResponse {
    ServerState(ServerMinimal),
    Error(String),
}

impl DeducersMain {
    #[allow(clippy::cast_possible_truncation)]
    pub fn refresh_game_state(&mut self) {
        // Record the current time before sending the request
        let start_time = Instant::now();

        // Make get request to get game state
        let url = format!(
            "http://{server_ip}/server/{room_name}/getstate/{player_name}",
            server_ip = self.server_ip,
            room_name = self.room_name,
            player_name = self.player_name
        );
        let http_client_clone = self.http_client.clone();
        let tx = self.result_sender.clone();
        self.runtime.spawn(async move {
            match http_client_clone.get(&url).send().await {
                Ok(response) => {
                    // Calculate the round-trip time (ping)
                    let ping = start_time.elapsed().as_millis() as i64;
                    let Ok(response_str) = response.text().await else {
                        tx.lock()
                            .await
                            .send(AsyncResult::RefreshGameStateError("Error getting game state".to_string()))
                            .await
                            .unwrap();
                        return;
                    };
                    tx.lock().await.send(AsyncResult::RefreshGameState(response_str, ping)).await.unwrap();
                }
                Err(error) => {
                    let error_message = error.status().map_or_else(
                        || format!("Error getting game state {error}"),
                        |status| format!("Error getting game state {status}"),
                    );

                    tx.lock().await.send(AsyncResult::RefreshGameStateError(error_message)).await.unwrap();
                }
            }
        });
    }

    #[allow(clippy::cast_precision_loss)]
    pub fn refresh_game_state_received(&mut self, response_str: &str, ping: i64) {
        self.base
            .get_node_as::<Label>("GameUI/HBoxContainer/VBoxContainer/Leaderboard/LobbyStatus/MarginContainer/HBoxContainer/Ping")
            .set_text(format!("Ping: {ping}ms").into());

        match serde_json::from_str::<GameStateResponse>(response_str) {
            Ok(GameStateResponse::ServerState(server)) => {
                self.process_game_state(&server);
            }
            _ => {
                godot_print!("Failed to parse game state");
            }
        }
    }

    pub fn process_join_server(&mut self, response: &str, server_ip_text: String, room_name_text: String, player_name_text: String) {
        match serde_json::from_str::<GameStateResponse>(response) {
            Ok(GameStateResponse::ServerState(server)) => {
                // Set fields
                self.server_ip = server_ip_text;
                self.player_name = player_name_text;
                self.room_name = room_name_text;
                self.connected = true;
                if server.key_player == self.player_name {
                    self.is_host = true;
                }

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

    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    fn process_game_state(&mut self, server: &ServerMinimal) {
        self.update_leaderboard(&server.players, &self.player_name.clone(), self.is_host);
        items::set_guess_list(
            &self.base.get_node_as::<Control>("GameUI/HBoxContainer/VBoxContainer/Management"),
            &server.items,
        );
        items::update(&self.base.get_node_as::<Control>("GameUI/HBoxContainer/Items"), &server.items);

        self.questions_queue_update(&server.questions_queue);

        // Set start button visibility
        self.base
            .get_node_as::<Control>("GameUI/HBoxContainer/VBoxContainer/Leaderboard/LobbyStatus/MarginContainer/HBoxContainer/StartButton")
            .set_visible(self.is_host && !server.started);

        // Set time label
        let elapsed_seconds = server.elapsed_time;
        println!("Elapsed seconds: {}", server.elapsed_time);
        self.base
            .get_node_as::<Label>("GameUI/HBoxContainer/VBoxContainer/Leaderboard/LobbyStatus/MarginContainer/HBoxContainer/Time")
            .set_text(format!("Time: {}s", elapsed_seconds as usize).into());
        self.server_started = server.started;

        // Countdown until question submitted every x seconds
        let remaining_time = (SUBMIT_QUESTION_EVERY_X_SECONDS - (elapsed_seconds % SUBMIT_QUESTION_EVERY_X_SECONDS)).round() as usize;
        self.base
            .get_node_as::<Label>("GameUI/HBoxContainer/VBoxContainer/QuestionQueue/MarginContainer/ScrollContainer/VBoxContainer/Label")
            .set_text(format!("Top Question Submitted In {remaining_time} Seconds").into());

        // Set coins available label
        let coins = server.players.get(&self.player_name).unwrap().coins.unwrap_or_default();
        self.base
            .get_node_as::<Label>("GameUI/HBoxContainer/VBoxContainer/Management/MarginContainer/VBoxContainer/CoinsRow/CoinsLabel")
            .set_text(format!("{coins} Coins Available").into());
    }
}
