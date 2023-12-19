use anyhow::Result;
use std::collections::HashSet;
use std::fs::{read_dir, File};
use std::io::{BufRead, BufReader};
use std::path::Path;

pub fn parse() -> Result<HashSet<String>> {
    let base_path = Path::new("data").join("wordnet");
    let files = read_dir(base_path)?;

    let mut words = HashSet::new();

    for file in files.flatten() {
        let path = file.path();
        if path.is_file() {
            words.extend(parse_wordnet_file(&path)?);
        }
    }

    Ok(words)
}

fn parse_wordnet_file(path: &Path) -> Result<HashSet<String>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    let mut words = HashSet::new();
    for line in reader.lines() {
        let line = line?;
        if let Some(word) = parse_wordnet_line(&line) {
            words.insert(word);
        }
    }

    Ok(words)
}

fn parse_wordnet_line(line: &str) -> Option<String> {
    if line.starts_with('{') {
        line.split_whitespace()
            .filter(|word| word.len() > 2 && word.ends_with(','))
            .take(3)
            .map(|word| word.trim_matches('{').trim_matches(',').to_lowercase())
            .find(|word| word.chars().all(char::is_alphabetic))
    } else {
        None
    }
}
