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
use std::time::{Duration, SystemTime, UNIX_EPOCH};

struct Entity {
    x: f32,
    speed: i32,
    entity_id: u32,
    position_buffer: Vec<(u128, f32)>,
}

impl Entity {
    fn new(entity_id: u32) -> Self {
        Entity {
            x: 40.0,
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
        let now = SystemTime::now();
        let duration_since_epoch = now.duration_since(UNIX_EPOCH).expect("Time went backwards");
        let in_ms = duration_since_epoch.as_millis();

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
            let now = SystemTime::now();
            let duration_since_epoch = now.duration_since(UNIX_EPOCH).expect("Time went backwards");

            let in_ms = duration_since_epoch.as_millis();

            if in_ms >= v.receive_time {
                let message = self.messages.remove(i);
                return Some(message.payload);
            }
        }
        return None;
    }
}

// function for drawing things on the screen
fn draw_client_entities(client: RefMut<Client>, y_offset: f32) {
    for (id, entity) in client.entities.iter() {
        draw_text(
            &format!("Client Entity {}: x = {}", id, entity.x),
            20.0,
            y_offset + (*id as f32 * 20.0),
            20.0,
            DARKGRAY,
        );
        // Draw the entity as a rectangle
        draw_rectangle(
            entity.x,
            y_offset + 40.0 + (*id as f32 * 20.0),
            20.0,
            20.0,
            BLUE,
        );
    }
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
        draw_text("Server View", 20.0, 20.0, 30.0, DARKGRAY);

        // Draw Server Entities
        for (id, entity) in server.borrow().entities.iter() {
            draw_text(
                &format!("Entity {}: x = {}", id, entity.x),
                20.0,
                20.0 + (*id as f32 * 20.0),
                20.0,
                DARKGRAY,
            );
            // Draw the entity as a rectangle
            let mut colour = RED;
            if entity.entity_id == 1 {
                colour = BLUE;
            }
            draw_rectangle(entity.x, 40.0 + (*id as f32 * 20.0), 20.0, 20.0, colour);
        }

        {
            let mut client1 = client1.borrow_mut();
            draw_client_entities(client1, 150.0);
        }

        {
            let mut client2 = client2.borrow_mut();
            draw_client_entities(client2, 300.0);
        }

        {
            let client1_ui = client1.clone();
            let client2_ui = client2.clone();
            widgets::Window::new(hash!(), vec2(400., 200.), vec2(320., 400.))
                .label("Settings")
                .titlebar(true)
                .ui(&mut *root_ui(), move |ui| {
                    let mut client = client1_ui.borrow_mut(); // RefMut here
                    let mut client2 = client2_ui.borrow_mut(); // RefMut here

                    ui.tree_node(hash!(), "Client 1", |ui| {
                        ui.label(None, &format!("Client 1 Entity ID: {}", client.entity_id));
                        ui.label(
                            None,
                            &format!("Client 1 Prediction?: {}", client.client_side_prediction),
                        );
                        ui.label(
                            None,
                            &format!(
                                "Client 1 Server Reconciliation?: {}",
                                client.server_reconciliation
                            ),
                        );
                        ui.label(
                            None,
                            &format!(
                                "Client 1 Entity Interpolation: {}",
                                client.entity_interpolation
                            ),
                        );
                        if ui.button(None, "Toggle Prediction Client 1") {
                            client.client_side_prediction = !client.client_side_prediction;
                        }
                        if ui.button(None, "Server Reconciliation Client 1") {
                            client.server_reconciliation = !client.server_reconciliation;
                        }
                        if ui.button(None, "Toggle Interpolation Client 1") {
                            client.entity_interpolation = !client.entity_interpolation;
                        }
                        ui.label(None, &format!("Client 1 lag: {}", client.latency_to_server));
                        ui.slider(
                            hash!(),
                            "[5 .. 500]",
                            5f32..5000f32,
                            &mut client.latency_to_server,
                        );
                    });

                    ui.tree_node(hash!(), "Client 2", |ui| {
                        ui.label(None, &format!("Client 2 Entity ID: {}", client2.entity_id));
                        ui.label(
                            None,
                            &format!("Client 2 Prediction?: {}", client2.client_side_prediction),
                        );
                        if ui.button(None, "Toggle Prediction Client 2") {
                            client2.client_side_prediction = !client2.client_side_prediction;
                        }
                        if ui.button(None, "Server Reconciliation Client 2") {
                            client2.server_reconciliation = !client2.server_reconciliation;
                        }
                        ui.label(
                            None,
                            &format!("Client 2 lag: {}", client2.latency_to_server),
                        );
                        ui.slider(
                            hash!(),
                            "[5 .. 500]",
                            5f32..5000f32,
                            &mut client2.latency_to_server,
                        );
                    });
                });
        }

        // Update server and clients at their respective intervals
        server.borrow_mut().update(delta_time);

        // Wait for the next frame
        next_frame().await;
    }
}
