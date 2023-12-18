use crate::{
    backend::openai::query_ai, with_lobby_mut, with_player_mut, Answer, Item, Lobby, PlayerMessage, Question, ADD_ITEM_EVERY_X_QUESTIONS,
    GUESS_ITEM_COST, QUESTION_MIN_VOTES, SUBMIT_QUESTION_EVERY_X_SECONDS,
};
use anyhow::Result;
use serde::Deserialize;

const MAX_RECURSIONS: u32 = 4; // Maximum for recursion depth for adding items

#[derive(Deserialize, Debug)]
struct ItemsResponse {
    item1: String,
    item2: String,
    item3: String,
}

#[async_recursion::async_recursion]
pub async fn add_item_to_queue(lobby_id: String, mut items_history: Vec<String>, recursions: u32) {
    // Check if maximum recursion depth has been reached
    if recursions >= MAX_RECURSIONS {
        println!("Maximum recursion depth reached on adding item");
        return;
    }

    // Query with OpenAI API
    let response = query_ai(
        &format!("u:Create 3 one word items to be used in a 20 questions game, such as Phone Bird Crystal, first letter capitalised, return compact one line JSON with keys item1 item2 item3, previous items were {items_history:?} don't repeat and aim for variety, British English, categories are [plant, animal, object]"),
        100, 2.0
    ).await;
    if let Ok(message) = response {
        // Parse response
        if let Ok(items_response) = serde_json::from_str::<ItemsResponse>(&message) {
            // Iterate and add items that aren't in history
            for item in [items_response.item1, items_response.item2, items_response.item3] {
                if item.len() < 3 {
                    continue;
                }
                if item.contains(' ') {
                    continue;
                }
                if !items_history.contains(&item) {
                    // Add item to history
                    items_history.push(item.clone());

                    // Send request to lobby with ureq
                    let _result = add_item_to_lobby_queue(lobby_id.clone(), item.clone()).await;
                }
            }
        } else {
            // Try again
            add_item_to_queue(lobby_id, items_history, recursions + 1).await;
        }
    } else {
        // Try again
        add_item_to_queue(lobby_id, items_history, recursions + 1).await;
    }
}

pub async fn add_item_to_lobby_queue(lobby_id: String, item_name: String) -> Result<()> {
    with_lobby_mut(&lobby_id, |lobby| {
        if !lobby.items_history.contains(&item_name) {
            lobby.items_queue.insert(0, item_name.clone());
            return Ok(());
        }
        Err(anyhow::anyhow!("Item already in history"))
    })
    .await
}

pub fn add_item_to_lobby(lobby: &mut Lobby) {
    if !lobby.started {
        return;
    }
    // Get oldest item in queue, if no items return
    let Some(item_name) = lobby.items_queue.pop() else {
        return;
    };

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

        if question.votes < QUESTION_MIN_VOTES {
            return Err(anyhow::anyhow!("Question needs at least {QUESTION_MIN_VOTES} votes"));
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
        if !lobby.questions_queue.iter().any(|q| q.votes >= QUESTION_MIN_VOTES) {
            lobby.questions_queue_waiting = true;
            lobby.questions_queue_countdown = SUBMIT_QUESTION_EVERY_X_SECONDS;
        }

        Ok(())
    })
    .await?;

    let items_str = items.iter().map(|item| item.name.as_str()).collect::<Vec<&str>>().join(", ");

    // Query with OpenAI API
    let mut answers = Vec::new();
    let mut attempt_count = 0;
    while attempt_count < 4 && answers.len() != items.len() {
        let response = query_ai(
            &format!("u:For each item in this list '{items_str}', answer the question '{question_text}', return compact one line JSON with key answers which is a list of yes, no or maybe, this is a 20 questions game, British English"),
            100,
            1.0,
        )
        .await;

        if let Ok(message) = response {
            if let Ok(validate_response) = serde_json::from_str::<AskQuestionResponse>(&message) {
                answers.clear();
                for answer in validate_response.answers {
                    let answer = match answer.to_lowercase().trim() {
                        "yes" => Answer::Yes,
                        "no" => Answer::No,
                        "maybe" => Answer::Maybe,
                        _ => continue,
                    };
                    answers.push(answer);
                }
            } else {
                println!("Failed to parse answer response {message}");
            }
        }

        attempt_count += 1;
    }

    with_lobby_mut(&lobby_id, |lobby| {
        if answers.len() != lobby.items.len() {
            return Err(anyhow::anyhow!("Failed to get answers for question '{question_text}'"));
        }

        // Ask question against each item
        let mut remove_items = Vec::new();
        for (index, item) in &mut lobby.items.iter_mut().enumerate() {
            let answer = answers.get(index).unwrap_or(&Answer::Maybe).clone();
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

        if lobby.questions_counter % ADD_ITEM_EVERY_X_QUESTIONS == 0 {
            add_item_to_lobby(lobby);
        }

        for player in lobby.players.values_mut() {
            player.messages.push(PlayerMessage::QuestionAsked);
        }
        Ok(())
    })
    .await
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

        if player.coins < GUESS_ITEM_COST {
            return Err(anyhow::anyhow!("Insufficient coins to guess"));
        }
        player.coins -= GUESS_ITEM_COST;

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
        return with_lobby_mut(&lobby_id, |lobby| {
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
        .await;
    }
    Err(anyhow::anyhow!("Failed to find item"))
}
