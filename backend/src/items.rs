use crate::openai::query;
use crate::{Answer, Item, Question, Server, ServerStorage, ADD_ITEM_EVERY_X_QUESTIONS, GUESS_ITEM_COST, SERVER_PORT};
use async_recursion::async_recursion;
use axum::extract::ConnectInfo;
use axum::{extract::Path, http::StatusCode, response::IntoResponse, Extension};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

const MAX_RECURSIONS: u32 = 4; // Set a maximum for recursion depth for adding items

#[derive(Deserialize, Serialize, Debug)]
struct ItemsResponse {
    item1: String,
    item2: String,
    item3: String,
}

#[async_recursion]
pub async fn add_item_to_queue(server_id: String, mut items_history: Vec<String>, recursions: u32) {
    // Check if maximum recursion depth has been reached
    if recursions >= MAX_RECURSIONS {
        println!("Maximum recursion depth reached on adding item");
        return;
    }

    // Query with OpenAI API
    let response = query(
        &format!("u:Create 3 one word items to be used in a 20 questions game, such as Phone Bird Crystal, first letter capitalised, return compact one line JSON with keys item1 item2 item3, previous items were {items_history:?} don't repeat and aim for variety, British English"),
        100, 2.0
    ).await;
    if let Ok(message) = response {
        // Parse response
        if let Ok(items_response) = serde_json::from_str::<ItemsResponse>(&message) {
            // Iterate and add items that aren't in history
            for item in [items_response.item1, items_response.item2, items_response.item3] {
                if !items_history.contains(&item) {
                    // Add item to history
                    items_history.push(item.clone());

                    // Send request to server with ureq
                    let url = format!("http://localhost:{SERVER_PORT}/internal/{server_id}/additemqueued/{item}");
                    tokio::spawn(async move {
                        let client = reqwest::Client::new();
                        match client.post(&url).timeout(std::time::Duration::from_secs(5)).send().await {
                            Ok(_) => {
                                println!("Added item {item}");
                            }
                            Err(error) => {
                                println!("Errored adding item: {error}");
                            }
                        }
                    });
                }
            }
        } else {
            // Try again
            add_item_to_queue(server_id, items_history, recursions + 1).await;
        }
    } else {
        // Try again
        add_item_to_queue(server_id, items_history, recursions + 1).await;
    }
}

#[allow(clippy::cast_possible_truncation)]
pub async fn add_item_to_server_queue(
    Path((server_id, item_name)): Path<(String, String)>,
    Extension(servers): Extension<ServerStorage>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    // Check if the request is from localhost
    if addr.ip().is_loopback() {
        let mut servers_lock = servers.lock().await;
        if let Some(server) = servers_lock.get_mut(&server_id) {
            if !server.items_history.contains(&item_name) {
                server.items_queue.insert(0, item_name.clone());
                return StatusCode::OK;
            }
            drop(servers_lock);
            return StatusCode::BAD_REQUEST;
        };
        drop(servers_lock);
        return StatusCode::NOT_FOUND;
    }
    // Reject requests not from localhost
    StatusCode::FORBIDDEN
}

#[allow(clippy::cast_possible_truncation)]
pub fn add_item_to_server(server: &mut Server) {
    if !server.started {
        return;
    }
    // Get oldest item in queue, if no items return
    let Some(item_name) = server.items_queue.pop() else {
        return;
    };

    // Add item to server
    server.items.push(Item {
        name: item_name.clone(),
        id: server.items_history.len() as u32 + 1,
        questions: Vec::new(),
    });
    server.items_history.push(item_name);
}

// Helper function to validate a question
#[derive(Deserialize, Serialize, Debug)]
struct AskQuestionResponse {
    answers: Vec<String>,
}

pub async fn ask_top_question(servers: ServerStorage, server_id: String) {
    let mut servers_lock = servers.lock().await;
    let Some(server) = servers_lock.get_mut(&server_id) else {
        drop(servers_lock);
        return;
    };

    let top_question = server.questions_queue.iter().max_by_key(|question| question.votes);
    let Some(question) = top_question else {
        drop(servers_lock);
        return;
    };

    let question_clone = question.question.clone();
    let question_id = server.questions_counter;

    // Create list of items in string
    let items_str = server.items.iter().map(|item| item.name.as_str()).collect::<Vec<&str>>().join(", ");

    // Query with OpenAI API
    let mut answers = Vec::new();
    let mut attempt_count = 0;

    while attempt_count < 4 && answers.len() != server.items.len() {
        let response = query(
            &format!("u:For each item in this list '{items_str}', answer the question '{question_clone}', return compact one line JSON with key answers which is a list of yes, no or maybe, this is a 20 questions game, British English"),
            100,
            1.0,
        )
        .await;

        println!("Attempt {attempt_count}, Response: {response:?}");

        if let Ok(message) = response {
            if let Ok(validate_response) = serde_json::from_str::<AskQuestionResponse>(&message) {
                println!("Attempt {attempt_count}, Answers: {validate_response:?}");
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
            }
        }

        attempt_count += 1;
    }

    // Default to "maybe" if correct response not received after 4 attempts
    if answers.len() != server.items.len() {
        answers = vec![Answer::Maybe; server.items.len()];
    }

    // Ask question against each item (give random answer temporarily)
    let mut retain_items = Vec::new();
    for (index, item) in &mut server.items.iter_mut().enumerate() {
        let random_answer = answers.get(index).unwrap_or(&Answer::Maybe).clone();
        item.questions.push(Question {
            player: question.player.clone(),
            id: question_id,
            question: question.question.clone(),
            answer: random_answer,
            anonymous: question.anonymous,
        });

        // If item has 20 questions, remove the item
        if item.questions.len() < 20 {
            retain_items.push(item.clone());
        }
    }
    server.items = retain_items;

    // Remove question from queue
    server.questions_queue.retain(|q| q.question != question_clone);
    server.questions_counter += 1;

    // Add new item if x questions have been asked
    if server.questions_counter % ADD_ITEM_EVERY_X_QUESTIONS == 0 {
        add_item_to_server(server);
    }
}

#[allow(clippy::cast_possible_truncation)]
pub async fn player_guess_item(
    Path((server_id, player_name, item_choice_str, guess)): Path<(String, String, String, String)>,
    Extension(servers): Extension<ServerStorage>,
) -> impl IntoResponse {
    let mut servers_lock = servers.lock().await;
    let Some(server) = servers_lock.get_mut(&server_id) else {
        return (StatusCode::NOT_FOUND, "Server not found".to_string());
    };
    let Some(player) = server.players.get_mut(&player_name) else {
        return (StatusCode::NOT_FOUND, "Player not found in server".to_string());
    };

    // Get item with id of item_choice
    let Ok(item_choice) = item_choice_str.parse::<u32>() else {
        return (StatusCode::BAD_REQUEST, "Invalid item choice, must be a number".to_string());
    };
    let Some(item) = server.items.iter().find(|i| i.id == item_choice) else {
        return (StatusCode::BAD_REQUEST, "Invalid item choice, item not found".to_string());
    };

    // Check if player has enough coins
    if player.coins < GUESS_ITEM_COST {
        return (StatusCode::BAD_REQUEST, "Insufficient coins to guess item".to_string());
    }
    player.coins -= GUESS_ITEM_COST;

    // Match guess with item name
    if item.name == guess {
        // Add score to player
        player.score += 1;
        drop(servers_lock);
        (StatusCode::OK, "Correct guess".to_string())
    } else {
        drop(servers_lock);
        (StatusCode::NOT_ACCEPTABLE, "Incorrect guess".to_string())
    }
}
