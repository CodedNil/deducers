use crate::{
    backend::openai::query_ai, Answer, Item, Lobby, PlayerMessage, Question,
    ADD_ITEM_EVERY_X_QUESTIONS, GUESS_ITEM_COST, LOBBYS,
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
            for item in [
                items_response.item1,
                items_response.item2,
                items_response.item3,
            ] {
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
    let lobbys = LOBBYS
        .get()
        .ok_or_else(|| anyhow::anyhow!("LOBBYS not initialized"))?;
    let mut lobbys_lock = lobbys.lock().await;

    let lobby = lobbys_lock
        .get_mut(&lobby_id)
        .ok_or_else(|| anyhow::anyhow!("Lobby '{lobby_id}' not found"))?;

    if !lobby.items_history.contains(&item_name) {
        lobby.items_queue.insert(0, item_name.clone());
        drop(lobbys_lock);
        return Ok(());
    }
    Err(anyhow::anyhow!("Item already in history"))
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
    println!("Adding item to lobby: {item_name}");
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
    let lobbys = LOBBYS
        .get()
        .ok_or_else(|| anyhow::anyhow!("LOBBYS not initialized"))?;
    let mut lobbys_lock = lobbys.lock().await;

    let lobby = lobbys_lock
        .get_mut(&lobby_id)
        .ok_or_else(|| anyhow::anyhow!("Lobby '{lobby_id}' not found"))?;

    let top_question = lobby
        .questions_queue
        .iter()
        .max_by_key(|question| question.votes);
    let Some(question) = top_question else {
        drop(lobbys_lock);
        return Err(anyhow::anyhow!("No questions in queue"));
    };

    let question_clone = question.clone();
    let question_text = question.question.clone();
    let question_id = lobby.questions_counter;

    // Create list of items in string
    let items_str = lobby
        .items
        .iter()
        .map(|item| item.name.as_str())
        .collect::<Vec<&str>>()
        .join(", ");
    let items = lobby.items.clone();

    // Remove question from queue
    lobby
        .questions_queue
        .retain(|q| q.question != question_text);
    lobby.questions_counter += 1;

    drop(lobbys_lock);

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

    // Create a new lock
    let mut lobbys_lock = lobbys.lock().await;
    let lobby = lobbys_lock
        .get_mut(&lobby_id)
        .ok_or_else(|| anyhow::anyhow!("Lobby '{lobby_id}' not found"))?;

    // Default to "maybe" if correct response not received after 4 attempts
    if answers.len() != lobby.items.len() {
        println!("Failed to get answers for question: {question_text}");
        answers = vec![Answer::Maybe; lobby.items.len()];
    }

    // Ask question against each item (give random answer temporarily)
    let mut retain_items = Vec::new();
    for (index, item) in &mut lobby.items.iter_mut().enumerate() {
        let random_answer = answers.get(index).unwrap_or(&Answer::Maybe).clone();
        item.questions.push(Question {
            player: question_clone.player.clone(),
            id: question_id,
            question: question_clone.question.clone(),
            answer: random_answer,
            anonymous: question_clone.anonymous,
        });

        // If item has 20 questions, remove the item
        if item.questions.len() < 20 {
            retain_items.push(item.clone());
        }
    }
    lobby.items = retain_items;

    // Add new item if x questions have been asked
    if lobby.questions_counter % ADD_ITEM_EVERY_X_QUESTIONS == 0 {
        add_item_to_lobby(lobby);
    }

    // Send message to all players of question asked
    for player in lobby.players.values_mut() {
        player.messages.push(PlayerMessage::QuestionAsked);
    }
    drop(lobbys_lock);
    Ok(())
}

pub async fn player_guess_item(
    lobby_id: String,
    player_name: String,
    item_choice: usize,
    guess: String,
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

    // Get item with id of item_choice
    let Some(item) = lobby.items.iter().find(|i| i.id == item_choice) else {
        return Err(anyhow::anyhow!("Item not found"));
    };

    // Check if player has enough coins
    if player.coins < GUESS_ITEM_COST {
        return Err(anyhow::anyhow!("Insufficient coins to guess"));
    }
    player.coins -= GUESS_ITEM_COST;

    // Match guess with item name
    if item.name != guess {
        drop(lobbys_lock);
        return Err(anyhow::anyhow!("Incorrect guess"));
    }

    // Add score to player based on how many questions the item had remaining
    let remaining_questions = 20 - item.questions.len();
    player.score += remaining_questions;
    let player_name = player.name.clone();

    // Remove item
    let item_id = item.id;
    let item_name = item.name.clone();
    lobby.items.retain(|i| i.id != item_id);

    // Send message to all players of item guessed
    for player_n in lobby.players.values_mut() {
        player_n.messages.push(PlayerMessage::ItemGuessed(
            player_name.clone(),
            item_id,
            item_name.clone(),
        ));
    }
    drop(lobbys_lock);
    Ok(())
}
