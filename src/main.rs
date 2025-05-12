mod client;
mod server;

use crate::client::Client;
use crate::server::Server;
use macroquad::math::f32;
use macroquad::prelude::*;
use macroquad::ui::{
    hash, root_ui,
    widgets::{self, Group},
    Drag, Ui,
};
use std::cell::{RefCell, RefMut};
use std::rc::Rc;

fn get_time_ms() -> u128 {
    (get_time() * 1000.0) as u128
}

struct Entity {
    x: f32,
    speed: i32,
    entity_id: u32,
    position_buffer: Vec<(u128, f32)>,
}

impl Entity {
    fn new(entity_id: u32) -> Self {
        Entity {
            x: 40.0 + entity_id as f32 * 100.0,
            speed: 40000,
            entity_id,
            position_buffer: Vec::new(),
        }
    }

    fn applyInput(&mut self, input: MovementInput) {
        self.x += input.press_time * self.speed as f32;
    }
}

#[derive(Debug)]
enum Message {
    Movement(MovementInput),
    WorldState(WorldStateMessage),
}

#[derive(Debug)]
struct MovementInput {
    press_time: f32,
    entity_id: u32,
    input_sequence_number: u32,
}

impl MovementInput {
    pub(crate) fn clone(&self) -> MovementInput {
        MovementInput {
            press_time: self.press_time,
            entity_id: self.entity_id,
            input_sequence_number: self.input_sequence_number,
        }
    }
}

#[derive(Debug, Clone)]
struct world_state {
    entity_id: u32,
    position: f32,
    last_processed_input: f32,
}

#[derive(Debug, Clone)]
struct WorldStateMessage {
    world_state: Vec<world_state>,
}

struct NetworkMessage {
    receive_time: u128,
    payload: Message,
}

struct LagNetwork {
    messages: Vec<NetworkMessage>,
}

impl LagNetwork {
    fn send(&mut self, lag_ms: f32, message: Message) {
        let in_ms = get_time_ms();

        //set recv time to time now + lag_ms
        let receive_time = in_ms + lag_ms as u128;

        // make the NetworkMessage
        let network_message = NetworkMessage {
            receive_time: receive_time,
            payload: message,
        };

        self.messages.push(network_message);
    }

    fn receive(&mut self) -> Option<Message> {
        if self.messages.len() == 0 {
            return None;
        }

        for (i, v) in self.messages.iter().enumerate() {
            let in_ms = get_time_ms();

            if in_ms >= v.receive_time {
                let message = self.messages.remove(i);
                return Some(message.payload);
            }
        }
        return None;
    }
}

fn draw_coloured_rectangle(x: f32, y: f32, width: f32, height: f32, colour: Color) {
    // draw with line thickness of 2.0
    draw_line(x, y, x + width, y, 2.0, colour);
}

// function for drawing things on the screen
fn draw_client_entities(client: RefMut<Client>, y_offset: f32) {
    let player_colour = if client.entity_id == 1 { BLUE } else { RED };

    // draw outline rectangle
    draw_rectangle_lines(
        10.0,
        y_offset - 65.0,
        screen_width() - 20.0,
        120.0,
        2.0,
        player_colour,
    );

    let mut move_message = "move with LEFT and RIGHT arrow keys";
    if client.entity_id == 2 {
        move_message = "move with A and D keys"
    }

    draw_text(
        &format!("Player {} view - {}", client.entity_id, move_message),
        20.0,
        y_offset - 40.0,
        20.0,
        DARKGRAY,
    );

    // write the number of non-acknowledged messages
    draw_text(
        &format!("Non-acknowledged messages: {}", client.pending_inputs.len()),
        20.0,
        y_offset - 20.0,
        20.0,
        DARKGRAY,
    );

    for (id, entity) in client.entities.iter() {
        let entity_colour = if entity.entity_id == 1 { BLUE } else { RED };
        draw_rectangle(entity.x, y_offset + 20.0, 20.0, 20.0, entity_colour);
    }
}

fn draw_server_perspective(s: RefMut<Server>) {
    draw_rectangle_lines(10.0, 220.0, screen_width() - 20.0, 120.0, 2.0, DARKGRAY);

    for (id, entity) in s.entities.iter() {
        // Draw the entity as a rectangle
        let mut colour = RED;
        if entity.entity_id == 1 {
            colour = BLUE;
        }
        draw_rectangle(entity.x, 260.0, 20.0, 20.0, colour);
    }

    draw_text(
        &format!(
            "Last Acknowledged: Player 0 - {} Player 1 - {}",
            s.last_processed_inputs.get(&1).unwrap_or(&0.0),
            s.last_processed_inputs.get(&2).unwrap_or(&0.0)
        ),
        20.0,
        240.0,
        20.0,
        DARKGRAY,
    );
}

#[macroquad::main("Netcode Example")]
async fn main() {
    // Create a server
    let server = Server::new();

    // Add two clients to the server
    let client1 = Server::add_client(server.clone());
    let client2 = Server::add_client(server.clone());

    // List the clients (for debugging)
    server.borrow().list_clients();

    // connect the two clients

    loop {
        // Get the last frame time
        let delta_time = get_frame_time();
        // println!("Delta time: {}", delta_time);

        //check for left and right arrow key press
        {
            let mut client1 = client1.borrow_mut();
            client1.key_left = is_key_down(KeyCode::Left);
            client1.key_right = is_key_down(KeyCode::Right);
        }

        // Update client2 (borrow mutably)
        {
            let mut client2 = client2.borrow_mut();
            client2.key_left = is_key_down(KeyCode::A);
            client2.key_right = is_key_down(KeyCode::D);
        }

        // Clear the screen for each frame
        clear_background(LIGHTGRAY);

        {
            let mut server = server.borrow_mut();
            draw_server_perspective(server);
        }

        {
            let mut client1 = client1.borrow_mut();
            draw_client_entities(client1, 120.0);
        }

        {
            let mut client2 = client2.borrow_mut();
            draw_client_entities(client2, 450.0);
        }

        {
            let client1_ui = client1.clone();
            let client2_ui = client2.clone();
            widgets::Window::new(hash!(), vec2(400., 200.), vec2(200., 220.))
                .label("Settings")
                .titlebar(true)
                .ui(&mut *root_ui(), move |ui| {
                    let mut client = client1_ui.borrow_mut(); // RefMut here
                    let mut client2 = client2_ui.borrow_mut(); // RefMut here

                    for (mut client, label) in vec![(client, "Client 1"), (client2, "Client 2")] {
                        ui.label(None, &format!("{} Entity ID: {}", label, client.entity_id));
                        ui.label(
                            None,
                            &format!("Prediction?: {}", client.client_side_prediction),
                        );
                        ui.label(
                            None,
                            &format!("Reconciliation?: {}", client.server_reconciliation),
                        );
                        ui.label(
                            None,
                            &format!("Interpolation: {}", client.entity_interpolation),
                        );
                        if ui.button(None, "Toggle Prediction") {
                            client.client_side_prediction = !client.client_side_prediction;
                        }
                        if ui.button(None, "Toggle Reconciliation") {
                            client.server_reconciliation = !client.server_reconciliation;
                        }
                        if ui.button(None, "Toggle Interpolation") {
                            client.entity_interpolation = !client.entity_interpolation;
                        }
                    }
                });
        }

        // Update server and clients at their respective intervals
        server.borrow_mut().update(delta_time);

        // Wait for the next frame
        next_frame().await;
    }
}
