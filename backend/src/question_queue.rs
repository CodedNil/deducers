use crate::{
    QueuedQuestion, ServerStorage, ANONYMOUS_QUESTION_COST, SUBMIT_QUESTION_COST,
    VOTE_QUESTION_COST,
};
use axum::{extract::Path, http::StatusCode, response::IntoResponse, Extension};
use serde::Deserialize;

#[derive(Deserialize)]
struct QuestionOptions {
    anonymous: bool,
}

pub async fn player_submit_question(
    Path((server_id, player_name, question, options)): Path<(String, String, String, String)>,
    Extension(servers): Extension<ServerStorage>,
) -> impl IntoResponse {
    let mut servers = servers.lock().await;

    if let Some(server) = servers.get_mut(&server_id) {
        if let Some(player) = server.players.get_mut(&player_name) {
            // Attempt to parse options JSON
            let Ok(question_options) = serde_json::from_str::<QuestionOptions>(&options) else {
                return (
                    StatusCode::BAD_REQUEST,
                    "Invalid options format".to_string(),
                );
            };

            // Calculate submission cost and check if player has enough coins
            let total_cost = if question_options.anonymous {
                SUBMIT_QUESTION_COST + ANONYMOUS_QUESTION_COST
            } else {
                SUBMIT_QUESTION_COST
            };
            if player.coins < total_cost {
                return (
                    StatusCode::BAD_REQUEST,
                    "Insufficient coins to submit question".to_string(),
                );
            }

            // Check if question already exists in the queue
            if server
                .questions_queue
                .iter()
                .any(|queued_question| queued_question.question == question)
            {
                return (
                    StatusCode::BAD_REQUEST,
                    "Question already exists in queue".to_string(),
                );
            }

            // Validate the question
            if !is_valid_question(&question) {
                return (
                    StatusCode::BAD_REQUEST,
                    "Invalid question format".to_string(),
                );
            }

            // Deduct coins and add question to queue
            player.coins -= total_cost;
            server.questions_queue.push(QueuedQuestion {
                player: player_name.clone(),
                question,
                votes: 0,
                anonymous: question_options.anonymous,
            });
            (
                StatusCode::OK,
                "Question submitted successfully".to_string(),
            )
        } else {
            (
                StatusCode::NOT_FOUND,
                "Player not found in server".to_string(),
            )
        }
    } else {
        (StatusCode::NOT_FOUND, "Server not found".to_string())
    }
}

// Helper function to validate a question
fn is_valid_question(question: &str) -> bool {
    // Implement actual question validation logic here
    !question.trim().is_empty()
}

pub async fn player_vote_question(
    Path((server_id, player_name, question)): Path<(String, String, String)>,
    Extension(servers): Extension<ServerStorage>,
) -> impl IntoResponse {
    let mut servers = servers.lock().await;

    if let Some(server) = servers.get_mut(&server_id) {
        if let Some(player) = server.players.get_mut(&player_name) {
            // Check if question exists in the queue
            if let Some(queued_question) = server
                .questions_queue
                .iter_mut()
                .find(|q| q.question == question)
            {
                // Check if player has enough coins
                if player.coins < VOTE_QUESTION_COST {
                    return (
                        StatusCode::BAD_REQUEST,
                        "Insufficient coins to upvote question".to_string(),
                    );
                }

                // Deduct coins and increment vote count
                player.coins -= VOTE_QUESTION_COST;
                queued_question.votes += 1;
                (StatusCode::OK, "Question upvoted successfully".to_string())
            } else {
                (
                    StatusCode::BAD_REQUEST,
                    "Question not found in queue".to_string(),
                )
            }
        } else {
            (
                StatusCode::NOT_FOUND,
                "Player not found in server".to_string(),
            )
        }
    } else {
        (StatusCode::NOT_FOUND, "Server not found".to_string())
    }
}
