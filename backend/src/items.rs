use crate::{Item, Server};

#[allow(clippy::cast_possible_truncation)]
fn add_item(mut server: Server) {
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
        name: random_item,
        id: server.items_history.len() as u32,
        questions: Vec::new(),
    });
}
