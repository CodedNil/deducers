use crate::{
    backend::openai::query_ai, with_lobby_mut, with_player, with_player_mut, QueuedQuestion, ANONYMOUS_QUESTION_COST, MAX_QUESTION_LENGTH,
    SCORE_TO_COINS_RATIO, SUBMIT_QUESTION_COST,
};
use anyhow::Result;
use serde::{Deserialize, Serialize};

pub async fn player_submit_question(lobby_id: String, player_name: String, question: String, anonymous: bool) -> Result<()> {
    let total_cost = if anonymous {
        SUBMIT_QUESTION_COST + ANONYMOUS_QUESTION_COST
    } else {
        SUBMIT_QUESTION_COST
    };

    with_player(&lobby_id, &player_name, |lobby, player| {
        if !lobby.started {
            return Err(anyhow::anyhow!("Lobby not started"));
        }
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
        Ok(())
    })
    .await?;

    // Validate the question
    let validate_response = is_valid_question(&question).await;
    if !validate_response.suitable {
        return Err(anyhow::anyhow!("{}", validate_response.reasoning));
    }

    // Add question mark if missing, and capitalise first letter
    let question = {
        let mut question = question.trim().to_string();
        if !question.ends_with('?') {
            question.push('?');
        }
        let mut chars = question.chars();
        if let Some(first_char) = chars.next() {
            question = first_char.to_uppercase().to_string() + chars.as_str();
        }
        question
    };

    // Reacquire lock and add question to queue
    with_lobby_mut(&lobby_id, |lobby| {
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
            question,
            votes: 0,
            anonymous,
        });
        Ok(())
    })
    .await
}

// Helper function to validate a question
#[derive(Deserialize, Serialize, Debug)]
struct ValidateQuestionResponse {
    suitable: bool,
    reasoning: String,
}

async fn is_valid_question(question: &str) -> ValidateQuestionResponse {
    let trimmed = question.trim();
    let length = trimmed.len();

    let (suitable, reasoning) = match length {
        0 => (false, "Question is empty"),
        1..=4 => (false, "Question is too short"),
        MAX_QUESTION_LENGTH.. => (false, "Question is too long"),
        _ => (true, ""),
    };
    if !suitable {
        return ValidateQuestionResponse {
            suitable,
            reasoning: reasoning.to_string(),
        };
    }

    // Query with OpenAI API
    let response = query_ai(
        &format!("u:Check '{trimmed}' for suitability in a 20 Questions game, return a compact one line JSON with two keys reasoning and suitable, reasoning (concise up to 4 word explanation for suitability, is it a question with clear yes/no/maybe answerability, is it relevant to identifying an item), suitable (bool, if uncertain err on allowing the question unless it clearly fails criteria), British English"),
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
        reasoning: "Failed to validate question".to_string(),
    }
}

pub async fn player_vote_question(lobby_id: String, player_name: String, question: String) -> Result<()> {
    with_lobby_mut(&lobby_id, |lobby| {
        let player = lobby
            .players
            .get_mut(&player_name)
            .ok_or_else(|| anyhow::anyhow!("Player '{player_name}' not found"))?;

        if !lobby.started {
            return Err(anyhow::anyhow!("Lobby not started"));
        }

        // Check if question exists in the queue
        if let Some(queued_question) = lobby.questions_queue.iter_mut().find(|q| q.question == question) {
            // Check if player has enough coins
            if player.coins < 1 {
                return Err(anyhow::anyhow!("Insufficient coins to vote"));
            }

            // Deduct coins and increment vote count
            player.coins -= 1;
            queued_question.votes += 1;
            return Ok(());
        }
        Err(anyhow::anyhow!("Question not found in queue"))
    })
    .await
}

pub async fn player_convert_score(lobby_id: String, player_name: String) -> Result<()> {
    with_player_mut(&lobby_id, &player_name, |lobby, player| {
        if !lobby.started {
            return Err(anyhow::anyhow!("Lobby not started"));
        }

        // Check if player has enough score
        if player.score < 1 {
            return Err(anyhow::anyhow!("Insufficient score to convert"));
        }

        // Deduct score and give coins
        player.score -= 1;
        player.coins += SCORE_TO_COINS_RATIO;
        Ok(())
    })
    .await
}
