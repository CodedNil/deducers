struct GameState {
    players: Vec<Player>,
    guesses: Vec<Guess>,
    items: Vec<Item>,
}

struct Player {
    id: u32,
    name: String,
    points: u32,
}

struct Guess {
    player_id: u32,
    guess: String,
    votes: u32,
}

struct Item {
    name: String,
    questions: Vec<Question>,
}

struct Question {
    asker_id: u32,
    question: String,
    answer: Answer,
}

enum Answer {
    Yes,
    No,
    Sometimes,
    Depends,
    Irrelevant,
}

fn test() {
    let game_state = GameState {
        players: vec![
            Player {
                id: 1,
                name: "Player 1".to_string(),
                points: 4,
            },
            Player {
                id: 2,
                name: "Player 2".to_string(),
                points: 2,
            },
            Player {
                id: 3,
                name: "Player 3".to_string(),
                points: 1,
            },
        ],
        guesses: vec![
            Guess {
                player_id: 1,
                guess: "Is it a square?".to_string(),
                votes: 2,
            },
            Guess {
                player_id: 2,
                guess: "Is it edible?".to_string(),
                votes: 1,
            },
            Guess {
                player_id: 3,
                guess: "Is it alive?".to_string(),
                votes: 0,
            },
        ],
        items: vec![],
    };
}
