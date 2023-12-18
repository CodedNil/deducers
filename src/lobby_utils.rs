#![allow(clippy::missing_errors_doc, clippy::future_not_send, clippy::significant_drop_tightening)]
use anyhow::{anyhow, Result};
use std::{
    collections::HashMap,
    sync::{Arc, OnceLock},
};
use tokio::sync::Mutex;

#[derive(Clone, Debug)]
pub struct Lobby {
    pub started: bool,
    pub elapsed_time: f64,
    pub last_update: f64,
    pub key_player: String,
    pub players: HashMap<String, Player>,
    pub questions_queue: Vec<QueuedQuestion>,
    pub questions_queue_waiting: bool,
    pub questions_queue_countdown: f64,
    pub items: Vec<Item>,
    pub items_history: Vec<String>,
    pub items_queue: Vec<String>,
    pub last_add_to_queue: f64,
    pub questions_counter: usize,
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
}

#[derive(Clone, Debug)]
pub struct Player {
    pub name: String,
    pub last_contact: f64,
    pub score: usize,
    pub coins: usize,
    pub messages: Vec<PlayerMessage>,
}

#[derive(Clone, Debug)]
pub struct QueuedQuestion {
    pub player: String,
    pub question: String,
    pub anonymous: bool,
    pub votes: usize,
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Answer {
    Yes,
    No,
    Maybe,
}

pub static LOBBYS: OnceLock<Arc<Mutex<HashMap<String, Lobby>>>> = OnceLock::new();

pub fn init() {
    LOBBYS.get_or_init(|| Arc::new(Mutex::new(HashMap::new())));
}

pub async fn with_lobby<F, T>(lobby_id: &str, f: F) -> Result<T>
where
    F: FnOnce(&Lobby) -> Result<T>,
{
    let lobbys = LOBBYS.get().ok_or_else(|| anyhow!("LOBBYS not initialized"))?;
    let lobbys_lock = lobbys.lock().await;
    let lobby = lobbys_lock.get(lobby_id).ok_or_else(|| anyhow!("Lobby '{lobby_id}' not found"))?;
    f(lobby)
}

pub async fn with_lobby_mut<F, T>(lobby_id: &str, f: F) -> Result<T>
where
    F: FnOnce(&mut Lobby) -> Result<T>,
{
    let lobbys = LOBBYS.get().ok_or_else(|| anyhow!("LOBBYS not initialized"))?;
    let mut lobbys_lock = lobbys.lock().await;
    let lobby = lobbys_lock
        .get_mut(lobby_id)
        .ok_or_else(|| anyhow!("Lobby '{lobby_id}' not found"))?;
    f(lobby)
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

pub fn get_current_time() -> f64 {
    let now = std::time::SystemTime::now();
    now.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs_f64()
}

pub fn get_time_diff(start: f64) -> f64 {
    get_current_time() - start
}
