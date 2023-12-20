use crate::{
    backend::openai::query_ai,
    backend::parse_words::WORD_SETS,
    lobby_utils::{with_lobby_mut, with_player_mut, Answer, Difficulty, Item, Lobby, PlayerMessage, Question},
};
use anyhow::Result;
use futures::future::join_all;
use rand::seq::SliceRandom;
use serde::Deserialize;
use std::collections::HashMap;

pub fn select_lobby_words(difficulty: &Difficulty, count: usize) -> Vec<String> {
    let mut rng = rand::thread_rng();

    let combined_words = match difficulty {
        Difficulty::Easy => WORD_SETS.easy_words.iter().collect::<Vec<_>>(),
        Difficulty::Medium => [&WORD_SETS.easy_words, &WORD_SETS.medium_words]
            .iter()
            .flat_map(|set| set.iter())
            .collect::<Vec<_>>(),
        Difficulty::Hard => [&WORD_SETS.easy_words, &WORD_SETS.medium_words, &WORD_SETS.hard_words]
            .iter()
            .flat_map(|set| set.iter())
            .collect::<Vec<_>>(),
    };

    let mut shuffled_words = combined_words;
    shuffled_words.shuffle(&mut rng);

    shuffled_words.into_iter().take(count).cloned().collect()
}

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
pub async fn ask_top_question(lobby_id: String) -> Result<()> {
    let (mut question_text, mut question_player, mut question_anonymous) = (String::new(), String::new(), false);
    let mut question_id = 0;
    let mut items = Vec::new();

    with_lobby_mut(&lobby_id, |lobby| {
        let question = lobby
            .questions_queue
            .iter()
            .max_by_key(|question| question.votes)
            .ok_or_else(|| anyhow::anyhow!("No questions in queue"))?;

        if question.votes < lobby.settings.question_min_votes {
            return Err(anyhow::anyhow!(
                "Question needs at least {} votes",
                lobby.settings.question_min_votes
            ));
        }

        question_text = question.question.clone();
        question_player = question.player.clone();
        question_anonymous = question.anonymous;
        items = lobby.items.clone();

        // Remove question from queue
        question_id = lobby.questions_counter;
        lobby.questions_queue.retain(|q| q.question != question_text);
        lobby.questions_counter += 1;

        // Reset queue waiting if needed
        if !lobby.questions_queue.iter().any(|q| q.votes >= lobby.settings.question_min_votes) {
            lobby.questions_queue_waiting = true;
            lobby.questions_queue_countdown = lobby.settings.submit_question_every_x_seconds as f64;
        }

        Ok(())
    })
    .await?;

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

    with_lobby_mut(&lobby_id, |lobby| {
        if answers.len() != lobby.items.len() {
            return Err(anyhow::anyhow!("Failed to get answers for question '{question_text}'"));
        }

        // Ask question against each item
        let mut remove_items = Vec::new();
        for (index, item) in &mut lobby.items.iter_mut().enumerate() {
            let answer = answers.get(index).unwrap_or(&Answer::Unknown).clone();
            item.questions.push(Question {
                player: question_player.clone(),
                id: question_id,
                question: question_text.clone(),
                answer,
                anonymous: question_anonymous,
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
    })
    .await?;

    test_game_over(lobby_id).await
}

pub async fn player_guess_item(lobby_id: String, player_name: String, item_choice: usize, guess: String) -> Result<()> {
    let mut found_item = None;
    with_player_mut(&lobby_id, &player_name, |lobby, player| {
        if !lobby.started {
            return Err(anyhow::anyhow!("Lobby not started"));
        }

        let Some(item) = lobby.items.iter().find(|i| i.id == item_choice) else {
            return Err(anyhow::anyhow!("Item not found"));
        };
        found_item = Some(item.clone());

        if player.coins < lobby.settings.guess_item_cost {
            return Err(anyhow::anyhow!("Insufficient coins to guess"));
        }
        player.coins -= lobby.settings.guess_item_cost;

        if item.name.to_lowercase() != guess.to_lowercase() {
            player.messages.push(PlayerMessage::GuessIncorrect);
            return Err(anyhow::anyhow!("Incorrect guess"));
        }

        // Add score to player based on how many questions the item had remaining
        let remaining_questions = 20 - item.questions.len();
        player.score += remaining_questions;
        Ok(())
    })
    .await?;

    if let Some(item) = found_item {
        with_lobby_mut(&lobby_id, |lobby| {
            // Remove item
            let item_id = item.id;
            let item_name = item.name.clone();
            lobby.items.retain(|i| i.id != item_id);

            // Send message to all players of item guessed
            for player_n in lobby.players.values_mut() {
                player_n
                    .messages
                    .push(PlayerMessage::ItemGuessed(player_name.clone(), item_id, item_name.clone()));
            }
            Ok(())
        })
        .await?;

        return test_game_over(lobby_id).await;
    }
    Err(anyhow::anyhow!("Failed to find item"))
}

pub async fn test_game_over(lobby_id: String) -> Result<()> {
    with_lobby_mut(&lobby_id, |lobby| {
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
    })
    .await?;
    Ok(())
}
