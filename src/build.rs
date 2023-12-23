use anyhow::Result;
use rsass::{compile_scss_path, output::Format};
use serde::Deserialize;
use std::{
    collections::HashSet,
    fs::{read_dir, write, File},
    io::{BufRead, BufReader, BufWriter, Write},
    path::Path,
};

fn main() -> Result<()> {
    // Compile scss
    let scss_path = Path::new("src").join("style.scss");
    let css = compile_scss_path(&scss_path, Format::default()).expect("Failed to compile SCSS");
    write(Path::new("assets").join("style.css"), css)?;

    // Create words.txt
    if !Path::new("src/backend/words.txt").exists() {
        if let Err(result) = parse_words() {
            println!("cargo:warning=Error parsing words: {result:?}");
        }
    }

    Ok(())
}

const EASY_CUTOFF: i32 = 3000;
const MEDIUM_CUTOFF: i32 = 1000;
const HARD_CUTOFF: i32 = 50;

#[derive(Debug)]
pub struct WordSets {
    pub easy_words: HashSet<String>,
    pub medium_words: HashSet<String>,
    pub hard_words: HashSet<String>,
}

fn parse_words() -> Result<()> {
    let start_time = std::time::Instant::now();
    let subtlex_result: WordSets = parse_subtlex().expect("Error parsing subtlex");
    let wordnet_result: HashSet<String> = parse_wordnet().expect("Error parsing wordnet");
    println!("cargo:warning=Parse time: {:?}", start_time.elapsed());

    // Gather words that are in both lists
    let word_sets = WordSets {
        easy_words: subtlex_result.easy_words.intersection(&wordnet_result).cloned().collect(),
        medium_words: subtlex_result.medium_words.intersection(&wordnet_result).cloned().collect(),
        hard_words: subtlex_result.hard_words.intersection(&wordnet_result).cloned().collect(),
    };
    println!("cargo:warning=Easy words count: {:?}", word_sets.easy_words.len());
    println!("cargo:warning=Medium words count: {:?}", word_sets.medium_words.len());
    println!("cargo:warning=Hard words count: {:?}", word_sets.hard_words.len());
    println!("cargo:warning=Intersection time: {:?}", start_time.elapsed());

    // Write words to file
    let file_path = "src/backend/words.txt";
    let mut file = BufWriter::new(File::create(file_path)?);
    write_word_set(&mut file, "easy_words", &word_sets.easy_words)?;
    write_word_set(&mut file, "medium_words", &word_sets.medium_words)?;
    write_word_set(&mut file, "hard_words", &word_sets.hard_words)?;
    file.flush()?;
    println!("cargo:warning=Total time: {:?}", start_time.elapsed());
    Ok(())
}

fn write_word_set(file: &mut BufWriter<File>, title: &str, words: &HashSet<String>) -> Result<()> {
    // Sort words alphabetically
    let mut words = words.iter().collect::<Vec<&String>>();
    words.sort();

    writeln!(file, "[{title}]")?;
    let words_string = words
        .iter()
        .map(|word| {
            let mut c = word.chars();
            c.next()
                .map_or_else(String::new, |first| first.to_uppercase().collect::<String>() + c.as_str())
        })
        .collect::<Vec<String>>()
        .join(",");
    writeln!(file, "{words_string}")?;
    Ok(())
}

#[derive(Debug, Deserialize)]
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

    let mut easy_words = HashSet::new();
    let mut medium_words = HashSet::new();
    let mut hard_words = HashSet::new();

    for result in rdr.deserialize() {
        match result {
            Ok(record) => {
                let record: SubtlexRecord = record;
                if record.wordtype != "noun" || record.spelling.contains(' ') || record.spelling.len() < 3 {
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

pub fn parse_wordnet() -> Result<HashSet<String>> {
    let base_path = Path::new("data").join("wordnet");
    let files = read_dir(base_path)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_file());

    let words = files.filter_map(|path| parse_wordnet_file(&path).ok()).flatten().collect();

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
