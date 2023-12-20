use once_cell::sync::Lazy;
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
