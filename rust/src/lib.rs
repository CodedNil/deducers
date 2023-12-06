use godot::prelude::*;

mod leaderboard;
mod networking;

struct DeducersExtension;

#[gdextension]
unsafe impl ExtensionLibrary for DeducersExtension {}
