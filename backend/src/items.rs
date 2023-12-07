use crate::{Answer, Item, Question, Server};

#[allow(clippy::cast_possible_truncation)]
pub fn add_item(server: &mut Server) {
    let potential_items = vec![
        "Pizza",
        "Boat",
        "Car",
        "Daisy",
        "Saw",
        "Rose",
        "Dog",
        "Shoes",
        "Laptop",
        "Drill",
        "Ball",
        "Toaster",
        "Chair",
        "Shirt",
        "Fish",
        "Bed",
        "Lego",
        "Puzzle",
        "Cactus",
        "Microwave",
        "Pants",
        "Sandwich",
        "Airplane",
        "Bird",
        "Doll",
        "Screwdriver",
        "Cat",
        "Bicycle",
        "Fridge",
        "Teddy Bear",
    ];

    let random_questions = vec![
        "Is it alive?",
        "Is it bigger than a microwave?",
        "Is it smaller than a shoebox?",
        "Is it edible?",
        "Is it a tool?",
        "Is it a toy?",
        "Is it a piece of furniture?",
        "Is it a piece of clothing?",
    ];

    // Get 3 random questions
    let mut questions = Vec::new();
    for _ in 0..3 {
        let random_question = random_questions[rand::random::<usize>() % random_questions.len()];
        // Get random answer
        let random_answer = match rand::random::<usize>() % 3 {
            0 => Answer::Yes,
            1 => Answer::No,
            _ => Answer::Maybe,
        };
        questions.push(Question {
            player: "Server".to_string(),
            question: random_question.to_string(),
            answer: random_answer,
            anonymous: false,
        });
    }

    let random_item = potential_items[rand::random::<usize>() % potential_items.len()].to_string();
    server.items.push(Item {
        name: random_item.clone(),
        id: server.items_history.len() as u32 + 1,
        questions,
    });

    server.items_history.push(random_item);
}
