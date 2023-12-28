use crate::{
    backend::openai::query_ai,
    lobby_utils::{
        with_lobby_mut, with_player_mut, Answer, Item, Lobby, PlayerMessage, Question, QueuedQuestionQuizmaster, QuizmasterItem,
    },
};
use anyhow::{bail, Result};
use futures::future::join_all;
use serde::Deserialize;
use std::collections::HashMap;

pub fn add_item_to_lobby(lobby: &mut Lobby) {
    if !lobby.started {
        return;
    }
    // Get first item in queue, if no items return
    if lobby.items_queue.is_empty() {
        return;
    }
    let item_name = lobby.items_queue.remove(0);

    // Add item to lobby
    println!("Adding item '{item_name}' to lobby");
    lobby.items.push(Item {
        name: item_name.clone(),
        id: lobby.items_history.len() + 1,
        questions: Vec::new(),
    });
    lobby.items_history.push(item_name);
    // Send message to all players of item added
    for player in lobby.players.values_mut() {
        player.messages.push(PlayerMessage::ItemAdded);
    }
}

#[derive(Deserialize, Debug)]
struct AskQuestionResponse {
    answers: Vec<String>,
}

#[allow(clippy::too_many_lines, clippy::cast_precision_loss)]
pub async fn ask_top_question(lobby_id: &str) -> Result<()> {
    let (mut question_text, mut question_player, mut question_masked) = (String::new(), String::new(), false);
    let mut question_voters = Vec::new();
    let mut items = Vec::new();
    let mut is_quizmaster = false;

    with_lobby_mut(lobby_id, |lobby| {
        let question = lobby
            .questions_queue
            .iter()
            .max_by_key(|question| question.votes)
            .ok_or_else(|| anyhow::anyhow!("No questions in queue"))?;

        if question.votes < lobby.settings.question_min_votes {
            bail!("Question needs at least {} votes", lobby.settings.question_min_votes);
        }

        question_text = question.question.clone();
        question_player = question.player.clone();
        question_masked = question.masked;
        question_voters = question.voters.clone();
        items = lobby.items.clone();

        // Remove question from queue
        lobby.questions_queue.retain(|q| q.question != question_text);

        // Reset queue waiting if needed
        if !lobby.question_queue_active() {
            lobby.questions_queue_countdown = lobby.settings.submit_question_every_x_seconds as f64;
        }

        is_quizmaster = lobby.settings.player_controlled;

        Ok(())
    })?;

    // If quizmaster end here and add to the quizmasters queue
    if is_quizmaster {
        let items_list = items
            .iter()
            .map(|item| QuizmasterItem {
                id: item.id,
                name: item.name.clone(),
                answer: Answer::Maybe,
            })
            .collect();
        with_lobby_mut(lobby_id, |lobby| {
            lobby.quizmaster_queue.push(QueuedQuestionQuizmaster {
                question: question_text.clone(),
                player: question_player.clone(),
                masked: question_masked,
                items: items_list,
                voters: question_voters,
            });
            Ok(())
        })?;
        return Ok(());
    }

    let items_str = items.iter().map(|item| item.name.as_str()).collect::<Vec<&str>>().join(", ");

    // Query with OpenAI API - Get 3 answers for each item to pick the most common answer
    let mut answers_choices: Vec<Vec<Answer>> = Vec::new();
    let mut successful_attempts = 0;
    let mut total_attempts = 0;

    let prompt = format!("u:For each item in this list '{items_str}', answer the question '{question_text}', return compact one line JSON with key answers which is a list of yes, no, maybe or unknown, this is a 20 questions game, British English");
    while successful_attempts < 3 && total_attempts < 3 {
        let mut futures = Vec::new();
        for _ in 0..3 {
            let future = query_ai(&prompt, 100, 1.0);
            futures.push(future);
        }

        let responses: Vec<Result<String>> = join_all(futures).await;
        for response in responses.into_iter().flatten() {
            if let Ok(validate_response) = serde_json::from_str::<AskQuestionResponse>(&response) {
                let mut choices = Vec::new();
                for answer_str in validate_response.answers {
                    if let Some(answer) = Answer::from_str(&answer_str) {
                        choices.push(answer);
                    }
                }
                if choices.len() == items.len() {
                    answers_choices.push(choices);
                    successful_attempts += 1;
                }
            } else {
                println!("Failed to parse answer response {response}");
            }
        }
        total_attempts += 1;
    }

    // Get most common answer for each item
    let mut answers: Vec<Answer> = Vec::new();
    for item_index in 0..items.len() {
        let mut answer_frequency: HashMap<Answer, usize> = HashMap::new();

        for answers in &answers_choices {
            let answer = answers.get(item_index).expect("Answers should have the same length as items");
            *answer_frequency.entry(answer.clone()).or_insert(0) += 1;
        }

        let most_common_answer = answer_frequency
            .into_iter()
            .max_by(|a, b| a.1.cmp(&b.1))
            .map_or(Answer::Unknown, |(ans, _)| ans);

        answers.push(most_common_answer);
    }

    with_lobby_mut(lobby_id, |lobby| {
        if answers.len() != lobby.items.len() {
            bail!("Failed to get answers for question '{question_text}'");
        }

        let question_id = lobby.questions_counter;
        lobby.questions_counter += 1;

        // Ask question against each item
        let mut remove_items = Vec::new();
        for (index, item) in &mut lobby.items.iter_mut().enumerate() {
            let answer = answers.get(index).unwrap_or(&Answer::Unknown).clone();
            item.questions.push(Question {
                player: question_player.clone(),
                id: question_id,
                text: question_text.clone(),
                answer,
                masked: question_masked,
            });

            // If item has 20 questions, remove the item
            if item.questions.len() >= 20 {
                remove_items.push(item.clone());
                for player_n in lobby.players.values_mut() {
                    player_n.messages.push(PlayerMessage::ItemRemoved(item.id, item.name.clone()));
                }
            }
        }
        if !remove_items.is_empty() {
            lobby.items.retain(|i| !remove_items.contains(i));
        }

        if lobby.questions_counter % lobby.settings.add_item_every_x_questions == 0 {
            add_item_to_lobby(lobby);
        }

        for player in lobby.players.values_mut() {
            player.messages.push(PlayerMessage::QuestionAsked);
        }
        Ok(())
    })?;

    test_game_over(lobby_id)
}

pub fn quizmaster_change_answer(lobby_id: &str, player_name: &str, question: &String, item_id: usize, new_answer: Answer) -> Result<()> {
    with_lobby_mut(lobby_id, |lobby| {
        if !lobby.started {
            bail!("Lobby not started");
        }
        let player = lobby.players.get(player_name).ok_or_else(|| anyhow::anyhow!("Player not found"))?;
        if !player.quizmaster {
            bail!("Only quizmaster can use this");
        }

        lobby
            .quizmaster_queue
            .iter_mut()
            .find(|q| &q.question == question)
            .and_then(|q| q.items.iter_mut().find(|i| i.id == item_id))
            .map(|i| i.answer = new_answer)
            .ok_or_else(|| anyhow::anyhow!("Question or item not found"))
    })
}

pub fn quizmaster_submit(lobby_id: &str, player_name: &str, question: &String) -> Result<()> {
    with_lobby_mut(lobby_id, |lobby| {
        if !lobby.started {
            bail!("Lobby not started");
        }
        let player = lobby.players.get(player_name).ok_or_else(|| anyhow::anyhow!("Player not found"))?;
        if !player.quizmaster {
            bail!("Only quizmaster can use this");
        }

        let question_index = lobby
            .quizmaster_queue
            .iter()
            .position(|q| &q.question == question)
            .ok_or_else(|| anyhow::anyhow!("Question not found"))?;
        let question = lobby.quizmaster_queue.remove(question_index);

        let question_id = lobby.questions_counter;
        lobby.questions_counter += 1;

        let mut remove_items = Vec::new();
        for quizmaster_item in question.items.clone() {
            let item = lobby.items.iter_mut().find(|i| i.id == quizmaster_item.id);
            if let Some(item) = item {
                item.questions.push(Question {
                    player: question.player.clone(),
                    id: question_id,
                    text: question.question.clone(),
                    answer: quizmaster_item.answer,
                    masked: question.masked,
                });

                // If item has 20 questions, remove the item
                if item.questions.len() >= 20 {
                    remove_items.push(item.clone());
                    for player_n in lobby.players.values_mut() {
                        player_n.messages.push(PlayerMessage::ItemRemoved(item.id, item.name.clone()));
                    }
                }
            }
        }

        if !remove_items.is_empty() {
            lobby.items.retain(|i| !remove_items.contains(i));
        }

        if lobby.questions_counter % lobby.settings.add_item_every_x_questions == 0 {
            add_item_to_lobby(lobby);
        }

        for player in lobby.players.values_mut() {
            player.messages.push(PlayerMessage::QuestionAsked);
        }

        Ok(())
    })
}

pub fn quizmaster_reject(lobby_id: &str, player_name: &str, question: &String) -> Result<()> {
    with_lobby_mut(lobby_id, |lobby| {
        if !lobby.started {
            bail!("Lobby not started");
        }
        let player = lobby.players.get(player_name).ok_or_else(|| anyhow::anyhow!("Player not found"))?;
        if !player.quizmaster {
            bail!("Only quizmaster can use this");
        }

        let question_index = lobby
            .quizmaster_queue
            .iter()
            .position(|q| &q.question == question)
            .ok_or_else(|| anyhow::anyhow!("Question not found"))?;
        let question = lobby.quizmaster_queue.remove(question_index);

        // Refund the voters
        for voter in question.voters {
            if let Some(player) = lobby.players.get_mut(&voter) {
                player.coins += 1;
            }
        }
        // Refund the question submitter and send them a message
        if let Some(player) = lobby.players.get_mut(&question.player) {
            player.coins += lobby.settings.submit_question_cost;
            player.messages.push(PlayerMessage::QuestionRejected(question.question));
        }

        Ok(())
    })
}

pub fn player_guess_item(lobby_id: &str, player_name: &str, item_choice: usize, guess: &str) -> Result<()> {
    let mut found_item = None;
    with_player_mut(lobby_id, player_name, |lobby, player| {
        if !lobby.started {
            bail!("Lobby not started");
        }
        if player.quizmaster {
            bail!("Quizmaster cannot engage");
        }

        let Some(item) = lobby.items.iter().find(|i| i.id == item_choice) else {
            bail!("Item not found");
        };
        found_item = Some(item.clone());

        if player.coins < lobby.settings.guess_item_cost {
            bail!("Insufficient coins to guess");
        }
        player.coins -= lobby.settings.guess_item_cost;

        if item.name.to_lowercase() != guess.to_lowercase() {
            player.messages.push(PlayerMessage::GuessIncorrect);
            bail!("Incorrect guess");
        }

        // Add score to player based on how many questions the item had remaining
        let remaining_questions = 20 - item.questions.len();
        player.score += remaining_questions;
        Ok(())
    })?;

    if let Some(item) = found_item {
        with_lobby_mut(lobby_id, |lobby| {
            // Remove item
            let item_id = item.id;
            let item_name = item.name.clone();
            lobby.items.retain(|i| i.id != item_id);

            // Send message to all players of item guessed
            for player_n in lobby.players.values_mut() {
                player_n
                    .messages
                    .push(PlayerMessage::ItemGuessed(player_name.to_string(), item_id, item_name.clone()));
            }

            // If lobby items is empty but theres still items in the item_queue, add another item
            if lobby.items.is_empty() && !lobby.items_queue.is_empty() {
                add_item_to_lobby(lobby);
            }

            Ok(())
        })?;

        return test_game_over(lobby_id);
    }
    Err(anyhow::anyhow!("Failed to find item"))
}

pub fn test_game_over(lobby_id: &str) -> Result<()> {
    with_lobby_mut(lobby_id, |lobby| {
        if lobby.started && lobby.items.is_empty() {
            lobby.started = false;

            // Find winner, player with max score, or if tied multiple players, or if 0 score no winner
            let mut max_score = 0;
            let mut winners = Vec::new();
            for player in lobby.players.values() {
                match player.score.cmp(&max_score) {
                    std::cmp::Ordering::Greater => {
                        max_score = player.score;
                        winners.clear();
                        winners.push(player.name.clone());
                    }
                    std::cmp::Ordering::Equal => {
                        winners.push(player.name.clone());
                    }
                    std::cmp::Ordering::Less => {}
                }
            }

            for player in lobby.players.values_mut() {
                player.messages.push(PlayerMessage::Winner(winners.clone()));
            }
        }
        Ok(())
    })?;
    Ok(())
}
