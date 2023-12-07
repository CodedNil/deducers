use crate::{Answer, Item, Question, Server, ADD_ITEM_EVERY_X_QUESTIONS};

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

    let random_item = potential_items[rand::random::<usize>() % potential_items.len()].to_string();
    server.items.push(Item {
        name: random_item.clone(),
        id: server.items_history.len() as u32 + 1,
        questions: Vec::new(),
    });

    server.items_history.push(random_item);
}

pub fn ask_top_question(server: &mut Server) {
    let top_question = server
        .questions_queue
        .iter()
        .max_by_key(|question| question.votes);

    if let Some(question) = top_question {
        let question_clone = question.question.clone();

        // Ask question against each item (give random answer temporarily)
        let mut retain_items = Vec::new();
        for item in &mut server.items {
            // Check if item already has question
            if item
                .questions
                .iter()
                .any(|q| q.question == question.question)
            {
                retain_items.push(item.clone());
                continue;
            }

            let random_answer = match rand::random::<usize>() % 3 {
                0 => Answer::Yes,
                1 => Answer::No,
                _ => Answer::Maybe,
            };
            item.questions.push(Question {
                player: question.player.clone(),
                question: question.question.clone(),
                answer: random_answer,
                anonymous: question.anonymous,
            });

            // If item has 20 questions, remove the item
            if item.questions.len() < 20 {
                retain_items.push(item.clone());
            }
        }
        server.items = retain_items;

        // Remove question from queue
        server
            .questions_queue
            .retain(|q| q.question != question_clone);
        server.questions_counter += 1;

        // Add new item if x questions have been asked
        if server.questions_counter % ADD_ITEM_EVERY_X_QUESTIONS == 0 {
            add_item(server);
        }
    }
}
