#![allow(clippy::missing_errors_doc)]
use anyhow::Result;
use rsass::{compile_scss_path, output::Format};
use serde::Deserialize;
use std::{
    collections::HashSet,
    fs::{read_dir, write, File},
    io::{BufRead, BufReader, BufWriter, Write},
    path::Path,
};

fn main() {
    // Compile scss
    let scss_path = Path::new("src").join("style.scss");
    let css = compile_scss_path(&scss_path, Format::default()).expect("Failed to compile SCSS");
    write(Path::new("src").join("style.css"), css).expect("Failed to write CSS");

    // Create words.txt
    if !Path::new("src/backend/words.txt").exists() {
        parse_words().expect("Failed to parse words");
    }
}

const EASY_CUTOFF: i32 = 3000;
const MEDIUM_CUTOFF: i32 = 1000;
const HARD_CUTOFF: i32 = 50;

pub struct WordSets {
    pub easy: HashSet<String>,
    pub medium: HashSet<String>,
    pub hard: HashSet<String>,
}

fn parse_words() -> Result<()> {
    let start_time = std::time::Instant::now();
    let subtlex_result: WordSets = parse_subtlex().expect("Error parsing subtlex");
    let wordnet_result: HashSet<String> = parse_wordnet().expect("Error parsing wordnet");
    println!("cargo:warning=Parse time: {:?}", start_time.elapsed());

    // Gather words that are in both lists
    let word_sets = WordSets {
        easy: subtlex_result.easy.intersection(&wordnet_result).cloned().collect(),
        medium: subtlex_result.medium.intersection(&wordnet_result).cloned().collect(),
        hard: subtlex_result.hard.intersection(&wordnet_result).cloned().collect(),
    };
    println!(
        "cargo:warning=Word counts: Easy:{:?} Medium:{:?} Hard:{:?}",
        word_sets.easy.len(),
        word_sets.medium.len(),
        word_sets.hard.len()
    );
    println!("cargo:warning=Intersection time: {:?}", start_time.elapsed());

    // Write words to file
    let file_path = "src/backend/words.txt";
    let mut file = BufWriter::new(File::create(file_path)?);
    write_word_set(&mut file, "easy", &word_sets.easy)?;
    write_word_set(&mut file, "medium", &word_sets.medium)?;
    write_word_set(&mut file, "hard", &word_sets.hard)?;
    file.flush()?;
    println!("cargo:warning=Total time: {:?}", start_time.elapsed());
    Ok(())
}

fn write_word_set(file: &mut BufWriter<File>, title: &str, words: &HashSet<String>) -> Result<()> {
    // Sort words alphabetically
    let mut words = words.iter().collect::<Vec<&String>>();
    words.sort();
    // Capitalize first letter of each word
    let words = words
        .iter()
        .map(|word| {
            let mut c = word.chars();
            c.next()
                .map_or_else(String::new, |first| first.to_uppercase().collect::<String>() + c.as_str())
        })
        .collect::<Vec<String>>();
    writeln!(file, "[{}]\n{}", title, words.join(","))?;
    Ok(())
}

#[derive(Deserialize)]
struct SubtlexRecord {
    #[serde(rename = "DomPoSLemma")] // The base form of the word
    spelling: String,
    #[serde(rename = "DomPoSLemmaTotalFreq")]
    frequency: i32,
    #[serde(rename = "DomPoS")]
    wordtype: String,
}

pub fn parse_subtlex() -> Result<WordSets> {
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .delimiter(b';')
        .from_path("data/subtlex.csv")?;

    let (mut easy, mut medium, mut hard) = (HashSet::new(), HashSet::new(), HashSet::new());
    for result in rdr.deserialize::<SubtlexRecord>() {
        if let Ok(record) = result {
            if record.wordtype == "noun" && !record.spelling.contains(' ') && record.spelling.len() >= 3 {
                if record.frequency >= EASY_CUTOFF {
                    easy.insert(record.spelling);
                } else if record.frequency >= MEDIUM_CUTOFF {
                    medium.insert(record.spelling);
                } else if record.frequency >= HARD_CUTOFF {
                    hard.insert(record.spelling);
                }
            }
        } else if let Err(err) = result {
            println!("Error reading line {err:?}");
        }
    }

    Ok(WordSets { easy, medium, hard })
}

pub fn parse_wordnet() -> Result<HashSet<String>> {
    let words = read_dir(Path::new("data").join("wordnet"))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .filter_map(|path| parse_wordnet_file(&path).ok())
        .flatten()
        .collect();
    Ok(words)
}

fn parse_wordnet_file(path: &Path) -> Result<HashSet<String>> {
    let mut words = HashSet::new();
    for line in BufReader::new(File::open(path)?).lines() {
        let line = line?;
        if let Some(word) = parse_wordnet_line(&line) {
            words.insert(word);
        }
    }
    Ok(words)
}

fn parse_wordnet_line(line: &str) -> Option<String> {
    line.starts_with('{').then(|| {
        line.split_whitespace()
            .filter(|word| word.len() > 2 && word.ends_with(','))
            .take(3)
            .map(|word| word.trim_matches('{').trim_matches(',').to_lowercase())
            .find(|word| word.chars().all(char::is_alphabetic))
    })?
}
