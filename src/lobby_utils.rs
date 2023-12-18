#![allow(clippy::missing_errors_doc, clippy::future_not_send, clippy::significant_drop_tightening)]
use crate::{
    backend::items::{add_item_to_queue, ask_top_question},
    COINS_EVERY_X_SECONDS, IDLE_KICK_TIME, QUESTION_MIN_VOTES, SUBMIT_QUESTION_EVERY_X_SECONDS,
};
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

static LOBBYS: OnceLock<Arc<Mutex<HashMap<String, Lobby>>>> = OnceLock::new();

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

pub async fn create_lobby(lobby_id: &str, key_player: String) -> Result<()> {
    let lobbys = LOBBYS.get().ok_or_else(|| anyhow!("LOBBYS not initialized"))?;
    let mut lobbys_lock = lobbys.lock().await;
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
            questions_queue: Vec::new(),
            questions_queue_waiting: true,
            questions_queue_countdown: SUBMIT_QUESTION_EVERY_X_SECONDS,
            items: Vec::new(),
            items_history: Vec::new(),
            items_queue: Vec::new(),
            last_add_to_queue: -10.0,
            questions_counter: 0,
        },
    );
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

pub fn get_current_time() -> f64 {
    let now = std::time::SystemTime::now();
    now.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs_f64()
}

pub fn get_time_diff(start: f64) -> f64 {
    get_current_time() - start
}

pub async fn lobby_loop() {
    let lobbys = LOBBYS.get().unwrap();
    let mut lobbys_lock = lobbys.lock().await;

    // Iterate through lobbys to update or remove
    lobbys_lock.retain(|lobby_id, lobby| {
        let current_time = get_current_time();

        // Remove inactive players and check if key player is active
        lobby.players.retain(|player_id, player| {
            if get_time_diff(player.last_contact) > IDLE_KICK_TIME {
                // Log player kicking due to inactivity
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
            // Add more items to the lobby's item queue if it's low
            let time_since_last_add_to_queue = get_time_diff(lobby.last_add_to_queue);
            if lobby.items_queue.len() < 3 && time_since_last_add_to_queue > 10.0 {
                lobby.last_add_to_queue = current_time;
                let lobby_id_clone = lobby_id.clone();
                let history_clone = lobby.items_history.clone();
                tokio::spawn(async move {
                    add_item_to_queue(lobby_id_clone, history_clone, 0).await;
                });
            }

            // Update lobby state if lobby is started
            if lobby.started {
                let elapsed_time_update = get_time_diff(lobby.last_update);

                // Distribute coins if the elapsed time has crossed a multiple of COINS_EVERY_X_SECONDS
                let previous_coin_multiple = lobby.elapsed_time / COINS_EVERY_X_SECONDS;
                let current_coin_multiple = (lobby.elapsed_time + elapsed_time_update) / COINS_EVERY_X_SECONDS;
                if current_coin_multiple.trunc() > previous_coin_multiple.trunc() {
                    for player in lobby.players.values_mut() {
                        player.coins += 1;
                        player.messages.push(PlayerMessage::CoinGiven);
                    }
                }

                // If lobby has a queued question with at least QUESTION_MIN_VOTES votes, tick it down, else reset
                if lobby.questions_queue.iter().any(|q| q.votes >= QUESTION_MIN_VOTES) {
                    lobby.questions_queue_waiting = false;
                    lobby.questions_queue_countdown -= elapsed_time_update;
                    if lobby.questions_queue_countdown <= 0.0 {
                        lobby.questions_queue_countdown += SUBMIT_QUESTION_EVERY_X_SECONDS;
                        let lobby_id_clone = lobby_id.clone();
                        tokio::spawn(async move {
                            let _result = ask_top_question(lobby_id_clone).await;
                        });
                    }
                } else {
                    lobby.questions_queue_waiting = true;
                    lobby.questions_queue_countdown = SUBMIT_QUESTION_EVERY_X_SECONDS;
                }

                // Update the elapsed time and last update time for the lobby
                lobby.elapsed_time += elapsed_time_update;
                lobby.last_update = current_time;
            }
            true
        }
    });
}
