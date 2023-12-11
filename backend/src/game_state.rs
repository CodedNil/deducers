use crate::{Answer, Server, ServerStorage};
use axum::{extract::Path, http::StatusCode, response::IntoResponse, Extension, Json};
use serde::Serialize;
use std::collections::HashMap;
use tokio::time::Instant;

#[derive(Clone, Debug, Serialize)]
pub struct ServerMinimal {
    id: String,
    started: bool,
    elapsed_time: f64,
    key_player: String,
    players: HashMap<String, Player>,
    questions_queue: Vec<QueuedQuestion>,
    items: Vec<Item>,
}

#[derive(Clone, Debug, Serialize)]
struct Player {
    name: String,
    score: usize,
    coins: Option<usize>,
}

#[derive(Clone, Debug, Serialize)]
struct QueuedQuestion {
    player: String,
    question: Option<String>,
    anonymous: bool,
    votes: usize,
}

#[derive(Clone, Debug, Serialize)]
struct Item {
    id: usize,
    questions: Vec<Question>,
}

#[derive(Clone, Debug, Serialize)]
struct Question {
    player: String,
    id: usize,
    question: Option<String>,
    answer: Answer,
    anonymous: bool,
}

#[derive(Serialize)]
pub enum Response {
    ServerState(ServerMinimal),
    Error(String),
}

pub async fn get_state(Path((server_id, player_name)): Path<(String, String)>, Extension(servers): Extension<ServerStorage>) -> impl IntoResponse {
    let mut servers_lock = servers.lock().await;
    let Some(server) = servers_lock.get_mut(&server_id) else {
        return (StatusCode::NOT_FOUND, Json(Response::Error("Server not found".into())));
    };
    let Some(player) = server.players.get_mut(&player_name) else {
        return (StatusCode::NOT_FOUND, Json(Response::Error("Player not found in server".into())));
    };

    // Update last contact time for the player and convert to minimal server
    player.last_contact = Instant::now();
    let minimal_server = convert_to_minimal(server, &player_name);
    drop(servers_lock);

    // Return the entire state of the server
    (StatusCode::OK, Json(Response::ServerState(minimal_server)))
}

// Convert server to minimal server
pub fn convert_to_minimal(server: &Server, player_name: &String) -> ServerMinimal {
    // Convert questions queue removing questions if anonymous
    let questions_queue = server
        .questions_queue
        .iter()
        .map(|queued_question| {
            let question_value = if queued_question.anonymous && &queued_question.player != player_name {
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
                let coins_value = if other_player_name == player_name { Some(player.coins) } else { None };
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
