use crate::backend::{openai::query_ai, with_lobby, Difficulty};
use anyhow::ensure;
use once_cell::sync::Lazy;
use rand::{
    distributions::{Distribution, WeightedIndex},
    thread_rng,
};
use std::sync::{Arc, Mutex};

fn generate_weighted_string(length: usize) -> Vec<String> {
    let letters = [
        'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z',
    ];
    let weights = [8, 2, 3, 4, 12, 2, 2, 6, 7, 1, 1, 4, 3, 7, 8, 2, 1, 6, 6, 9, 3, 2, 2, 1, 2, 1];

    let mut rng = thread_rng();
    let dist = WeightedIndex::new(weights).unwrap();

    (0..length).map(|_| letters[dist.sample(&mut rng)].to_string()).collect()
}

async fn get_ai_words(theme: String, difficulty: Difficulty, items: usize, item_history: Vec<String>) -> Vec<String> {
    let difficulty_description = match difficulty {
        Difficulty::Easy => "choose simple words",
        Difficulty::Medium => "choose simple or middling difficulty words",
        Difficulty::Hard => "choose simple or complex words",
    };
    let theme_description = if theme.trim().is_empty() {
        String::new()
    } else {
        format!("with theme {theme}, ")
    };
    let item_history = if item_history.is_empty() {
        String::new()
    } else {
        format!("previous items chosen were {}, ", item_history.join(", "))
    };
    let characters_prompt = format!(
        "words should start with each of these letters in order {}",
        generate_weighted_string(items).join(";").to_uppercase()
    );

    let mut items_return = Vec::new();
    let mut attempts = 0;

    while attempts < 2 && items_return.len() < items {
        let response = query_ai(
            &format!("u:Create {items} unique single word items to be used in a 20 questions game, such as Phone Bird Crystal, return a comma separated list of items no additional text or spaces, {characters_prompt}, {theme_description}{item_history}aim for variety, British English, categories are [plant, animal, object] unless theme specifies otherwise, {difficulty_description}"),
            items * 3 + 20, 2.0, false
        ).await;
        if let Ok(message) = response {
            for item in message.split(',') {
                if item.len() > 2
                    && item.len() < 15
                    && !item.contains(' ')
                    && items_return.len() < items
                    && !items_return.contains(&item.to_owned())
                {
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
        }
        attempts += 1;
    }
    if attempts >= 2 {
        println!("Failed to get words from AI after 2 attempts");
    }

    items_return
}

static LOBBYS_PROCESSING: Lazy<Arc<Mutex<Vec<String>>>> = Lazy::new(|| Arc::new(Mutex::new(Vec::new())));

async fn topup_lobby(lobby_id: &str) {
    let (mut items_queue, mut item_count, mut theme, mut difficulty) = (Vec::new(), 0, String::new(), Difficulty::Easy);
    let _result = with_lobby(lobby_id, |lobby| {
        items_queue.clone_from(&lobby.items_queue);
        item_count = lobby.settings.item_count;
        theme.clone_from(&lobby.settings.theme);
        difficulty = lobby.settings.difficulty;
        Ok(())
    });
    if items_queue.len() >= item_count {
        return;
    }
    let items_needed = item_count - items_queue.len();

    let words = get_ai_words(theme, difficulty, items_needed, items_queue).await;
    let _result = with_lobby(lobby_id, |lobby| {
        let (items_queue, item_count) = (&mut lobby.items_queue, lobby.settings.item_count);
        ensure!(items_queue.len() < item_count, "Item queue is full");
        items_queue.extend(words.into_iter().take(item_count - items_queue.len()));
        Ok(())
    });
}

pub fn topup_lobby_if_available(lobby_id: &str) {
    let mut processing = LOBBYS_PROCESSING.lock().unwrap();
    if processing.contains(&lobby_id.to_string()) {
        return;
    }
    processing.push(lobby_id.to_string());
    drop(processing);

    let lobby_id_clone = lobby_id.to_owned();
    tokio::spawn(async move {
        let _result = topup_lobby(&lobby_id_clone).await;
        let mut processing = LOBBYS_PROCESSING.lock().unwrap();
        processing.retain(|id| id != &lobby_id_clone);
        drop(processing);
    });
}
