use crate::{
    backend::{alert_popup, openai::query_ai, with_lobby_mut, with_player, with_player_mut, QueuedQuestion},
    MAX_QUESTION_LENGTH,
};
use anyhow::{anyhow, bail, ensure, Result};
use serde::{Deserialize, Serialize};

pub async fn submit_question(lobby_id: &str, player_name: &str, question: String, masked: bool) -> Result<()> {
    let mut total_cost = 0;
    let mut has_quizmaster = false;
    with_player(lobby_id, player_name, |lobby, player| {
        ensure!(lobby.started, "Lobby not started");
        total_cost = if masked {
            lobby.settings.submit_question_cost + lobby.settings.masked_question_cost
        } else {
            lobby.settings.submit_question_cost
        };
        ensure!(player.coins >= total_cost, "Insufficient coins to submit question");
        has_quizmaster = lobby.settings.player_controlled;
        ensure!(!player.quizmaster, "Quizmaster cannot engage");
        if lobby.questions_queue.iter().any(|q| q.question == question) {
            bail!("Question already exists in queue");
        }
        Ok(())
    })?;

    // Validate the question
    let validate_response = validate_question(&question, !has_quizmaster).await;
    ensure!(validate_response.suitable, validate_response.reasoning);

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
            .ok_or_else(|| anyhow!("Player '{player_name}' not found"))?;

        // Deduct coins and add question to queue
        ensure!(player.coins >= total_cost, "Insufficient coins to submit question");
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
#[derive(Deserialize, Serialize)]
struct ValidateQuestionResponse {
    suitable: bool,
    reasoning: String,
}

async fn validate_question(question: &str, use_ai: bool) -> ValidateQuestionResponse {
    let trimmed = question.trim();
    let (suitable, reasoning) = match trimmed.len() {
        0 => (false, "Question is empty"),
        1..=4 => (false, "Question is too short"),
        MAX_QUESTION_LENGTH.. => (false, "Question is too long"),
        _ => (true, ""),
    };
    if !suitable {
        return ValidateQuestionResponse {
            suitable: false,
            reasoning: reasoning.to_owned(),
        };
    }
    if !use_ai {
        return ValidateQuestionResponse {
            suitable: true,
            reasoning: String::new(),
        };
    }

    let response = query_ai(
        &format!("u:Check '{trimmed}' for suitability in a 20 Questions game, return a compact one line JSON with two keys reasoning and suitable, reasoning (concise up to 4 word explanation for suitability, is it a question with clear yes/no/maybe answerability, is it relevant to identifying an item), suitable (bool, if uncertain err on allowing the question unless it clearly fails criteria), British English"),
        100, 1.0
    ).await;
    if let Ok(message) = response {
        if let Ok(validate_response) = serde_json::from_str::<ValidateQuestionResponse>(&message) {
            return validate_response;
        }
    }
    ValidateQuestionResponse {
        suitable: false,
        reasoning: "Failed to validate question".to_owned(),
    }
}

pub fn vote_question(lobby_id: &str, player_name: &str, question: &String) {
    let result = with_lobby_mut(lobby_id, |lobby| {
        let player = lobby
            .players
            .get_mut(player_name)
            .ok_or_else(|| anyhow!("Player '{player_name}' not found"))?;
        ensure!(lobby.started, "Lobby not started");
        ensure!(!player.quizmaster, "Quizmaster cannot engage");
        ensure!(player.coins >= 1, "Insufficient coins");

        let queued_question = lobby
            .questions_queue
            .iter_mut()
            .find(|q| &q.question == question)
            .ok_or_else(|| anyhow!("Question not found in queue"))?;

        player.coins -= 1;
        queued_question.votes += 1;
        queued_question.voters.push(player_name.to_owned());
        Ok(())
    });
    if let Err(error) = result {
        alert_popup(lobby_id, player_name, &format!("Vote rejected {error}"));
    }
}

pub fn convert_score(lobby_id: &str, player_name: &str) {
    let result = with_player_mut(lobby_id, player_name, |lobby, player| {
        ensure!(lobby.started, "Lobby not started");
        ensure!(!player.quizmaster, "Quizmaster cannot engage");
        ensure!(player.score >= 1, "Insufficient score");
        player.score -= 1;
        player.coins += lobby.settings.score_to_coins_ratio;
        Ok(())
    });
    if let Err(error) = result {
        alert_popup(lobby_id, player_name, &format!("{error}"));
    }
}
