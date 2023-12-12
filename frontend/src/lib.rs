use godot::prelude::*;

mod game_state;
mod items;
mod leaderboard;
mod networking;
mod questions_queue;

struct DeducersExtension;

#[gdextension]
unsafe impl ExtensionLibrary for DeducersExtension {}
