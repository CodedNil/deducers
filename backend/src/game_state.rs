use crate::{Answer, Server, ServerStorage};
use axum::{extract::Path, http::StatusCode, response::IntoResponse, Extension, Json};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
struct Player {
    name: String,
    score: i32,
    coins: Option<i32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct QueuedQuestion {
    player: String,
    question: Option<String>,
    anonymous: bool,
    votes: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Item {
    id: u32,
    questions: Vec<Question>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Question {
    player: String,
    pub id: u32,
    question: Option<String>,
    answer: Answer,
    anonymous: bool,
}

#[allow(clippy::large_enum_variant)]
#[derive(Serialize)]
pub enum Response {
    ServerState(ServerMinimal),
    Error(String),
}

pub async fn get_state(
    Path((server_id, player_name)): Path<(String, String)>,
    Extension(servers): Extension<ServerStorage>,
) -> impl IntoResponse {
    let mut servers_locked = servers.lock().await;

    if let Some(server) = servers_locked.get_mut(&server_id) {
        if let Some(player) = server.players.get_mut(&player_name) {
            // Update last contact time for the player
            player.last_contact = Utc::now();

            let minimal_server = convert_to_minimal(server, &player_name);

            // Return the entire state of the server
            (StatusCode::OK, Json(Response::ServerState(minimal_server)))
        } else {
            (
                StatusCode::NOT_FOUND,
                Json(Response::Error(format!(
                    "Player '{player_name}' not found in server '{server_id}'"
                ))),
            )
        }
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(Response::Error(format!("Server '{server_id}' not found"))),
        )
    }
}

// Convert server to minimal server
pub fn convert_to_minimal(server: &Server, player_name: &String) -> ServerMinimal {
    // Convert questions queue removing questions if anonymous
    let questions_queue = server
        .questions_queue
        .iter()
        .map(|queued_question| {
            let question_value =
                if queued_question.anonymous && &queued_question.player != player_name {
                    None
                } else {
                    Some(queued_question.question.clone())
                };

            QueuedQuestion {
                player: queued_question.player.clone(),
                question: question_value,
                anonymous: queued_question.anonymous,
                votes: queued_question.votes,
            }
        })
        .collect();

    // Convert items removing questions if anonymous
    let items = server
        .items
        .iter()
        .map(|item| Item {
            id: item.id,
            questions: item
                .questions
                .iter()
                .map(|question| {
                    let question_value = if question.anonymous && &question.player != player_name {
                        None
                    } else {
                        Some(question.question.clone())
                    };

                    Question {
                        player: question.player.clone(),
                        id: question.id,
                        question: question_value,
                        answer: question.answer.clone(),
                        anonymous: question.anonymous,
                    }
                })
                .collect(),
        })
        .collect();

    // Convert players removing coins for other players
    let players = server
        .players
        .iter()
        .map(|(other_player_name, player)| {
            (other_player_name.clone(), {
                let coins_value = if other_player_name == player_name {
                    Some(player.coins)
                } else {
                    None
                };
                Player {
                    name: player.name.clone(),
                    score: player.score,
                    coins: coins_value,
                }
            })
        })
        .collect();

    ServerMinimal {
        id: server.id.clone(),
        started: server.started,
        elapsed_time: server.elapsed_time,
        key_player: server.key_player.clone(),
        players,
        questions_queue,
        items,
    }
}
