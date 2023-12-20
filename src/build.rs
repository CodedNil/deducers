use crate::build_utils::{subtlex, wordnet};
use anyhow::Result;
use rsass::{compile_scss_path, output::Format};
use std::{
    collections::HashSet,
    fs::File,
    io::{BufWriter, Write},
};
use std::{fs, io, path::Path};

mod build_utils;

fn main() -> io::Result<()> {
    let scss_path = Path::new("src").join("style.scss");
    let css = compile_scss_path(&scss_path, Format::default()).expect("Failed to compile SCSS");

    let output_path = Path::new("assets").join("style.css");
    fs::write(output_path, css)?;

    if !Path::new("src/backend/words.txt").exists() {
        if let Err(result) = parse_words() {
            println!("cargo:warning=Error parsing words: {result:?}");
        }
    }

    Ok(())
}

#[derive(Debug)]
pub struct WordSets {
    pub easy_words: HashSet<String>,
    pub medium_words: HashSet<String>,
    pub hard_words: HashSet<String>,
}

fn parse_words() -> Result<()> {
    let start_time = std::time::Instant::now();
    let subtlex_result: WordSets = subtlex::parse().expect("Error parsing subtlex");
    let wordnet_result: HashSet<String> = wordnet::parse().expect("Error parsing wordnet");
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

    let file_path = "src/backend/words.txt";
    let mut file = BufWriter::new(File::create(file_path)?);

    write_word_sets(&mut file, &word_sets)?;

    file.flush()?;
    println!("cargo:warning=Total time: {:?}", start_time.elapsed());
    Ok(())
}

fn write_word_sets(file: &mut BufWriter<File>, word_sets: &WordSets) -> Result<()> {
    write_word_set(file, "easy_words", &word_sets.easy_words)?;
    write_word_set(file, "medium_words", &word_sets.medium_words)?;
    write_word_set(file, "hard_words", &word_sets.hard_words)?;
    Ok(())
}

fn write_word_set(file: &mut BufWriter<File>, title: &str, words: &HashSet<String>) -> Result<()> {
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
