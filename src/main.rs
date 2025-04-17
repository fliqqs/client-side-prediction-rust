mod client;
mod server;

use macroquad::prelude::*;
use std::time::{SystemTime, UNIX_EPOCH};
use crate::server::Server;

struct Entity {
    x: f32,
    speed: i32,
    entity_id: u32,
}

impl Entity {
    fn new(entity_id: u32) -> Self {
        Entity {
            x: 40.0,
            speed: 10000,
            entity_id,
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
    receive_time: f32,
    payload: Message,
}

struct LagNetwork {
    messages: Vec<NetworkMessage>,
}

impl LagNetwork {
    fn send(&mut self, lag_ms: f32, message: Message) {
        let now = SystemTime::now();
        let duration_since_epoch = now.duration_since(UNIX_EPOCH).expect("Time went backwards");


        //set recv time to time now + lag_ms
        let current_time = duration_since_epoch.as_secs_f32();
        let receive_time = current_time + lag_ms / 1000.0;


        // make the NetworkMessage
        let network_message = NetworkMessage {
            receive_time: receive_time,
            payload: message,
        };

        println!("Sending message: {:?}", network_message.payload);

        self.messages.push(network_message);
    }

    fn receive(&mut self) -> Option<Message> {

        println!("what is msg length: {}", self.messages.len());

        if self.messages.len() == 0 {
            return None;
        }

        for (i, v) in self.messages.iter().enumerate() {
            let now = SystemTime::now();
            let duration_since_epoch = now.duration_since(UNIX_EPOCH).expect("Time went backwards");
            let current_time = duration_since_epoch.as_secs_f32();

            println!("current time: {}", current_time);
            println!("receive time: {}", v.receive_time);

            if current_time >= v.receive_time {
                let message = self.messages.remove(i);
                println!("returning : {:?}", message.payload);
                return Some(message.payload);
            }
        }
        return None;
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

        // Drawing shapes (for visualization)
        // draw_line(40.0, 40.0, 100.0, 200.0, 15.0, BLUE);
        // draw_rectangle(screen_width() / 2.0 - 60.0, 100.0, 120.0, 60.0, GREEN);
        // draw_circle(screen_width() - 30.0, screen_height() - 30.0, 15.0, YELLOW);

        draw_text("Server View", 20.0, 20.0, 30.0, DARKGRAY);


        // Draw Server Entities
        for (id, entity) in server.borrow().entities.iter() {
            draw_text(&format!("Entity {}: x = {}", id, entity.x), 20.0, 20.0 + (*id as f32 * 20.0), 20.0, DARKGRAY);
            // Draw the entity as a rectangle
            draw_rectangle(entity.x, 40.0 + (*id as f32 * 20.0), 20.0, 20.0, BLUE);
        }

        draw_text("Client 1 View", 20.0, 150.0, 30.0, DARKGRAY);
        // Draw Client 1 Entities
        for (id, entity) in client1.borrow().entities.iter() {
            draw_text(&format!("Client 1 Entity {}: x = {}", id, entity.x), 20.0, 150.0 + (*id as f32 * 20.0), 20.0, DARKGRAY);
            // Draw the entity as a rectangle
            draw_rectangle(entity.x, 200.0 + (*id as f32 * 20.0), 20.0, 20.0, BLUE);
        }

        // Update server and clients at their respective intervals
        server.borrow_mut().update(delta_time);

        // Wait for the next frame
        next_frame().await;
    }
}
