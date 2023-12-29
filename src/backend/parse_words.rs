use crate::backend::Difficulty;
use once_cell::sync::Lazy;
use rand::seq::SliceRandom;
use std::collections::HashSet;

pub static WORD_SETS: Lazy<WordSets> = Lazy::new(|| parse_words(include_str!("words.txt")));

#[derive(Debug)]
pub struct WordSets {
    pub easy: HashSet<String>,
    pub medium: HashSet<String>,
    pub hard: HashSet<String>,
}

fn parse_words(contents: &str) -> WordSets {
    let mut easy = HashSet::new();
    let mut medium = HashSet::new();
    let mut hard = HashSet::new();
    let mut current_set = &mut easy;

    for line in contents.lines() {
        if line.starts_with("[easy]") {
            current_set = &mut easy;
        } else if line.starts_with("[medium]") {
            current_set = &mut medium;
        } else if line.starts_with("[hard]") {
            current_set = &mut hard;
        } else {
            current_set.extend(line.split(',').map(String::from));
        }
    }

    WordSets { easy, medium, hard }
}

pub fn select_lobby_words(difficulty: Difficulty, count: usize) -> Vec<String> {
    let mut rng = rand::thread_rng();

    let combined_words = match difficulty {
        Difficulty::Easy => WORD_SETS.easy.iter().collect::<Vec<_>>(),
        Difficulty::Medium => [&WORD_SETS.easy, &WORD_SETS.medium]
            .iter()
            .flat_map(|set| set.iter())
            .collect::<Vec<_>>(),
        Difficulty::Hard => [&WORD_SETS.easy, &WORD_SETS.medium, &WORD_SETS.hard]
            .iter()
            .flat_map(|set| set.iter())
            .collect::<Vec<_>>(),
    };

    let mut shuffled_words = combined_words;
    shuffled_words.shuffle(&mut rng);

    shuffled_words.into_iter().take(count).cloned().collect()
}

pub fn select_lobby_words_unique(current_words: &[String], difficulty: Difficulty, count: usize) -> Vec<String> {
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
