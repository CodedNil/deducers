use crate::lobby_utils::Difficulty;
use once_cell::sync::Lazy;
use rand::seq::SliceRandom;
use std::collections::HashSet;

pub static WORD_SETS: Lazy<WordSets> = Lazy::new(|| parse_words(include_str!("words.txt")));

#[derive(Debug)]
pub struct WordSets {
    pub easy_words: HashSet<String>,
    pub medium_words: HashSet<String>,
    pub hard_words: HashSet<String>,
}

fn parse_words(contents: &str) -> WordSets {
    let mut easy_words = HashSet::new();
    let mut medium_words = HashSet::new();
    let mut hard_words = HashSet::new();
    let mut current_set = &mut easy_words;

    for line in contents.lines() {
        if line.starts_with("[easy_words]") {
            current_set = &mut easy_words;
        } else if line.starts_with("[medium_words]") {
            current_set = &mut medium_words;
        } else if line.starts_with("[hard_words]") {
            current_set = &mut hard_words;
        } else {
            current_set.extend(line.split(',').map(String::from));
        }
    }

    WordSets {
        easy_words,
        medium_words,
        hard_words,
    }
}

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

pub fn select_lobby_words_unique(current_words: &[String], difficulty: &Difficulty, count: usize) -> Vec<String> {
    let mut unique_new_words = HashSet::new();
    let mut additional_items_needed = count;
    while additional_items_needed > 0 {
        for word in select_lobby_words(difficulty, additional_items_needed) {
            if !current_words.contains(&word) && unique_new_words.insert(word) {
                additional_items_needed -= 1;
            }
            if additional_items_needed == 0 {
                break;
            }
        }
    }
    unique_new_words.into_iter().collect()
}
