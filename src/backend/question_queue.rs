use crate::{
    backend::openai::query_ai, QueuedQuestion, ANONYMOUS_QUESTION_COST, LOBBYS,
    SCORE_TO_COINS_RATIO, SUBMIT_QUESTION_COST,
};
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct QuestionOptions {
    anonymous: bool,
}

pub async fn player_submit_question(
    lobby_id: String,
    player_name: String,
    question: String,
    question_options: QuestionOptions,
) -> Result<()> {
    let lobbys = LOBBYS
        .get()
        .ok_or_else(|| anyhow::anyhow!("LOBBYS not initialized"))?;
    let mut lobbys_lock = lobbys.lock().await;

    let lobby = lobbys_lock
        .get_mut(&lobby_id)
        .ok_or_else(|| anyhow::anyhow!("Lobby '{lobby_id}' not found"))?;
    let player = lobby
        .players
        .get_mut(&player_name)
        .ok_or_else(|| anyhow::anyhow!("Player '{player_name}' not found"))?;

    // Calculate submission cost and check if player has enough coins
    let total_cost = if question_options.anonymous {
        SUBMIT_QUESTION_COST + ANONYMOUS_QUESTION_COST
    } else {
        SUBMIT_QUESTION_COST
    };
    if player.coins < total_cost {
        return Err(anyhow::anyhow!("Insufficient coins to submit question"));
    }

    // Check if question already exists in the queue
    if lobby
        .questions_queue
        .iter()
        .any(|queued_question| queued_question.question == question)
    {
        return Err(anyhow::anyhow!("Question already exists in queue"));
    }

    // Validate the question
    drop(lobbys_lock);
    let validate_response = is_valid_question(&question).await;
    if !validate_response.suitable {
        return Err(anyhow::anyhow!(
            "Question is not suitable: {}",
            validate_response.reasoning
        ));
    }

    // Reacquire lock and add question to queue
    let mut lobbys_lock = lobbys.lock().await;

    let lobby = lobbys_lock
        .get_mut(&lobby_id)
        .ok_or_else(|| anyhow::anyhow!("Lobby '{lobby_id}' not found"))?;
    let player = lobby
        .players
        .get_mut(&player_name)
        .ok_or_else(|| anyhow::anyhow!("Player '{player_name}' not found"))?;

    // Deduct coins and add question to queue
    if player.coins < total_cost {
        return Err(anyhow::anyhow!("Insufficient coins to submit question"));
    }
    player.coins -= total_cost;
    lobby.questions_queue.push(QueuedQuestion {
        player: player_name.clone(),
        question: validate_response.formatted_question,
        votes: 0,
        anonymous: question_options.anonymous,
    });
    drop(lobbys_lock);
    Ok(())
}

// Helper function to validate a question
#[derive(Deserialize, Serialize, Debug)]
struct ValidateQuestionResponse {
    suitable: bool,
    formatted_question: String,
    reasoning: String,
}

async fn is_valid_question(question: &str) -> ValidateQuestionResponse {
    let trimmed = question.trim();
    if trimmed.is_empty() {
        return ValidateQuestionResponse {
            suitable: false,
            formatted_question: question.to_string(),
            reasoning: "Question is empty".to_string(),
        };
    }

    // Query with OpenAI API
    let response = query_ai(
        &format!("u:Check '{trimmed}' for suitability in a 20 Questions game, format it, and return a compact one line JSON with reasoning (concise up to 4 word explanation for suitability, is it a question with clear yes/no/maybe answerability, is it relevant to identifying an item), formatted_question (the input question with first letter capitalized and with a question mark), suitable (bool, if uncertain err on allowing the question unless it clearly fails criteria), British English"),
        100, 1.0
    ).await;
    if let Ok(message) = response {
        // Parse response
        if let Ok(validate_response) = serde_json::from_str::<ValidateQuestionResponse>(&message) {
            return validate_response;
        }
    }

    ValidateQuestionResponse {
        suitable: false,
        formatted_question: question.to_string(),
        reasoning: "Failed to validate question".to_string(),
    }
}

pub async fn player_vote_question(
    lobby_id: String,
    player_name: String,
    question: String,
) -> Result<()> {
    let lobbys = LOBBYS
        .get()
        .ok_or_else(|| anyhow::anyhow!("LOBBYS not initialized"))?;
    let mut lobbys_lock = lobbys.lock().await;

    let lobby = lobbys_lock
        .get_mut(&lobby_id)
        .ok_or_else(|| anyhow::anyhow!("Lobby '{lobby_id}' not found"))?;
    let player = lobby
        .players
        .get_mut(&player_name)
        .ok_or_else(|| anyhow::anyhow!("Player '{player_name}' not found"))?;

    // Check if question exists in the queue
    if let Some(queued_question) = lobby
        .questions_queue
        .iter_mut()
        .find(|q| q.question == question)
    {
        // Check if player has enough coins
        if player.coins < 1 {
            return Err(anyhow::anyhow!("Insufficient coins to vote"));
        }

        // Deduct coins and increment vote count
        player.coins -= 1;
        queued_question.votes += 1;
        drop(lobbys_lock);
        return Ok(());
    }
    drop(lobbys_lock);
    Err(anyhow::anyhow!("Question not found in queue"))
}

pub async fn player_convert_score(lobby_id: String, player_name: String) -> Result<()> {
    let lobbys = LOBBYS
        .get()
        .ok_or_else(|| anyhow::anyhow!("LOBBYS not initialized"))?;
    let mut lobbys_lock = lobbys.lock().await;

    let lobby = lobbys_lock
        .get_mut(&lobby_id)
        .ok_or_else(|| anyhow::anyhow!("Lobby '{lobby_id}' not found"))?;
    let player = lobby
        .players
        .get_mut(&player_name)
        .ok_or_else(|| anyhow::anyhow!("Player '{player_name}' not found"))?;

    // Check if player has enough score
    if player.score < 1 {
        return Err(anyhow::anyhow!("Insufficient score to convert"));
    }

    // Deduct score and give coins
    player.score -= 1;
    player.coins += SCORE_TO_COINS_RATIO;
    drop(lobbys_lock);
    Ok(())
}
