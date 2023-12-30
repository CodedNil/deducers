#![allow(clippy::significant_drop_tightening)]
use crate::{
    backend::{
        items::{add_item_to_lobby, ask_top_question},
        parse_words::{select_lobby_words, select_lobby_words_unique},
    },
    IDLE_KICK_TIME, ITEM_NAME_PATTERN, LOBBY_ID_PATTERN, MAX_CHAT_LENGTH, MAX_CHAT_MESSAGES, MAX_ITEM_NAME_LENGTH, MAX_LOBBY_ID_LENGTH,
    MAX_LOBBY_ITEMS, MAX_PLAYER_NAME_LENGTH, PLAYER_NAME_PATTERN,
};
use anyhow::{anyhow, Result};
use once_cell::sync::Lazy;
use rand::prelude::*;
use std::{
    cmp::Ordering,
    collections::HashMap,
    fmt,
    sync::{Arc, Mutex},
    time,
};
use strum::IntoEnumIterator;
use strum_macros::{Display, EnumIter, EnumProperty, EnumString};

pub mod items;
pub mod openai;
pub mod parse_words;
pub mod question_queue;

#[derive(Clone, Default)]
pub struct Lobby {
    pub id: String,
    pub started: bool,
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
    pub items_history: Vec<String>,
    pub items_queue: Vec<String>,
    pub questions_counter: usize,
    pub settings: LobbySettings,
}

impl Lobby {
    pub fn questions_queue_active(&self) -> bool {
        self.questions_queue.iter().any(|q| q.votes >= self.settings.question_min_votes)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct LobbySettings {
    pub item_count: usize,
    pub difficulty: Difficulty,
    pub player_controlled: bool,

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
            difficulty: Difficulty::Easy,
            player_controlled: false,

            starting_coins: 4,
            coin_every_x_seconds: 8,
            submit_question_every_x_seconds: 10,
            add_item_every_x_questions: 5,

            submit_question_cost: 4,
            masked_question_cost: 8,
            guess_item_cost: 3,
            question_min_votes: 2,

            score_to_coins_ratio: 3,
        }
    }
}

impl fmt::Display for LobbySettings {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} Items, {}, {}",
            self.item_count,
            self.difficulty,
            if self.player_controlled { "Quizmaster" } else { "AI Controlled" }
        )
    }
}

#[derive(Clone, Copy, PartialEq, Eq, EnumString, Display, EnumIter)]
pub enum Difficulty {
    Easy,
    Medium,
    Hard,
}

#[derive(Clone)]
pub enum AlterLobbySetting {
    ItemCount(usize),
    Difficulty(Difficulty),
    PlayerControlled(bool),
    AddItem(String),
    RemoveItem(String),
    RefreshItem(String),
    RefreshAllItems,
    Advanced(String, usize),
}

#[derive(Clone)]
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
    Winner(Vec<String>),
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
    F: FnOnce(&Lobby) -> Result<T>,
{
    let lobbys_lock = LOBBYS.lock().unwrap();
    let lobby = lobbys_lock.get(lobby_id).ok_or_else(|| anyhow!("Lobby '{lobby_id}' not found"))?;
    f(lobby)
}

pub fn with_lobby_mut<F, T>(lobby_id: &str, f: F) -> Result<T>
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
    F: FnOnce(&Lobby, &Player) -> Result<T>,
{
    with_lobby(lobby_id, |lobby| {
        let player = lobby
            .players
            .get(player_name)
            .ok_or_else(|| anyhow!("Player '{player_name}' not found"))?;
        f(lobby, player)
    })
}

pub fn with_player_mut<F, T>(lobby_id: &str, player_name: &str, f: F) -> Result<T>
where
    F: FnOnce(Lobby, &mut Player) -> Result<T>,
{
    with_lobby_mut(lobby_id, |lobby| {
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
    pub players_count: usize,
}

pub fn get_lobby_info() -> Vec<LobbyInfo> {
    let lobbys_lock = LOBBYS.lock().unwrap();
    let mut lobby_infos = Vec::new();
    for (id, lobby) in &lobbys_lock.clone() {
        if !lobby.started {
            lobby_infos.push(LobbyInfo {
                id: id.clone(),
                players_count: lobby.players.len(),
            });
        }
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
            items_queue: select_lobby_words(LobbySettings::default().difficulty, LobbySettings::default().item_count),
            settings: LobbySettings::default(),
            ..Default::default()
        },
    );
    drop(lobbys_lock);
    println!("Lobby '{lobby_id}' created by key player '{player_name}'");

    // If lobby_id is debug, create a loaded lobby
    if lobby_id == "debug" {
        println!("Creating debug lobby");
        start_lobby(lobby_id, player_name);
        with_lobby_mut(lobby_id, |lobby| {
            for _ in 0..10 {
                lobby.chat_messages.push(ChatMessage {
                    player: "debug".to_owned(),
                    message: select_lobby_words(Difficulty::Easy, 1).pop().unwrap(),
                });
            }
            let questions: Vec<&str> = vec![
                "Is it a living thing?",
                "Is it bigger than a breadbox?",
                "Is it made by humans?",
                "Can it be found indoors?",
                "Is it used for communication?",
                "Is it a type of food?",
                "Is it electronic?",
                "Can it move?",
                "Is it usually colorful?",
                "Does it make a sound?",
                "Is it found in nature?",
                "Is it related to sports?",
                "Does it have a specific smell?",
                "Is it heavier than a person?",
                "Can it be worn?",
            ];
            for question in questions {
                lobby.questions_queue.push(QueuedQuestion {
                    player: "debug".to_owned(),
                    question: question.to_owned(),
                    votes: rand::random::<usize>() % 6,
                    voters: Vec::new(),
                    masked: rand::random::<usize>() % 5 == 0,
                });
                let question_id = lobby.questions_counter;
                lobby.questions_counter += 1;
                let masked = rand::random::<usize>() % 5 == 0;
                for item in &mut lobby.items {
                    item.questions.push(Question {
                        player: "debug".to_owned(),
                        id: question_id,
                        text: question.to_owned(),
                        answer: Answer::iter().choose(&mut rand::thread_rng()).unwrap(),
                        masked,
                    });
                }
                if lobby.questions_counter % lobby.settings.add_item_every_x_questions == 0 {
                    add_item_to_lobby(lobby);
                }
            }
            Ok(())
        })?;
    }
    Ok(())
}

#[allow(clippy::cast_sign_loss, clippy::cast_precision_loss, clippy::cast_possible_truncation)]
pub fn connect_player(lobby_id: &str, player_name: &str) -> Result<()> {
    let lobby_id = lobby_id.trim();
    let player_name = player_name.trim();
    if lobby_id.len() < 3 || lobby_id.len() > MAX_LOBBY_ID_LENGTH {
        return Err(anyhow!("Lobby ID must be between 3 and {MAX_LOBBY_ID_LENGTH} characters long"));
    }
    if player_name.len() < 3 || player_name.len() > MAX_PLAYER_NAME_LENGTH {
        return Err(anyhow!(
            "Player name must be between 3 and {MAX_PLAYER_NAME_LENGTH} characters long"
        ));
    }
    if !regex::Regex::new(LOBBY_ID_PATTERN).unwrap().is_match(lobby_id) {
        return Err(anyhow!("Lobby ID must be alphabetic"));
    }
    if !regex::Regex::new(PLAYER_NAME_PATTERN).unwrap().is_match(player_name) {
        return Err(anyhow!("Player name must be alphabetic"));
    }

    if let Err(e) = create_lobby(lobby_id, player_name) {
        println!("Error creating lobby {e}");
    }

    with_lobby_mut(lobby_id, |lobby| {
        if lobby.players.contains_key(player_name) {
            return Err(anyhow!("Player '{player_name}' is already connected to lobby '{lobby_id}'"));
        }

        // If lobby is started already, give starting coins plus coins for game time
        let coins = if lobby.started {
            lobby.settings.starting_coins + (lobby.elapsed_time / lobby.settings.coin_every_x_seconds as f64).floor() as usize
        } else {
            0
        };

        lobby.players.entry(player_name.to_owned()).or_insert(Player {
            name: player_name.to_owned(),
            last_contact: get_current_time(),
            coins,
            ..Default::default()
        });

        println!("Player '{player_name}' connected to lobby '{lobby_id}'");
        Ok(())
    })
}

pub fn disconnect_player(lobby_id: &str, player_name: &str) {
    let _result = with_lobby_mut(lobby_id, |lobby| {
        lobby.players.remove(player_name);
        println!("Player '{player_name}' disconnected from lobby '{lobby_id}'");
        Ok(())
    });
}

pub fn alter_lobby_settings(lobby_id: &str, player_name: &str, setting: AlterLobbySetting) {
    let result = with_lobby_mut(lobby_id, |lobby| {
        if player_name != lobby.key_player {
            return Err(anyhow!("Only the key player can alter the lobby settings"));
        }

        match setting {
            AlterLobbySetting::ItemCount(item_count) => {
                if !(1..=MAX_LOBBY_ITEMS).contains(&item_count) {
                    return Err(anyhow!("Item count must be between 1 and 20"));
                }
                lobby.settings.item_count = item_count;
                // Expand or shrink the items queue to match the new item count
                match lobby.items_queue.len().cmp(&item_count) {
                    Ordering::Less => {
                        lobby
                            .items_queue
                            .extend(select_lobby_words_unique(&lobby.items_queue, lobby.settings.difficulty, item_count));
                    }
                    Ordering::Greater => {
                        lobby.items_queue.truncate(item_count);
                    }
                    Ordering::Equal => {}
                }
            }
            AlterLobbySetting::Difficulty(difficulty) => {
                lobby.settings.difficulty = difficulty;
            }
            AlterLobbySetting::PlayerControlled(player_controlled) => {
                lobby.settings.player_controlled = player_controlled;
            }
            AlterLobbySetting::AddItem(item) => {
                // If item is empty, pick a random unique word from the difficulty
                if item.is_empty() {
                    lobby.items_queue.push(
                        select_lobby_words_unique(&lobby.items_queue, lobby.settings.difficulty, 1)
                            .pop()
                            .unwrap(),
                    );
                    lobby.settings.item_count = lobby.items_queue.len();
                    return Ok(());
                }
                // Else check if the item is valid and add it to the queue
                if !regex::Regex::new(ITEM_NAME_PATTERN).unwrap().is_match(&item) {
                    return Err(anyhow!("Item name must be alphabetic"));
                }
                if item.len() < 3 || item.len() > MAX_ITEM_NAME_LENGTH {
                    return Err(anyhow!("Item name must be between 3 and {MAX_ITEM_NAME_LENGTH} characters long"));
                }
                if lobby.items_queue.contains(&item) {
                    return Err(anyhow!("Item '{item}' already exists in the lobby"));
                }
                // Capitalise the first letter of the item
                let item = item
                    .chars()
                    .enumerate()
                    .map(|(i, c)| if i == 0 { c.to_ascii_uppercase() } else { c })
                    .collect::<String>();
                lobby.items_queue.push(item);
                lobby.settings.item_count = lobby.items_queue.len();
            }
            AlterLobbySetting::RemoveItem(item) => {
                let index = lobby.items_queue.iter().position(|i| i.to_lowercase() == item.to_lowercase());
                if let Some(index) = index {
                    lobby.items_queue.remove(index);
                }
                lobby.settings.item_count = lobby.items_queue.len();
            }
            AlterLobbySetting::RefreshItem(item) => {
                let index = lobby.items_queue.iter().position(|i| i.to_lowercase() == item.to_lowercase());
                if let Some(index) = index {
                    lobby.items_queue[index] = select_lobby_words_unique(&lobby.items_queue, lobby.settings.difficulty, 1)
                        .pop()
                        .unwrap();
                }
            }
            AlterLobbySetting::RefreshAllItems => {
                lobby.items_queue = select_lobby_words(lobby.settings.difficulty, lobby.settings.item_count);
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
                _ => return Err(anyhow!("Invalid advanced setting key")),
            },
        }

        Ok(())
    });
    if let Err(e) = result {
        alert_popup(lobby_id, player_name, &format!("Setting change failed {e}"));
    }
}

pub fn start_lobby(lobby_id: &str, player_name: &str) {
    let result = with_lobby_mut(lobby_id, |lobby| {
        if lobby.started {
            return Err(anyhow!("Lobby '{lobby_id}' already started"));
        } else if player_name != lobby.key_player {
            return Err(anyhow!("Only the key player can start the lobby '{lobby_id}'",));
        }
        lobby.started = true;
        lobby.last_update = get_current_time();

        for player in lobby.players.values_mut() {
            player.messages.push(PlayerMessage::GameStart);
            player.coins = lobby.settings.starting_coins;
            if player.name == lobby.key_player && lobby.settings.player_controlled {
                player.quizmaster = true;
            }
        }

        if !lobby.settings.player_controlled {
            lobby.items_queue = select_lobby_words(lobby.settings.difficulty, lobby.settings.item_count);
        }
        add_item_to_lobby(lobby);
        if lobby.settings.item_count > 1 {
            add_item_to_lobby(lobby);
        }

        println!(
            "Lobby '{lobby_id}' started by key player '{player_name}' with settings {}",
            lobby.settings
        );
        Ok(())
    });
    if let Err(e) = result {
        alert_popup(lobby_id, player_name, &format!("Start lobby failed {e}"));
    }
}

pub fn alert_popup(lobby_id: &str, player_name: &str, message: &str) {
    let result = with_player_mut(lobby_id, player_name, |_, player| {
        player.messages.push(PlayerMessage::AlertPopup(message.to_owned()));
        Ok(())
    });
    if let Err(e) = result {
        println!("Alert popup failed {e}");
    }
}

pub fn kick_player(lobby_id: &str, player_name: &str) {
    let result = with_lobby_mut(lobby_id, |lobby| {
        lobby.players.remove(player_name);
        println!("Lobby '{lobby_id}' player '{player_name}' kicked by key player");
        Ok(())
    });
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
        match with_lobby_mut(lobby_id, |lobby| {
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
            Err(e) => Some(format!("Chat message failed {e}")),
        }
    };
    if let Some(msg) = error_message {
        alert_popup(lobby_id, player_name, &msg);
    }
}

pub fn get_state(lobby_id: &str, player_name: &str) -> Result<(Lobby, Vec<PlayerMessage>)> {
    with_player_mut(lobby_id, player_name, |lobby, player| {
        player.last_contact = get_current_time();
        let messages = player.messages.clone();
        player.messages.clear();
        Ok((lobby, messages))
    })
}

pub fn get_current_time() -> f64 {
    let now = time::SystemTime::now();
    now.duration_since(time::UNIX_EPOCH).unwrap_or_default().as_secs_f64()
}

#[allow(clippy::cast_precision_loss)]
pub fn lobby_loop() {
    let mut lobbys_lock = LOBBYS.lock().unwrap();

    // Iterate through lobbys to update or remove
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
            if lobby.started && lobby_id != "debug" {
                let elapsed_time_update = current_time - lobby.last_update;

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
            }
            true
        }
    });
}
