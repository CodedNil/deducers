use crate::{openai::query, QueuedQuestion, ServerStorage, ANONYMOUS_QUESTION_COST, SCORE_TO_COINS_RATIO, SUBMIT_QUESTION_COST, VOTE_QUESTION_COST};
use axum::{extract::Path, http::StatusCode, response::IntoResponse, Extension};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct QuestionOptions {
    anonymous: bool,
}

pub async fn player_submit_question(
    Path((server_id, player_name, question, options)): Path<(String, String, String, String)>,
    Extension(servers): Extension<ServerStorage>,
) -> impl IntoResponse {
    let mut servers_lock = servers.lock().await;
    let Some(server) = servers_lock.get_mut(&server_id) else {
        return (StatusCode::NOT_FOUND, "Server not found".to_string());
    };
    let Some(player) = server.players.get_mut(&player_name) else {
        return (StatusCode::NOT_FOUND, "Player not found in server".to_string());
    };

    // Attempt to parse options JSON
    let Ok(question_options) = serde_json::from_str::<QuestionOptions>(&options) else {
        return (StatusCode::BAD_REQUEST, "Invalid options format".to_string());
    };

    // Calculate submission cost and check if player has enough coins
    let total_cost = if question_options.anonymous {
        SUBMIT_QUESTION_COST + ANONYMOUS_QUESTION_COST
    } else {
        SUBMIT_QUESTION_COST
    };
    if player.coins < total_cost {
        return (StatusCode::BAD_REQUEST, "Insufficient coins to submit question".to_string());
    }

    // Check if question already exists in the queue
    if server.questions_queue.iter().any(|queued_question| queued_question.question == question) {
        return (StatusCode::BAD_REQUEST, "Question already exists in queue".to_string());
    }

    // Validate the question
    drop(servers_lock);
    let validate_response = is_valid_question(&question).await;
    if !validate_response.is_suitable {
        return (
            StatusCode::BAD_REQUEST,
            format!("Question is not suitable: {}", validate_response.suitable_reasoning),
        );
    }

    // Reacquire lock and add question to queue
    let mut servers_lock = servers.lock().await;
    let Some(server) = servers_lock.get_mut(&server_id) else {
        return (StatusCode::NOT_FOUND, "Server not found".to_string());
    };
    let Some(player) = server.players.get_mut(&player_name) else {
        return (StatusCode::NOT_FOUND, "Player not found in server".to_string());
    };

    // Deduct coins and add question to queue
    if player.coins < total_cost {
        return (StatusCode::BAD_REQUEST, "Insufficient coins to submit question".to_string());
    }
    player.coins -= total_cost;
    server.questions_queue.push(QueuedQuestion {
        player: player_name.clone(),
        question: validate_response.formatted_question,
        votes: 0,
        anonymous: question_options.anonymous,
    });
    drop(servers_lock);
    (StatusCode::OK, "Question submitted successfully".to_string())
}

// Helper function to validate a question
#[derive(Deserialize, Serialize, Debug)]
struct ValidateQuestionResponse {
    is_suitable: bool,
    formatted_question: String,
    suitable_reasoning: String,
}

async fn is_valid_question(question: &str) -> ValidateQuestionResponse {
    let trimmed = question.trim();
    if trimmed.is_empty() {
        return ValidateQuestionResponse {
            is_suitable: false,
            formatted_question: question.to_string(),
            suitable_reasoning: "Question is empty".to_string(),
        };
    }
    println!("Question to validate: {question}");

    // Query with OpenAI API
    let response = query(
        &format!("u:Check '{trimmed}' for suitability in a 20 Questions game, format it, and return a compact one line JSON with suitable_reasoning (up to 6 word explanation for suitability, is it a question with clear yes/no/maybe answerability, is it relevant to identifying an item), formatted_question (string, the input question capitalized and with a question mark), is_suitable (bool, if uncertain err on allowing the question unless it clearly fails criteria), British English"),
        100, 1.0
    ).await;
    println!("Response: {response:?}");
    if let Ok(message) = response {
        // Parse response
        if let Ok(validate_response) = serde_json::from_str::<ValidateQuestionResponse>(&message) {
            return validate_response;
        }
    }

    ValidateQuestionResponse {
        is_suitable: false,
        formatted_question: question.to_string(),
        suitable_reasoning: "Failed to validate question".to_string(),
    }
}

pub async fn player_vote_question(
    Path((server_id, player_name, question)): Path<(String, String, String)>,
    Extension(servers): Extension<ServerStorage>,
) -> impl IntoResponse {
    let mut servers_lock = servers.lock().await;
    let Some(server) = servers_lock.get_mut(&server_id) else {
        return (StatusCode::NOT_FOUND, "Server not found".to_string());
    };
    let Some(player) = server.players.get_mut(&player_name) else {
        return (StatusCode::NOT_FOUND, "Player not found in server".to_string());
    };

    // Check if question exists in the queue
    if let Some(queued_question) = server.questions_queue.iter_mut().find(|q| q.question == question) {
        // Check if player has enough coins
        if player.coins < VOTE_QUESTION_COST {
            return (StatusCode::BAD_REQUEST, "Insufficient coins to upvote question".to_string());
        }

        // Deduct coins and increment vote count
        player.coins -= VOTE_QUESTION_COST;
        queued_question.votes += 1;
        drop(servers_lock);
        return (StatusCode::OK, "Question upvoted successfully".to_string());
    }
    drop(servers_lock);
    (StatusCode::BAD_REQUEST, "Question not found in queue".to_string())
}

pub async fn player_convert_score(Path((server_id, player_name)): Path<(String, String)>, Extension(servers): Extension<ServerStorage>) -> impl IntoResponse {
    let mut servers_lock = servers.lock().await;
    let Some(server) = servers_lock.get_mut(&server_id) else {
        return (StatusCode::NOT_FOUND, "Server not found".to_string());
    };
    let Some(player) = server.players.get_mut(&player_name) else {
        return (StatusCode::NOT_FOUND, "Player not found in server".to_string());
    };

    // Check if player has enough score
    if player.score < 1 {
        return (StatusCode::BAD_REQUEST, "Insufficient score to convert".to_string());
    }

    // Deduct score and give coins
    player.score -= 1;
    player.coins += SCORE_TO_COINS_RATIO;
    drop(servers_lock);
    (StatusCode::OK, "Score converted successfully".to_string())
}
