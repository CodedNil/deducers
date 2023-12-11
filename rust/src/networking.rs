use godot::{
    engine::{AudioStream, Button, CheckBox, ColorRect, Control, IControl, Label, LineEdit, OptionButton, ResourceLoader},
    prelude::*,
};
use std::{sync::Arc, time::Duration};
use tokio::{
    sync::{mpsc, Mutex},
    time::Instant,
};

pub const SUBMIT_QUESTION_EVERY_X_SECONDS: f64 = 10.0;

pub const SUBMIT_QUESTION_COST: usize = 2;
pub const ANONYMOUS_QUESTION_COST: usize = 5;
pub const GUESS_ITEM_COST: usize = 3;

pub const SCORE_TO_COINS_RATIO: usize = 2;

pub enum AsyncResult {
    ProcessJoinServer(String, String, String, String),
    ProcessJoinServerError(String),
    QuestionSubmitError(String),
    QuestionVoted(u32),
    GuessItemError(String),
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
    pub management_info_text_clear_time: Option<Instant>,
    pub guess_dialog_clear_time: Option<Instant>,
}

#[godot_api]
impl DeducersMain {
    #[func]
    fn on_connect_button_pressed(&mut self) {
        self.play_button_pressed_sound();

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
        let url = format!("http://{server_ip_text}/server/{room_name_text}/connect/{player_name_text}");
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
                    let error_message = error.status().map_or_else(
                        || format!("Error connecting to server {error}"),
                        |status| format!("Error connecting to server {status}"),
                    );

                    tx.lock().await.send(AsyncResult::ProcessJoinServerError(error_message)).await.unwrap();
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

    #[allow(clippy::cast_sign_loss)]
    pub fn show_management_info(&mut self, message: String, duration: i64) {
        // Set message text, then wait duration and if message text is still the same, clear it
        self.base
            .get_node_as::<Label>("GameUI/HBoxContainer/VBoxContainer/Management/MarginContainer/VBoxContainer/ManagementInfoLabel")
            .set_text(message.into());
        // Set management info text clear time
        self.management_info_text_clear_time = Some(Instant::now() + Duration::from_millis(duration as u64));
    }

    #[func]
    fn on_error_dialog_ok_pressed(&mut self) {
        self.play_button_pressed_sound();
        self.base.get_node_as::<ColorRect>("AlertDialog").hide();
    }

    pub fn play_sound(&mut self, sound: &str) {
        // Load the audio streams
        let sound = ResourceLoader::singleton()
            .load(format!("res://Resources/{sound}").into())
            .unwrap()
            .cast::<AudioStream>();

        // Create an AudioStreamPlayer node
        let mut audio_stream_player = AudioStreamPlayer::new_alloc();
        audio_stream_player.set_stream(sound);

        // Free the audio stream player when it's done playing
        let player_ref = audio_stream_player.clone();
        audio_stream_player.connect("finished".into(), Callable::from_object_method(&player_ref, "queue_free"));

        let mut player_ref = audio_stream_player.clone();
        self.base.add_child(audio_stream_player.upcast::<Node>());

        // Play the audio stream
        player_ref.play();
    }

    pub fn play_button_pressed_sound(&mut self) {
        self.play_sound("button_pressed.mp3");
    }

    #[func]
    fn on_input_typed(&mut self, _: GString) {
        self.play_sound("typing.mp3");
    }

    #[func]
    fn on_button_pressed(&mut self) {
        self.play_sound("button_pressed.mp3");
    }

    #[func]
    fn on_start_server_pressed(&mut self) {
        self.play_button_pressed_sound();

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
        self.play_button_pressed_sound();

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
        self.play_button_pressed_sound();
        self.submit_question();
    }

    #[func]
    fn on_anonymous_checkbox_pressed(&mut self) {
        self.play_button_pressed_sound();

        let checkbox = self
            .base
            .get_node_as::<CheckBox>("GameUI/HBoxContainer/VBoxContainer/Management/MarginContainer/VBoxContainer/AnonymousCheckbox");
        let cost = SUBMIT_QUESTION_COST + if checkbox.is_pressed() { ANONYMOUS_QUESTION_COST } else { 0 };
        self.base
            .get_node_as::<Button>("GameUI/HBoxContainer/VBoxContainer/Management/MarginContainer/VBoxContainer/QuestionSubmit/SubmitButton")
            .set_text(format!("Submit Question {cost} Coins").into());
    }

    #[func]
    fn on_submit_guess_pressed(&mut self) {
        self.play_button_pressed_sound();

        let mut guess_text_lineedit = self
            .base
            .get_node_as::<LineEdit>("GameUI/HBoxContainer/VBoxContainer/Management/MarginContainer/VBoxContainer/GuessItem/GuessText");
        let guess_text = guess_text_lineedit.get_text().to_string();
        guess_text_lineedit.set_text("".into());
        let item_choice_button = self
            .base
            .get_node_as::<OptionButton>("GameUI/HBoxContainer/VBoxContainer/Management/MarginContainer/VBoxContainer/GuessItem/ItemChoice");
        if item_choice_button.get_selected() == -1 {
            return;
        }
        let item_choice = item_choice_button.get_item_text(item_choice_button.get_selected()).to_string();

        // Make post request to guess
        let url = format!(
            "http://{server_ip}/server/{room_name}/guessitem/{player_name}/{item_choice}/{guess_text}",
            server_ip = self.server_ip,
            room_name = self.room_name,
            player_name = self.player_name
        );
        let http_client_clone = self.http_client.clone();
        let tx = self.result_sender.clone();
        self.runtime.spawn(async move {
            match http_client_clone.post(&url).send().await {
                Ok(_) => {}
                Err(error) => {
                    let error_message = error
                        .status()
                        .map_or_else(|| format!("Error guessing item {error}"), |status| format!("Error guessing item {status}"));

                    tx.lock().await.send(AsyncResult::GuessItemError(error_message)).await.unwrap();
                }
            }
        });
    }

    #[func]
    fn on_convert_score_pressed(&mut self) {
        self.play_button_pressed_sound();

        // Make post request to convert
        let url = format!(
            "http://{server_ip}/server/{room_name}/convertscore/{player_name}",
            server_ip = self.server_ip,
            room_name = self.room_name,
            player_name = self.player_name
        );
        let http_client_clone = self.http_client.clone();
        self.runtime.spawn(async move {
            match http_client_clone.post(&url).send().await {
                Ok(_) => {}
                Err(error) => {
                    godot_print!("Error converting score {error}");
                }
            }
        });
    }

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
            http_client: reqwest::Client::builder().timeout(Duration::from_secs(10)).build().unwrap(),
            result_sender: Arc::new(Mutex::new(tx)),
            result_receiver: Arc::new(Mutex::new(rx)),
            server_ip: String::new(),
            player_name: String::new(),
            room_name: String::new(),
            connected: false,
            server_started: false,
            is_host: false,
            management_info_text_clear_time: None,
            guess_dialog_clear_time: None,
        }
    }

    fn ready(&mut self) {
        // Show connect ui
        self.base.get_node_as::<Control>("ConnectUI").show();

        // Update costs in the UI
        self.base
            .get_node_as::<Button>("GameUI/HBoxContainer/VBoxContainer/Management/MarginContainer/VBoxContainer/QuestionSubmit/SubmitButton")
            .set_text(format!("Submit Question {SUBMIT_QUESTION_COST} Coins").into());
        self.base
            .get_node_as::<CheckBox>("GameUI/HBoxContainer/VBoxContainer/Management/MarginContainer/VBoxContainer/AnonymousCheckbox")
            .set_text(format!("Anonymous (+{ANONYMOUS_QUESTION_COST} Coins)").into());
        self.base
            .get_node_as::<Button>("GameUI/HBoxContainer/VBoxContainer/Management/MarginContainer/VBoxContainer/ConvertScoreButton")
            .set_text(format!("Convert Leaderboard Score To {SCORE_TO_COINS_RATIO} Coins").into());
        self.base
            .get_node_as::<Button>("GameUI/HBoxContainer/VBoxContainer/Management/MarginContainer/VBoxContainer/GuessItem/SubmitButton")
            .set_text(format!("Submit Guess {GUESS_ITEM_COST} Coins").into());
    }

    fn process(&mut self, _: f64) {
        // Clear management info text if it's time
        if let Some(clear_time) = self.management_info_text_clear_time {
            if Instant::now() >= clear_time {
                self.base
                    .get_node_as::<Label>("GameUI/HBoxContainer/VBoxContainer/Management/MarginContainer/VBoxContainer/ManagementInfoLabel")
                    .set_text("".into());
                self.management_info_text_clear_time = None;
            }
        }
        // Clear guess dialog text if it's time
        if let Some(clear_time) = self.guess_dialog_clear_time {
            if Instant::now() >= clear_time {
                self.base.get_node_as::<Control>("GuessedDialog").hide();
                self.guess_dialog_clear_time = None;
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
                AsyncResult::QuestionSubmitError(error_message) | AsyncResult::GuessItemError(error_message) => {
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
