use godot::{
    engine::{Button, Control, IControl},
    prelude::*,
};

#[derive(GodotClass)]
#[class(base=Control)]
struct DeducersMain {
    #[base]
    base: Base<Control>,
    server_ip: String,
    player_name: String,
    room_name: String,
}

#[godot_api]
impl IControl for DeducersMain {
    fn init(base: Base<Control>) -> Self {
        Self {
            base,
            server_ip: "".to_string(),
            player_name: "".to_string(),
            room_name: "".to_string(),
        }
    }

    fn ready(&mut self) {
        // Find the Connect button and bind the on_connect_button_pressed method)
        let mut connect_button = self
            .base
            .get_node_as::<Button>("ConnectUI/ColorRect/VBoxContainer/HBoxContainer/Connect");
        connect_button.connect(
            "pressed".into(),
            Callable::from_fn("button_pressed", |_args: &[&Variant]| {
                godot_print!("Connect button pressed!");
                Ok(Variant::nil())
            }),
        );
    }
}
