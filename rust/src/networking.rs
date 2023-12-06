use std::time::Duration;

use godot::{
    engine::{Control, IControl, LineEdit},
    prelude::*,
};

#[derive(GodotClass)]
#[class(base=Control)]
struct DeducersMain {
    #[base]
    base: Base<Control>,
    http_client: ureq::Agent,
    server_ip: String,
    player_name: String,
    room_name: String,
}

#[godot_api]
impl DeducersMain {
    #[signal]
    fn start_game();

    #[func]
    fn on_connect_button_pressed(&mut self) {
        // Get nodes
        let server_ip_text = self
            .base
            .get_node_as::<LineEdit>("ConnectUI/ColorRect/VBoxContainer/ServerIp")
            .get_text()
            .to_string();
        let room_name_text = self
            .base
            .get_node_as::<LineEdit>("ConnectUI/ColorRect/VBoxContainer/HBoxContainer/RoomName")
            .get_text()
            .to_string();
        let player_name_text = self
            .base
            .get_node_as::<LineEdit>("ConnectUI/ColorRect/VBoxContainer/HBoxContainer/PlayerName")
            .get_text()
            .to_string();

        // Make post request to connect
        let url =
            format!("http://{server_ip_text}/server/{room_name_text}/connect/{player_name_text}");
        let result = ureq::post(&url).call();

        godot_print!("{}", url);
        match result {
            Ok(response) => {
                // Handle successful response
                godot_print!(
                    "Response: {}",
                    response
                        .into_string()
                        .unwrap_or_else(|_| "Failed to read response".to_string())
                );
            }
            Err(error) => {
                // Handle error
                godot_print!("Error: {}", error);
            }
        }

        // Set fields
        self.server_ip = server_ip_text;
        self.player_name = player_name_text;
        self.room_name = room_name_text;
    }
}

#[godot_api]
impl IControl for DeducersMain {
    fn init(base: Base<Control>) -> Self {
        Self {
            base,
            http_client: ureq::builder()
                .timeout_connect(Duration::from_secs(5))
                .build(),
            server_ip: String::new(),
            player_name: String::new(),
            room_name: String::new(),
        }
    }
}
