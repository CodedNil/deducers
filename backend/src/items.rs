use crate::openai::query;
use crate::{
    Answer, Item, Question, Server, ServerStorage, ADD_ITEM_EVERY_X_QUESTIONS, SERVER_PORT,
};
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
        &format!("u:Create 3 one word items to be used in a 20 questions game, such as Phone Bird Crystal, first letter capitalised, return compact JSON with keys item1 item2 item3, previous items were {items_history:?} don't repeat and aim for variety, British English"),
        100,
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
                if !items_history.contains(&item) {
                    // Add item to history
                    items_history.push(item.clone());

                    // Send request to server with ureq
                    let url = format!(
                        "http://localhost:{SERVER_PORT}/internal/{server_id}/additemqueued/{item}"
                    );
                    tokio::spawn(async move {
                        let client = reqwest::Client::new();
                        match client
                            .post(&url)
                            .timeout(std::time::Duration::from_secs(5))
                            .send()
                            .await
                        {
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
        let mut servers = servers.lock().await;
        if let Some(server) = servers.get_mut(&server_id) {
            if !server.items_history.contains(&item_name) {
                server.items_queue.insert(0, item_name.clone());
                return StatusCode::OK;
            }
            return StatusCode::BAD_REQUEST;
        };
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

pub fn ask_top_question(server: &mut Server) {
    let top_question = server
        .questions_queue
        .iter()
        .max_by_key(|question| question.votes);

    if let Some(question) = top_question {
        let question_clone = question.question.clone();
        let question_id = server.questions_counter;

        // Ask question against each item (give random answer temporarily)
        let mut retain_items = Vec::new();
        for item in &mut server.items {
            // Check if item already has question
            if item
                .questions
                .iter()
                .any(|q| q.question == question.question)
            {
                retain_items.push(item.clone());
                continue;
            }

            let random_answer = match rand::random::<usize>() % 3 {
                0 => Answer::Yes,
                1 => Answer::No,
                _ => Answer::Maybe,
            };
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
        server
            .questions_queue
            .retain(|q| q.question != question_clone);
        server.questions_counter += 1;

        // Add new item if x questions have been asked
        if server.questions_counter % ADD_ITEM_EVERY_X_QUESTIONS == 0 {
            add_item_to_server(server);
        }
    }
}
