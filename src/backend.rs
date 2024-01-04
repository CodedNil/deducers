use crate::{
    backend::{
        items::{add_item_to_lobby, ask_top_question},
        words::topup_lobby_if_available,
    },
    IDLE_KICK_TIME, ITEM_NAME_PATTERN, LOBBY_ID_PATTERN, MAX_CHAT_LENGTH, MAX_CHAT_MESSAGES, MAX_ITEM_NAME_LENGTH, MAX_LOBBY_ID_LENGTH,
    MAX_LOBBY_ITEMS, MAX_PLAYER_NAME_LENGTH, PLAYER_NAME_PATTERN,
};
use anyhow::{anyhow, bail, ensure, Result};
use once_cell::sync::Lazy;
use rand::prelude::*;
use regex::Regex;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time,
};
use strum::IntoEnumIterator;
use strum_macros::{Display, EnumIter, EnumProperty, EnumString};

pub mod items;
pub mod openai;
pub mod question_queue;
pub mod words;

#[derive(Clone, Default)]
pub struct Lobby {
    pub id: String,
    pub started: bool,
    pub starting: bool,
    pub ended: bool,
    pub elapsed_time: f64,
    pub last_update: f64,
    pub key_player: String,
    pub players: HashMap<String, Player>,
    pub coins_countdown: f64,
    pub chat_messages: Vec<ChatMessage>,
    pub questions_queue: Vec<QueuedQuestion>,
    pub questions_queue_countdown: f64,
    pub quizmaster_queue: Vec<QueuedQuestionQuizmaster>,
    pub items: Vec<Item>,
    pub items_queue: Vec<String>,
    pub items_counter: usize,
    pub questions_counter: usize,
    pub settings: LobbySettings,
}

impl Lobby {
    pub fn questions_queue_active(&self) -> bool {
        self.questions_queue.iter().any(|q| q.votes >= self.settings.question_min_votes)
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct LobbySettings {
    pub item_count: usize,
    pub difficulty: Difficulty,
    pub player_controlled: bool,
    pub theme: String,

    pub starting_coins: usize,
    pub coin_every_x_seconds: usize,
    pub submit_question_every_x_seconds: usize,
    pub add_item_every_x_questions: usize,

    pub submit_question_cost: usize,
    pub masked_question_cost: usize,
    pub guess_item_cost: usize,
    pub question_min_votes: usize,

    pub score_to_coins_ratio: usize,
}

impl Default for LobbySettings {
    fn default() -> Self {
        Self {
            item_count: 6,
            difficulty: Difficulty::default(),
            player_controlled: false,
            theme: String::new(),
            starting_coins: 4,
            coin_every_x_seconds: 6,
            submit_question_every_x_seconds: 10,
            add_item_every_x_questions: 5,
            submit_question_cost: 4,
            masked_question_cost: 8,
            guess_item_cost: 3,
            question_min_votes: 2,
            score_to_coins_ratio: 2,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, EnumString, Default, Display, EnumIter)]
pub enum Difficulty {
    #[default]
    Easy,
    Medium,
    Hard,
}

#[derive(Clone)]
pub enum AlterLobbySetting {
    ItemCount(usize),
    Difficulty(Difficulty),
    PlayerControlled(bool),
    Theme(String),
    AddItem(String),
    RemoveItem(String),
    RefreshItem(String),
    RefreshAllItems,
    Advanced(String, usize),
}

#[derive(Clone, PartialEq, Eq)]
pub enum PlayerMessage {
    ItemAdded,
    QuestionAsked,
    QuestionRejected(String),
    AlertPopup(String),
    GameStart,
    CoinGiven,
    ItemGuessed(String, usize, String),
    GuessIncorrect,
    ItemRemoved(usize, String),
    Winner(String),
    PlayerKicked,
}

#[derive(Clone, Default)]
pub struct Player {
    pub name: String,
    pub last_contact: f64,
    pub quizmaster: bool,
    pub score: usize,
    pub coins: usize,
    pub messages: Vec<PlayerMessage>,
}

#[derive(Clone, PartialEq, Eq)]
pub struct PlayerReduced {
    pub name: String,
    pub quizmaster: bool,
    pub score: usize,
    pub coins: usize,
}

impl Player {
    pub fn reduce(&self) -> PlayerReduced {
        PlayerReduced {
            name: self.name.clone(),
            quizmaster: self.quizmaster,
            score: self.score,
            coins: self.coins,
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct ChatMessage {
    pub player: String,
    pub message: String,
}

#[derive(Clone, PartialEq, Eq)]
pub struct QueuedQuestion {
    pub player: String,
    pub question: String,
    pub masked: bool,
    pub votes: usize,
    pub voters: Vec<String>,
}

#[derive(Clone, PartialEq, Eq)]
pub struct QueuedQuestionQuizmaster {
    pub player: String,
    pub question: String,
    pub masked: bool,
    pub items: Vec<QuizmasterItem>,
    pub voters: Vec<String>,
}

#[derive(Clone, PartialEq, Eq)]
pub struct QuizmasterItem {
    pub name: String,
    pub id: usize,
    pub answer: Answer,
}

#[derive(Clone, PartialEq, Eq)]
pub struct Item {
    pub name: String,
    pub id: usize,
    pub questions: Vec<Question>,
}

#[derive(Clone, PartialEq, Eq)]
pub struct Question {
    pub player: String,
    pub id: usize,
    pub text: String,
    pub answer: Answer,
    pub masked: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, EnumString, Display, EnumIter, EnumProperty)]
pub enum Answer {
    #[strum(props(color = "rgb(60, 130, 50)"))]
    Yes,
    #[strum(props(color = "rgb(130, 50, 50)"))]
    No,
    #[strum(props(color = "rgb(140, 80, 0)"))]
    Maybe,
    #[strum(props(color = "rgb(80, 80, 80)"))]
    Unknown,
}

static LOBBYS: Lazy<Arc<Mutex<HashMap<String, Lobby>>>> = Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

pub fn with_lobby<F, T>(lobby_id: &str, f: F) -> Result<T>
where
    F: FnOnce(&mut Lobby) -> Result<T>,
{
    let mut lobbys_lock = LOBBYS.lock().unwrap();
    let lobby = lobbys_lock
        .get_mut(lobby_id)
        .ok_or_else(|| anyhow!("Lobby '{lobby_id}' not found"))?;
    f(lobby)
}

pub fn with_player<F, T>(lobby_id: &str, player_name: &str, f: F) -> Result<T>
where
    F: FnOnce(Lobby, &mut Player) -> Result<T>,
{
    with_lobby(lobby_id, |lobby| {
        let lobby_state = lobby.clone();
        let player = lobby
            .players
            .get_mut(player_name)
            .ok_or_else(|| anyhow!("Player '{player_name}' not found"))?;
        f(lobby_state, player)
    })
}

#[derive(PartialEq, Eq)]
pub struct LobbyInfo {
    pub id: String,
    pub started: bool,
    pub players_count: usize,
}

pub fn get_lobby_info() -> Vec<LobbyInfo> {
    let lobbys_lock = LOBBYS.lock().unwrap();
    let mut lobby_infos = Vec::new();
    for (id, lobby) in &lobbys_lock.clone() {
        lobby_infos.push(LobbyInfo {
            id: id.clone(),
            started: lobby.started || lobby.starting,
            players_count: lobby.players.len(),
        });
    }
    lobby_infos
}

pub fn create_lobby(lobby_id: &str, player_name: &str) -> Result<()> {
    let mut lobbys_lock = LOBBYS.lock().unwrap();
    if lobbys_lock.contains_key(lobby_id) {
        return Ok(());
    }
    lobbys_lock.insert(
        lobby_id.to_owned(),
        Lobby {
            id: lobby_id.to_owned(),
            last_update: get_current_time(),
            key_player: player_name.to_owned(),
            ..Default::default()
        },
    );
    drop(lobbys_lock);
    println!("Lobby '{lobby_id}' created by key player '{player_name}'");

    if lobby_id == "debug" {
        create_debug_lobby(lobby_id)?;
    }
    Ok(())
}

fn create_debug_lobby(lobby_id: &str) -> Result<()> {
    println!("Creating debug lobby");
    with_lobby(lobby_id, |lobby| {
        lobby.started = true;
        lobby.last_update = get_current_time();
        lobby.items_queue = ["Apple", "Banana", "Orange", "Pear", "Pineapple"]
            .iter()
            .map(std::string::ToString::to_string)
            .collect();
        add_item_to_lobby(lobby);
        add_item_to_lobby(lobby);
        for _ in 0..10 {
            lobby.chat_messages.push(ChatMessage {
                player: "debug".to_owned(),
                message: rand::random::<usize>().to_string(),
            });
        }
        let questions = vec!["brown", "red", "yellow", "green", "blue", "purple", "orange", "black", "white"];
        for question in questions {
            let question = format!("Is it {question}");
            lobby.questions_queue.push(QueuedQuestion {
                player: "debug".to_owned(),
                question: question.clone(),
                votes: rand::random::<usize>() % 6,
                voters: Vec::new(),
                masked: rand::random::<usize>() % 5 == 0,
            });
            let id = lobby.questions_counter;
            lobby.questions_counter += 1;
            for item in &mut lobby.items {
                item.questions.push(Question {
                    player: "debug".to_owned(),
                    id,
                    text: question.clone(),
                    answer: Answer::iter().choose(&mut rand::thread_rng()).unwrap(),
                    masked: rand::random::<usize>() % 5 == 0,
                });
            }
            if lobby.questions_counter % lobby.settings.add_item_every_x_questions == 0 {
                add_item_to_lobby(lobby);
            }
        }
        Ok(())
    })
}

pub fn connect_player(lobby_id: &str, player_name: &str) -> Result<()> {
    if !(3..=MAX_LOBBY_ID_LENGTH).contains(&lobby_id.len()) {
        bail!("Lobby ID must be between 3 and {MAX_LOBBY_ID_LENGTH} characters long");
    }
    if !(3..=MAX_PLAYER_NAME_LENGTH).contains(&player_name.len()) {
        bail!("Player name must be between 3 and {MAX_PLAYER_NAME_LENGTH} characters long");
    }
    if !regex_match(LOBBY_ID_PATTERN, lobby_id) || !regex_match(PLAYER_NAME_PATTERN, player_name) {
        bail!("Lobby ID and Player name must be alphabetic");
    }
    ensure!(player_name != "SYSTEM", "Player name cannot be 'SYSTEM'");

    if let Err(e) = create_lobby(lobby_id, player_name) {
        println!("Error creating lobby {e}");
    }

    with_lobby(lobby_id, |lobby| {
        ensure!(!lobby.players.contains_key(player_name), "Player '{player_name}' already in lobby");

        lobby.players.entry(player_name.to_owned()).or_insert(Player {
            name: player_name.to_owned(),
            last_contact: get_current_time(),
            coins: if lobby.started {
                lobby.settings.starting_coins + (lobby.elapsed_time / lobby.settings.coin_every_x_seconds as f64).floor() as usize
            } else {
                0
            },
            ..Default::default()
        });

        add_chat_message_to_lobby(lobby, "SYSTEM", &format!("Player '{player_name}' connected"));
        Ok(())
    })
}

pub fn disconnect_player(lobby_id: &str, player_name: &str) {
    let _result = with_lobby(lobby_id, |lobby| {
        add_chat_message_to_lobby(lobby, "SYSTEM", &format!("Player '{player_name}' left"));
        lobby.players.remove(player_name);
        Ok(())
    });
}

pub fn alter_lobby_settings(lobby_id: &str, player_name: &str, setting: AlterLobbySetting) {
    let result = with_lobby(lobby_id, |lobby| {
        ensure!(!lobby.started && !lobby.starting, "Lobby is started");
        ensure!(player_name == lobby.key_player, "Only the key player can alter the lobby settings");
        match setting {
            AlterLobbySetting::ItemCount(item_count) => {
                ensure!((1..=MAX_LOBBY_ITEMS).contains(&item_count), "Items must be 1 to {MAX_LOBBY_ITEMS}");
                lobby.settings.item_count = item_count;
                lobby.items_queue.truncate(item_count);
            }
            AlterLobbySetting::Difficulty(difficulty) => {
                lobby.settings.difficulty = difficulty;
            }
            AlterLobbySetting::PlayerControlled(player_controlled) => {
                lobby.settings.player_controlled = player_controlled;
            }
            AlterLobbySetting::Theme(theme) => {
                lobby.settings.theme = theme;
            }
            AlterLobbySetting::AddItem(item) => {
                // If item is empty, pick a random unique word from the difficulty
                if item.is_empty() {
                    lobby.settings.item_count += 1;
                    return Ok(());
                }
                // Else check if the item is valid and add it to the queue
                ensure!(regex_match(ITEM_NAME_PATTERN, &item), "Item name must be alphabetic");
                if !(3..=MAX_ITEM_NAME_LENGTH).contains(&item.len()) {
                    bail!("Item name must be between 3 and {MAX_ITEM_NAME_LENGTH} characters long");
                }
                ensure!(!lobby.items_queue.contains(&item), "Item already exists in the lobby");
                // Capitalise the first letter of the item
                let item = item
                    .chars()
                    .enumerate()
                    .map(|(i, c)| if i == 0 { c.to_ascii_uppercase() } else { c })
                    .collect::<String>();
                lobby.items_queue.push(item);
                lobby.settings.item_count += 1;
            }
            AlterLobbySetting::RemoveItem(item) => {
                let index = lobby.items_queue.iter().position(|i| i == &item);
                if let Some(index) = index {
                    lobby.items_queue.remove(index);
                    lobby.settings.item_count -= 1;
                }
            }
            AlterLobbySetting::RefreshItem(item) => {
                let index = lobby.items_queue.iter().position(|i| i == &item);
                if let Some(index) = index {
                    lobby.items_queue.remove(index);
                }
            }
            AlterLobbySetting::RefreshAllItems => {
                lobby.items_queue.clear();
            }
            AlterLobbySetting::Advanced(key, value) => match key.as_str() {
                "starting_coins" => lobby.settings.starting_coins = value,
                "coin_every_x_seconds" => lobby.settings.coin_every_x_seconds = value,
                "submit_question_every_x_seconds" => lobby.settings.submit_question_every_x_seconds = value,
                "add_item_every_x_questions" => lobby.settings.add_item_every_x_questions = value,
                "submit_question_cost" => lobby.settings.submit_question_cost = value,
                "masked_question_cost" => lobby.settings.masked_question_cost = value,
                "guess_item_cost" => lobby.settings.guess_item_cost = value,
                "question_min_votes" => lobby.settings.question_min_votes = value,
                "score_to_coins_ratio" => lobby.settings.score_to_coins_ratio = value,
                _ => bail!("Invalid advanced setting key"),
            },
        }

        Ok(())
    });
    if let Err(e) = result {
        alert_popup(lobby_id, player_name, &format!("Setting change failed {e}"));
    }
}

pub fn start_lobby(lobby_id: &str, player_name: &str) {
    let result = with_lobby(lobby_id, |lobby| {
        if lobby.started || lobby.starting {
            bail!("Lobby '{lobby_id}' already started");
        } else if player_name != lobby.key_player {
            bail!("Only the key player can start the lobby '{lobby_id}'",);
        }
        lobby.starting = true;
        if !lobby.settings.player_controlled {
            lobby.items_queue = Vec::new();
        }
        let lobby_settings_str = format!(
            "{} Items, theme {}, {}, {}",
            lobby.settings.item_count,
            lobby.settings.theme,
            lobby.settings.difficulty,
            if lobby.settings.player_controlled {
                "Quizmaster"
            } else {
                "AI Controlled"
            }
        );

        println!("Lobby '{lobby_id}' started by key player '{player_name}' with settings {lobby_settings_str}");
        Ok(())
    });
    if let Err(e) = result {
        alert_popup(lobby_id, player_name, &format!("Start lobby failed {e}"));
    }
}

pub fn alert_popup(lobby_id: &str, player_name: &str, message: &str) {
    let result = with_player(lobby_id, player_name, |_, player| {
        player.messages.push(PlayerMessage::AlertPopup(message.to_owned()));
        Ok(())
    });
    if let Err(e) = result {
        println!("Alert popup failed {e}");
    }
}

pub fn kick_player(lobby_id: &str, player_name: &str, player_to_kick: &str) {
    let result = with_player(lobby_id, player_to_kick, |_, player| {
        player.messages.push(PlayerMessage::PlayerKicked);
        Ok(())
    });
    add_chat_message(lobby_id, "SYSTEM", &format!("Player '{player_to_kick}' was kicked"));
    if let Err(e) = result {
        alert_popup(lobby_id, player_name, &format!("Kick failed {e}"));
    }
}

pub fn add_chat_message(lobby_id: &str, player_name: &str, message: &str) {
    let error_message = if message.len() < 2 {
        Some("Chat message must be at least 2 characters long".to_string())
    } else if message.len() > MAX_CHAT_LENGTH {
        Some(format!("Chat message must be less than {MAX_CHAT_LENGTH} characters long"))
    } else {
        match with_lobby(lobby_id, |lobby| {
            lobby.chat_messages.push(ChatMessage {
                player: player_name.to_owned(),
                message: message.to_owned(),
            });
            if lobby.chat_messages.len() > MAX_CHAT_MESSAGES {
                lobby.chat_messages.remove(0);
            }
            Ok(())
        }) {
            Ok(()) => None,
            Err(e) => Some(format!("Chat message failed to send {e}")),
        }
    };
    if let Some(msg) = error_message {
        alert_popup(lobby_id, player_name, &msg);
    }
}

pub fn add_chat_message_to_lobby(lobby: &mut Lobby, player_name: &str, message: &str) {
    lobby.chat_messages.push(ChatMessage {
        player: player_name.to_owned(),
        message: message.to_owned(),
    });
    if lobby.chat_messages.len() > MAX_CHAT_MESSAGES {
        lobby.chat_messages.remove(0);
    }
}

pub fn get_state(lobby_id: &str, player_name: &str) -> Result<(Lobby, Vec<PlayerMessage>)> {
    let mut should_kick = false;
    let result = with_player(lobby_id, player_name, |lobby, player| {
        player.last_contact = get_current_time();
        let messages = player.messages.clone();
        if messages.contains(&PlayerMessage::PlayerKicked) {
            should_kick = true;
        }
        player.messages.clear();
        Ok((lobby, messages))
    });
    if should_kick {
        let _result = with_lobby(lobby_id, |lobby| {
            lobby.players.remove(player_name);
            Ok(())
        });
    }
    result
}

pub fn get_current_time() -> f64 {
    let now = time::SystemTime::now();
    now.duration_since(time::UNIX_EPOCH).unwrap_or_default().as_secs_f64()
}

fn regex_match(pattern: &str, haystack: &str) -> bool {
    Regex::new(pattern).unwrap().is_match(haystack)
}

pub fn lobby_loop() {
    let mut lobbys_lock = LOBBYS.lock().unwrap();

    // Iterate through lobbys to update or remove
    let mut lobbies_needing_words = Vec::new();
    lobbys_lock.retain(|lobby_id, lobby| {
        let current_time = get_current_time();

        // Remove inactive players and check if key player is active
        lobby.players.retain(|player_id, player| {
            if current_time - player.last_contact > IDLE_KICK_TIME {
                println!("Kicking player '{player_id}' due to idle");
                false
            } else {
                true
            }
        });
        let key_player_active = lobby.players.contains_key(&lobby.key_player);

        // Remove lobby if key player is inactive or no players are left
        if !key_player_active || lobby.players.is_empty() {
            println!("Removing lobby '{lobby_id}' due to no key player or no players");
            false
        } else {
            // Update lobby state if lobby is started
            if lobby.started {
                let elapsed_time_update = current_time - lobby.last_update;
                if lobby.ended && lobby.elapsed_time > 10.0 {
                    println!("Removing lobby '{lobby_id}' due to ended and elapsed time");
                    return false;
                }

                // Distribute coins if countdown is ready
                lobby.coins_countdown -= elapsed_time_update;
                if lobby.coins_countdown <= 0.0 {
                    lobby.coins_countdown += lobby.settings.coin_every_x_seconds as f64;
                    for player in lobby.players.values_mut() {
                        if !player.quizmaster {
                            player.coins += 1;
                            player.messages.push(PlayerMessage::CoinGiven);
                        }
                    }
                }

                // If lobby has a queued question with at least QUESTION_MIN_VOTES votes, tick it down, else reset
                if lobby.questions_queue_active() {
                    lobby.questions_queue_countdown -= elapsed_time_update;
                    if lobby.questions_queue_countdown <= 0.0 {
                        lobby.questions_queue_countdown += lobby.settings.submit_question_every_x_seconds as f64;
                        let lobby_id_clone = lobby_id.clone();
                        tokio::spawn(async move {
                            let _result = ask_top_question(&lobby_id_clone).await;
                        });
                    }
                } else {
                    lobby.questions_queue_countdown = lobby.settings.submit_question_every_x_seconds as f64;
                }

                // Update the elapsed time and last update time for the lobby
                lobby.elapsed_time += elapsed_time_update;
                lobby.last_update = current_time;
            } else {
                if lobby.items_queue.len() > lobby.settings.item_count {
                    lobby.items_queue.truncate(lobby.settings.item_count);
                }
                if !lobby.started
                    && (lobby.starting || lobby.settings.player_controlled)
                    && lobby.items_queue.len() < lobby.settings.item_count
                {
                    lobbies_needing_words.push(lobby_id.clone());
                }
                if lobby.starting && lobby.items_queue.len() == lobby.settings.item_count {
                    add_chat_message_to_lobby(lobby, "SYSTEM", "The game has started, good luck!");
                    lobby.started = true;
                    lobby.starting = true;
                    lobby.last_update = get_current_time();

                    for player in lobby.players.values_mut() {
                        player.messages.push(PlayerMessage::GameStart);
                        player.coins = lobby.settings.starting_coins;
                        if player.name == lobby.key_player && lobby.settings.player_controlled {
                            player.quizmaster = true;
                        }
                    }

                    add_item_to_lobby(lobby);
                    if lobby.settings.item_count > 1 {
                        add_item_to_lobby(lobby);
                    }
                }
            }
            true
        }
    });

    for lobby_id in lobbies_needing_words {
        topup_lobby_if_available(&lobby_id);
    }
}
