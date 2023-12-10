use chrono::{DateTime, Utc};
use godot::{
    engine::{ColorRect, Control, IControl, Label, LineEdit},
    prelude::*,
};
use std::{sync::Arc, time::Duration};
use tokio::sync::{mpsc, Mutex};

pub enum AsyncResult {
    ProcessJoinServer(String, String, String, String),
    ProcessJoinServerError(String),
    QuestionSubmitted,
    QuestionSubmitError(String),
    QuestionVoted(u32),
    RefreshGameState(String, i64),
    RefreshGameStateError(String),
    KickPlayer(u32),
}

#[derive(GodotClass)]
#[class(base=Control)]
pub struct DeducersMain {
    #[base]
    pub base: Base<Control>,
    pub runtime: tokio::runtime::Runtime,
    pub http_client: reqwest::Client,
    pub result_sender: Arc<Mutex<mpsc::Sender<AsyncResult>>>,
    result_receiver: Arc<Mutex<mpsc::Receiver<AsyncResult>>>,
    pub server_ip: String,
    pub player_name: String,
    pub room_name: String,
    pub connected: bool,
    pub server_started: bool,
    pub is_host: bool,
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
        let http_client_clone = self.http_client.clone();
        let tx = self.result_sender.clone();
        self.runtime.spawn(async move {
            match http_client_clone.post(&url).send().await {
                Ok(response) => {
                    let result_text = response.text().await.unwrap_or_default();
                    tx.lock()
                        .await
                        .send(AsyncResult::ProcessJoinServer(
                            result_text,
                            server_ip_text.to_string(),
                            room_name_text.to_string(),
                            player_name_text.to_string(),
                        ))
                        .await
                        .unwrap();
                }
                Err(error) => {
                    let error_message = if let Some(status) = error.status() {
                        format!("Error connecting to server {status}")
                    } else {
                        format!("Error connecting to server {error}")
                    };

                    tx.lock()
                        .await
                        .send(AsyncResult::ProcessJoinServerError(error_message))
                        .await
                        .unwrap();
                }
            }
        });
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

        let http_client_clone = self.http_client.clone();
        self.runtime.spawn(async move {
            match http_client_clone.post(&url).send().await {
                Ok(_) => {}
                Err(error) => {
                    godot_print!("Error starting server {error}");
                }
            }
        });
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
        let http_client_clone = self.http_client.clone();
        self.runtime.spawn(async move {
            match http_client_clone.post(&url).send().await {
                Ok(_) => {}
                Err(error) => {
                    godot_print!("Error disconnecting from server {error}");
                }
            }
        });

        // Show connect ui
        self.base.get_node_as::<Control>("ConnectUI").show();

        self.connected = false;
    }

    #[func]
    fn on_submit_question_pressed(&mut self) {
        self.submit_question();
    }

    #[func]
    fn on_submit_guess_pressed(&mut self) {}

    #[func]
    fn on_convert_score_pressed(&mut self) {}

    #[func]
    fn on_refresh_game_state(&mut self) {
        if self.connected {
            self.refresh_game_state();
        }
    }
}

#[godot_api]
impl IControl for DeducersMain {
    fn init(base: Base<Control>) -> Self {
        let (tx, rx) = mpsc::channel::<AsyncResult>(32);

        Self {
            base,
            runtime: tokio::runtime::Runtime::new().unwrap(),
            http_client: reqwest::Client::builder()
                .timeout(Duration::from_secs(5))
                .build()
                .unwrap(),
            result_sender: Arc::new(Mutex::new(tx)),
            result_receiver: Arc::new(Mutex::new(rx)),
            server_ip: String::new(),
            player_name: String::new(),
            room_name: String::new(),
            connected: false,
            server_started: false,
            is_host: false,
            management_info_text_clear_time: None,
        }
    }

    fn ready(&mut self) {
        // Show connect ui
        self.base.get_node_as::<Control>("ConnectUI").show();
    }

    fn process(&mut self, _: f64) {
        // Clear management info text if it's time
        if let Some(clear_time) = self.management_info_text_clear_time {
            if clear_time < Utc::now() {
                self.base
                    .get_node_as::<Label>("GameUI/HBoxContainer/VBoxContainer/Management/MarginContainer/VBoxContainer/ManagementInfoLabel")
                    .set_text("".into());
                self.management_info_text_clear_time = None;
            }
        }

        // Collect results
        let mut results = Vec::new();
        {
            let mut receiver = self.result_receiver.blocking_lock();
            while let Ok(result) = receiver.try_recv() {
                results.push(result);
            }
        }

        // Process async results
        for result in results {
            match result {
                AsyncResult::ProcessJoinServer(response, server_ip, room_name, player_name) => {
                    self.process_join_server(&response, server_ip, room_name, player_name);
                }
                AsyncResult::ProcessJoinServerError(error_message) => {
                    self.show_alert(error_message);
                }
                AsyncResult::QuestionSubmitted => {
                    self.question_submitted();
                }
                AsyncResult::QuestionSubmitError(error_message) => {
                    self.show_management_info(error_message, 5000);
                }
                AsyncResult::QuestionVoted(button_id) => {
                    self.question_queue_vote_pressed(button_id);
                }
                AsyncResult::RefreshGameState(response, ping) => {
                    self.refresh_game_state_received(&response, ping);
                }
                AsyncResult::RefreshGameStateError(error_message) => {
                    godot_print!("Error getting game state {error_message}");

                    // Disconnect
                    self.base.get_node_as::<Control>("ConnectUI").show();
                    self.connected = false;
                    self.show_alert("Lost connection to server".to_string());
                }
                AsyncResult::KickPlayer(button_id) => {
                    self.kick_player_pressed(button_id);
                }
            }
        }
    }
}
