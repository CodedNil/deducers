#![allow(clippy::missing_errors_doc, clippy::future_not_send, clippy::significant_drop_tightening)]
use crate::{
    backend::items::{add_item_to_lobby, ask_top_question, select_lobby_words},
    IDLE_KICK_TIME, ITEM_NAME_PATTERN, LOBBY_ID_PATTERN, MAX_CHAT_LENGTH, MAX_CHAT_MESSAGES, MAX_LOBBY_ID_LENGTH, MAX_LOBBY_ITEMS,
    MAX_PLAYER_NAME_LENGTH, PLAYER_NAME_PATTERN,
};
use anyhow::{anyhow, Result};
use once_cell::sync::Lazy;
use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet},
    fmt::Display,
    sync::Arc,
};
use tokio::sync::Mutex;

#[derive(Clone, Debug)]
pub struct Lobby {
    pub started: bool,
    pub elapsed_time: f64,
    pub last_update: f64,
    pub key_player: String,
    pub players: HashMap<String, Player>,
    pub chat_messages: Vec<ChatMessage>,
    pub questions_queue: Vec<QueuedQuestion>,
    pub questions_queue_waiting: bool,
    pub questions_queue_countdown: f64,
    pub quizmaster_queue: Vec<QueuedQuestionQuizmaster>,
    pub items: Vec<Item>,
    pub items_history: Vec<String>,
    pub items_queue: Vec<String>,
    pub questions_counter: usize,
    pub settings: LobbySettings,
}

#[derive(Clone, Debug)]
pub struct LobbySettings {
    pub item_count: usize,
    pub difficulty: Difficulty,
    pub player_controlled: bool,

    pub starting_coins: usize,
    pub coin_every_x_seconds: usize,
    pub submit_question_every_x_seconds: usize,
    pub add_item_every_x_questions: usize,

    pub submit_question_cost: usize,
    pub anonymous_question_cost: usize,
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
            anonymous_question_cost: 8,
            guess_item_cost: 3,
            question_min_votes: 2,

            score_to_coins_ratio: 3,
        }
    }
}

impl Display for LobbySettings {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Item Count: {}, Difficulty: {}, Player Controlled: {}",
            self.item_count,
            self.difficulty,
            if self.player_controlled { "Yes" } else { "No" }
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Difficulty {
    Easy,
    Medium,
    Hard,
}

impl std::str::FromStr for Difficulty {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "easy" => Ok(Self::Easy),
            "medium" => Ok(Self::Medium),
            "hard" => Ok(Self::Hard),
            _ => Err(anyhow!("Difficulty must be easy, medium, or hard")),
        }
    }
}

impl Display for Difficulty {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let difficulty = match self {
            Self::Easy => "Easy",
            Self::Medium => "Medium",
            Self::Hard => "Hard",
        };
        write!(f, "{difficulty}")
    }
}

#[derive(Clone, Debug)]
pub enum AlterLobbySetting {
    ItemCount(usize),
    Difficulty(Difficulty),
    PlayerControlled(bool),
    AddItem(String),
    RemoveItem(String),
    RefreshItem(String),
    RefreshAllItems,
}

#[derive(Clone, Debug)]
pub enum PlayerMessage {
    ItemAdded,
    QuestionAsked,
    GameStart,
    CoinGiven,
    ItemGuessed(String, usize, String),
    GuessIncorrect,
    ItemRemoved(usize, String),
    Winner(Vec<String>),
}

#[derive(Clone, Debug)]
pub struct Player {
    pub name: String,
    pub last_contact: f64,
    pub quizmaster: bool,
    pub score: usize,
    pub coins: usize,
    pub messages: Vec<PlayerMessage>,
}

#[derive(Clone, Debug)]
pub struct ChatMessage {
    pub player: String,
    pub message: String,
}

#[derive(Clone, Debug)]
pub struct QueuedQuestion {
    pub player: String,
    pub question: String,
    pub anonymous: bool,
    pub votes: usize,
}

#[derive(Clone, Debug)]
pub struct QueuedQuestionQuizmaster {
    pub player: String,
    pub question: String,
    pub anonymous: bool,
    pub items: Vec<QuizmasterItem>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuizmasterItem {
    pub name: String,
    pub id: usize,
    pub answer: Answer,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Item {
    pub name: String,
    pub id: usize,
    pub questions: Vec<Question>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Question {
    pub player: String,
    pub id: usize,
    pub question: String,
    pub answer: Answer,
    pub anonymous: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Answer {
    Yes,
    No,
    Maybe,
    Unknown,
}

impl Answer {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "yes" => Some(Self::Yes),
            "no" => Some(Self::No),
            "maybe" => Some(Self::Maybe),
            "unknown" => Some(Self::Unknown),
            _ => None,
        }
    }

    pub const fn next(&self) -> Self {
        match self {
            Self::Maybe => Self::Yes,
            Self::Yes => Self::No,
            Self::No => Self::Unknown,
            Self::Unknown => Self::Maybe,
        }
    }

    pub fn variants() -> Vec<Self> {
        vec![Self::Yes, Self::No, Self::Maybe, Self::Unknown]
    }
}

impl Display for Answer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let answer = match self {
            Self::Yes => "Yes",
            Self::No => "No",
            Self::Maybe => "Maybe",
            Self::Unknown => "Unknown",
        };
        write!(f, "{answer}")
    }
}

static LOBBYS: Lazy<Arc<Mutex<HashMap<String, Lobby>>>> = Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

pub async fn with_lobby<F, T>(lobby_id: &str, f: F) -> Result<T>
where
    F: FnOnce(&Lobby) -> Result<T>,
{
    let lobbys_lock = LOBBYS.lock().await;
    let lobby = lobbys_lock.get(lobby_id).ok_or_else(|| anyhow!("Lobby '{lobby_id}' not found"))?;
    f(lobby)
}

pub struct LobbyInfo {
    pub id: String,
    pub players_count: usize,
}

pub async fn get_lobby_info() -> Result<Vec<LobbyInfo>> {
    let lobbys_lock = LOBBYS.lock().await;
    let mut lobby_infos = Vec::new();
    for (id, lobby) in &lobbys_lock.clone() {
        if !lobby.started {
            lobby_infos.push(LobbyInfo {
                id: id.clone(),
                players_count: lobby.players.len(),
            });
        }
    }
    Ok(lobby_infos)
}

pub async fn with_lobby_mut<F, T>(lobby_id: &str, f: F) -> Result<T>
where
    F: FnOnce(&mut Lobby) -> Result<T>,
{
    let mut lobbys_lock = LOBBYS.lock().await;
    let lobby = lobbys_lock
        .get_mut(lobby_id)
        .ok_or_else(|| anyhow!("Lobby '{lobby_id}' not found"))?;
    f(lobby)
}

pub async fn create_lobby(lobby_id: &str, key_player: String) -> Result<()> {
    let mut lobbys_lock = LOBBYS.lock().await;
    if lobbys_lock.contains_key(lobby_id) {
        return Err(anyhow!("Lobby '{lobby_id}' already exists"));
    }
    lobbys_lock.insert(
        lobby_id.to_string(),
        Lobby {
            started: false,
            elapsed_time: 0.0,
            last_update: get_current_time(),
            key_player: key_player.clone(),
            players: HashMap::new(),
            chat_messages: Vec::new(),
            questions_queue: Vec::new(),
            questions_queue_waiting: true,
            questions_queue_countdown: 0.0,
            quizmaster_queue: Vec::new(),
            items: Vec::new(),
            items_history: Vec::new(),
            items_queue: select_lobby_words(&LobbySettings::default().difficulty, LobbySettings::default().item_count),
            questions_counter: 0,
            settings: LobbySettings::default(),
        },
    );
    println!("Lobby '{lobby_id}' created by key player '{key_player}'");
    Ok(())
}

pub async fn with_player<F, T>(lobby_id: &str, player_name: &str, f: F) -> Result<T>
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
    .await
}

pub async fn with_player_mut<F, T>(lobby_id: &str, player_name: &str, f: F) -> Result<T>
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
    .await
}

pub async fn connect_player(lobby_id: String, player_name: String) -> Result<()> {
    let lobby_id = lobby_id.trim().to_string();
    let player_name = player_name.trim().to_string();
    if lobby_id.len() < 3 || lobby_id.len() > MAX_LOBBY_ID_LENGTH {
        return Err(anyhow!("Lobby ID must be between 3 and {MAX_LOBBY_ID_LENGTH} characters long"));
    }
    if player_name.len() < 3 || player_name.len() > MAX_PLAYER_NAME_LENGTH {
        return Err(anyhow!(
            "Player name must be between 3 and {MAX_PLAYER_NAME_LENGTH} characters long"
        ));
    }
    if !regex::Regex::new(LOBBY_ID_PATTERN).unwrap().is_match(&lobby_id) {
        return Err(anyhow!("Lobby ID must be alphabetic"));
    }
    if !regex::Regex::new(PLAYER_NAME_PATTERN).unwrap().is_match(&player_name) {
        return Err(anyhow!("Player name must be alphabetic"));
    }

    let _ = create_lobby(&lobby_id, player_name.clone()).await;

    with_lobby_mut(&lobby_id, |lobby| {
        if lobby.players.contains_key(&player_name) {
            return Err(anyhow!("Player '{player_name}' is already connected to lobby '{lobby_id}'"));
        }

        lobby.players.entry(player_name.clone()).or_insert(Player {
            name: player_name.clone(),
            last_contact: get_current_time(),
            quizmaster: false,
            score: 0,
            coins: 0,
            messages: Vec::new(),
        });

        println!("Player '{player_name}' connected to lobby '{lobby_id}'");
        Ok(())
    })
    .await
}

pub async fn disconnect_player(lobby_id: String, player_name: String) -> Result<()> {
    with_lobby_mut(&lobby_id, |lobby| {
        lobby.players.remove(&player_name);
        println!("Player '{player_name}' disconnected from lobby '{lobby_id}'");
        Ok(())
    })
    .await
}

pub async fn alter_lobby_settings(lobby_id: String, player_name: String, setting: AlterLobbySetting) -> Result<()> {
    with_lobby_mut(&lobby_id, |lobby| {
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
                        let mut additional_items_needed = item_count - lobby.items_queue.len();
                        let mut unique_new_words = HashSet::new();

                        while additional_items_needed > 0 {
                            let new_words = select_lobby_words(&lobby.settings.difficulty, additional_items_needed);

                            for word in new_words {
                                if !lobby.items_queue.contains(&word) && unique_new_words.insert(word) {
                                    additional_items_needed -= 1;
                                }
                                if additional_items_needed == 0 {
                                    break;
                                }
                            }
                        }

                        lobby.items_queue.extend(unique_new_words);
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
                if !regex::Regex::new(ITEM_NAME_PATTERN).unwrap().is_match(&item) {
                    return Err(anyhow!("Item name must be alphabetic"));
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
                    let mut new_word = select_lobby_words(&lobby.settings.difficulty, 1).pop().unwrap();
                    while lobby.items_queue.contains(&new_word) {
                        new_word = select_lobby_words(&lobby.settings.difficulty, 1).pop().unwrap();
                    }
                    lobby.items_queue[index] = new_word;
                }
            }
            AlterLobbySetting::RefreshAllItems => {
                lobby.items_queue = select_lobby_words(&lobby.settings.difficulty, lobby.settings.item_count);
            }
        }

        Ok(())
    })
    .await
}

pub async fn start_lobby(lobby_id: String, player_name: String) -> Result<()> {
    with_lobby_mut(&lobby_id, |lobby| {
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
            lobby.items_queue = select_lobby_words(&lobby.settings.difficulty, lobby.settings.item_count);
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
    })
    .await
}

pub async fn kick_player(lobby_id: String, player_name: String) -> Result<()> {
    with_lobby_mut(&lobby_id, |lobby| {
        lobby.players.remove(&player_name);
        println!("Lobby '{lobby_id}' player '{player_name}' kicked by key player");
        Ok(())
    })
    .await
}

pub async fn add_chat_message(lobby_id: String, player_name: String, message: String) -> Result<()> {
    if message.len() > MAX_CHAT_LENGTH {
        return Err(anyhow!("Chat message must be less than {MAX_CHAT_LENGTH} characters long"));
    }
    with_lobby_mut(&lobby_id, |lobby| {
        lobby.chat_messages.push(ChatMessage {
            player: player_name.clone(),
            message,
        });
        if lobby.chat_messages.len() > MAX_CHAT_MESSAGES {
            lobby.chat_messages.remove(0);
        }
        Ok(())
    })
    .await
}

pub async fn get_state(lobby_id: String, player_name: String) -> Result<(Lobby, Vec<PlayerMessage>)> {
    with_player_mut(&lobby_id, &player_name, |lobby, player| {
        player.last_contact = get_current_time();
        let messages = player.messages.clone();
        player.messages.clear();
        Ok((lobby, messages))
    })
    .await
}

pub fn get_current_time() -> f64 {
    let now = std::time::SystemTime::now();
    now.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs_f64()
}

pub fn get_time_diff(start: f64) -> f64 {
    get_current_time() - start
}

#[allow(clippy::cast_precision_loss)]
pub async fn lobby_loop() -> Result<()> {
    let mut lobbys_lock = LOBBYS.lock().await;

    // Iterate through lobbys to update or remove
    lobbys_lock.retain(|lobby_id, lobby| {
        let current_time = get_current_time();

        // Remove inactive players and check if key player is active
        lobby.players.retain(|player_id, player| {
            if get_time_diff(player.last_contact) > IDLE_KICK_TIME {
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
                let elapsed_time_update = get_time_diff(lobby.last_update);

                // Distribute coins if the elapsed time has crossed a multiple of COINS_EVERY_X_SECONDS
                let previous_coin_multiple = lobby.elapsed_time / lobby.settings.coin_every_x_seconds as f64;
                let current_coin_multiple = (lobby.elapsed_time + elapsed_time_update) / lobby.settings.coin_every_x_seconds as f64;
                if current_coin_multiple.trunc() > previous_coin_multiple.trunc() {
                    for player in lobby.players.values_mut() {
                        if !player.quizmaster {
                            player.coins += 1;
                            player.messages.push(PlayerMessage::CoinGiven);
                        }
                    }
                }

                // If lobby has a queued question with at least QUESTION_MIN_VOTES votes, tick it down, else reset
                if lobby.questions_queue.iter().any(|q| q.votes >= lobby.settings.question_min_votes) {
                    lobby.questions_queue_waiting = false;
                    lobby.questions_queue_countdown -= elapsed_time_update;
                    if lobby.questions_queue_countdown <= 0.0 {
                        lobby.questions_queue_countdown += lobby.settings.submit_question_every_x_seconds as f64;
                        let lobby_id_clone = lobby_id.clone();
                        tokio::spawn(async move {
                            let _result = ask_top_question(lobby_id_clone).await;
                        });
                    }
                } else {
                    lobby.questions_queue_waiting = true;
                    lobby.questions_queue_countdown = lobby.settings.submit_question_every_x_seconds as f64;
                }

                // Update the elapsed time and last update time for the lobby
                lobby.elapsed_time += elapsed_time_update;
                lobby.last_update = current_time;
            }
            true
        }
    });

    Ok(())
}
