use crate::WordSets;
use anyhow::Result;
use serde::Deserialize;
use std::collections::HashSet;

// Cutoff values for word frequency
const EASY_CUTOFF: i32 = 3000;
const MEDIUM_CUTOFF: i32 = 1000;
const HARD_CUTOFF: i32 = 50;

#[derive(Debug, Deserialize)]
struct SubtlexRecord {
    #[serde(rename = "DomPoSLemma")] // The base form of the word
    spelling: String,
    #[serde(rename = "DomPoSLemmaTotalFreq")]
    frequency: i32,
    #[serde(rename = "DomPoS")]
    wordtype: String,
}

pub fn parse() -> Result<WordSets> {
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .delimiter(b';')
        .from_path("data/subtlex.csv")?;

    let mut easy_words = HashSet::new();
    let mut medium_words = HashSet::new();
    let mut hard_words = HashSet::new();

    for result in rdr.deserialize() {
        match result {
            Ok(record) => {
                let record: SubtlexRecord = record;
                if record.wordtype != "noun" {
                    continue;
                }
                if record.spelling.contains(' ') || record.spelling.len() < 3 {
                    continue;
                }
                if record.frequency >= EASY_CUTOFF {
                    easy_words.insert(record.spelling);
                } else if record.frequency >= MEDIUM_CUTOFF {
                    medium_words.insert(record.spelling);
                } else if record.frequency >= HARD_CUTOFF {
                    hard_words.insert(record.spelling);
                }
            }
            Err(err) => println!("Error reading line {err:?}"),
        }
    }

    Ok(WordSets {
        easy_words,
        medium_words,
        hard_words,
    })
}
