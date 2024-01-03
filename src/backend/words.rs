use crate::backend::{openai::query_ai, with_lobby, Difficulty};
use once_cell::sync::Lazy;
use serde::Deserialize;
use std::sync::{Arc, Mutex};

#[derive(Deserialize)]
struct ItemsResponse {
    items: Vec<String>,
}

async fn get_ai_words(theme: String, difficulty: Difficulty, items: usize, item_history: Vec<String>) -> Vec<String> {
    let difficulty_description = match difficulty {
        Difficulty::Easy => "choose easy difficulty words",
        Difficulty::Medium => "choose easy or medium difficulty words",
        Difficulty::Hard => "choose easy, medium or hard difficulty words",
    };
    let theme_description = if !theme.trim().is_empty() {
        format!("with theme {}, ", theme)
    } else {
        String::new()
    };
    let item_history = if item_history.is_empty() {
        String::new()
    } else {
        format!("previous items chosen were {}, ", item_history.join(", "))
    };

    let mut items_return = Vec::new();
    let mut attempts = 0;

    while attempts < 2 && items_return.len() < items {
        let response = query_ai(
            &format!("u:Create {items} unique single word items to be used in a 20 questions game, such as Phone Bird Crystal, return compact one line JSON with key items, {theme_description}{item_history}aim for variety, British English, categories are [plant, animal, object] unless theme specifies otherwise, {difficulty_description}"),
            items * 3 + 20, 1.8
        ).await;

        if let Ok(message) = response {
            if let Ok(items_response) = serde_json::from_str::<ItemsResponse>(&message) {
                for item in items_response.items {
                    if item.len() > 2 && !item.contains(' ') && items_return.len() < items && !items_return.contains(&item) {
                        // Capitalise the first letter of the item
                        let item = item
                            .to_lowercase()
                            .chars()
                            .enumerate()
                            .map(|(i, c)| if i == 0 { c.to_ascii_uppercase() } else { c })
                            .collect::<String>();
                        items_return.push(item);
                    }
                }
            } else {
                println!("Failed to parse words from AI {}", message);
            }
        } else {
            println!("Failed to get words from AI {}", response.unwrap_err());
        }
        attempts += 1;
    }
    if attempts >= 2 {
        println!("Failed to get words from AI after 2 attempts");
    }

    items_return
}

static LOBBYS_PROCESSING: Lazy<Arc<Mutex<Vec<String>>>> = Lazy::new(|| Arc::new(Mutex::new(Vec::new())));

pub async fn topup_lobby_words(lobby_id: &str) {
    let (mut items_queue, mut item_count, mut theme, mut difficulty) = (Vec::new(), 0, String::new(), Difficulty::Easy);
    let _result = with_lobby(lobby_id, |lobby| {
        items_queue = lobby.items_queue.clone();
        item_count = lobby.settings.item_count;
        theme = lobby.settings.theme.clone();
        difficulty = lobby.settings.difficulty;
        Ok(())
    });
    let items_needed = item_count - items_queue.len();
    if items_needed <= 0 {
        return;
    }

    let words = get_ai_words(theme, difficulty, items_needed, items_queue).await;
    let _result = with_lobby(lobby_id, |lobby| {
        let items_needed = lobby.settings.item_count - lobby.items_queue.len();
        if items_needed <= 0 {
            return Ok(());
        }
        lobby.items_queue.extend(words.into_iter().take(items_needed));
        Ok(())
    });
}

pub fn topup_lobby_words_if_available(lobby_id: &str) {
    let mut processing = LOBBYS_PROCESSING.lock().unwrap();
    if processing.contains(&lobby_id.to_string()) {
        return;
    }
    processing.push(lobby_id.to_string());
    drop(processing);

    let lobby_id_clone = lobby_id.to_owned();
    tokio::spawn(async move {
        let _result = topup_lobby_words(&lobby_id_clone).await;
        let mut processing = LOBBYS_PROCESSING.lock().unwrap();
        processing.retain(|id| id != &lobby_id_clone);
        drop(processing);
    });
}
