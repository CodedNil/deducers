use godot::prelude::*;

mod networking;

struct DeducersExtension;

#[gdextension]
unsafe impl ExtensionLibrary for DeducersExtension {}
