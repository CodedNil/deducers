use crate::{
    backend::openai::query_ai,
    backend::{with_lobby, with_lobby_mut, with_player, with_player_mut, QueuedQuestion},
    MAX_QUESTION_LENGTH,
};
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

pub async fn submit_question(lobby_id: &str, player_name: &str, question: String, masked: bool) -> Result<()> {
    let mut total_cost = 0;
    let mut is_quizmaster = false;
    with_lobby(lobby_id, |lobby| {
        total_cost = if masked {
            lobby.settings.submit_question_cost + lobby.settings.masked_question_cost
        } else {
            lobby.settings.submit_question_cost
        };
        is_quizmaster = lobby.settings.player_controlled;
        Ok(())
    })?;

    with_player(lobby_id, player_name, |lobby, player| {
        if !lobby.started {
            bail!("Lobby not started");
        }
        if player.coins < total_cost {
            bail!("Insufficient coins to submit question");
        }
        if player.quizmaster {
            bail!("Quizmaster cannot engage");
        }

        // Check if question already exists in the queue
        if lobby
            .questions_queue
            .iter()
            .any(|queued_question| queued_question.question == question)
        {
            bail!("Question already exists in queue");
        }
        Ok(())
    })?;

    // Validate the question
    let validate_response = validate_question(&question, !is_quizmaster).await;
    if !validate_response.suitable {
        bail!("{}", validate_response.reasoning);
    }

    // Add question mark if missing, and capitalise first letter
    let question = {
        let mut question = question.trim().to_owned();
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
    with_lobby_mut(lobby_id, |lobby| {
        let player = lobby
            .players
            .get_mut(player_name)
            .ok_or_else(|| anyhow::anyhow!("Player '{player_name}' not found"))?;

        // Deduct coins and add question to queue
        if player.coins < total_cost {
            bail!("Insufficient coins to submit question");
        }
        player.coins -= total_cost;
        lobby.questions_queue.push(QueuedQuestion {
            player: player_name.to_owned(),
            question,
            votes: 0,
            voters: Vec::new(),
            masked,
        });
        Ok(())
    })
}

// Helper function to validate a question
#[derive(Deserialize, Serialize, Debug)]
struct ValidateQuestionResponse {
    suitable: bool,
    reasoning: String,
}

async fn validate_question(question: &str, use_ai: bool) -> ValidateQuestionResponse {
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
            reasoning: reasoning.to_owned(),
        };
    }

    if !use_ai {
        return ValidateQuestionResponse {
            suitable: true,
            reasoning: String::new(),
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
        reasoning: "Failed to validate question".to_owned(),
    }
}

pub fn vote_question(lobby_id: &str, player_name: &str, question: &String) -> Result<()> {
    with_lobby_mut(lobby_id, |lobby| {
        let player = lobby
            .players
            .get_mut(player_name)
            .ok_or_else(|| anyhow::anyhow!("Player '{player_name}' not found"))?;

        if !lobby.started {
            bail!("Lobby not started");
        }
        if player.quizmaster {
            bail!("Quizmaster cannot engage");
        }

        // Check if question exists in the queue
        if let Some(queued_question) = lobby.questions_queue.iter_mut().find(|q| &q.question == question) {
            // Check if player has enough coins
            if player.coins < 1 {
                bail!("Insufficient coins to vote");
            }

            // Deduct coins and increment vote count
            player.coins -= 1;
            queued_question.votes += 1;
            queued_question.voters.push(player_name.to_owned());
            return Ok(());
        }
        Err(anyhow::anyhow!("Question not found in queue"))
    })
}

pub fn convert_score(lobby_id: &str, player_name: &str) -> Result<()> {
    with_player_mut(lobby_id, player_name, |lobby, player| {
        if !lobby.started {
            bail!("Lobby not started");
        }
        if player.score < 1 {
            bail!("Insufficient score to convert");
        }
        if player.quizmaster {
            bail!("Quizmaster cannot engage");
        }

        player.score -= 1;
        player.coins += lobby.settings.score_to_coins_ratio;
        Ok(())
    })
}
