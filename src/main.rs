use macroquad::prelude::*;
use serde_json::json;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::{Rc, Weak};
use std::time::{SystemTime, UNIX_EPOCH};

struct Entity {
    x: f32,
    speed: i32,
    entity_id: u32,
}

impl Entity {
    fn new(entity_id: u32) -> Self {
        Entity {
            x: 0.0,
            speed: 10,
            entity_id,
        }
    }

    fn applyInput(&mut self, input: MovementInput) {
        self.x += input.press_time * self.speed as f32;
    }
}

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
        let receive_time = duration_since_epoch.as_secs_f32();

        // make the NetworkMessage
        let network_message = NetworkMessage {
            receive_time: receive_time,
            payload: message,
        };
        self.messages.push(network_message);

        //print the queue
        // println!("Network queue: {:?}", self.messages);
    }

    fn receive(&mut self) -> Option<Message> {
        if self.messages.len() == 0 {
            return None;
        }

        for (i, v) in self.messages.iter().enumerate() {
            let now = SystemTime::now();
            let duration_since_epoch = now.duration_since(UNIX_EPOCH).expect("Time went backwards");
            let current_time = duration_since_epoch.as_secs_f32();

            if current_time - v.receive_time >= 0.1 {
                let message = self.messages.remove(i);
                return Some(message.payload);
            }
        }
        return None;
    }
}

struct Server {
    clients: Vec<Rc<RefCell<Client>>>,
    network: LagNetwork,
    time_since_last_update: f32,
    update_interval: f32, // 20ms for server update interval
    entities: HashMap<u32, Entity>,
}

impl Server {
    fn new() -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Self {
            clients: Vec::new(),
            network: LagNetwork { messages: vec![] },
            time_since_last_update: 0.0,
            update_interval: 1.0, // 20ms
            entities: HashMap::new(),
        }))
    }

    fn add_client(server: Rc<RefCell<Self>>) -> Rc<RefCell<Client>> {
        let client = Rc::new(RefCell::new(Client::new(
            Rc::downgrade(&server), // weak reference to the server
            0.5,                    // update interval for the client
        )));

        server.borrow_mut().clients.push(client.clone());
        client
    }

    fn list_clients(&self) {
        println!("Server has {} clients.", self.clients.len());
    }

    fn processInputs(&mut self) {
        while true {
            if let Some(msg) = self.network.receive() {
                match msg {
                    Message::Movement(movement_input) => {
                        // update the entry if it exists
                        if let Some(entity) = self.entities.get_mut(&movement_input.entity_id) {
                            entity.applyInput(movement_input);
                            println!("Entity {} moved to x: {}", entity.entity_id, entity.x);
                        }
                    }
                    Message::WorldState(world_state) => {
                        // do nothing for now
                    }
                }
            } else {
                break;
            }
        }
    }

    fn sendWorldState(&mut self) {
        let mut world_state = Vec::new();
        for (id, entity) in &self.entities {
            world_state.push(world_state {
                entity_id: *id,
                position: entity.x,
                last_processed_input: 0.0,
            });
        }

        let world_state_message = WorldStateMessage { world_state };

        // Send the world state to all clients
        for client in &self.clients {
            let mut client = client.borrow_mut();
            client
                .lag_network
                .send(0.1, Message::WorldState(world_state_message.clone()));
        }
    }

    fn update(&mut self, delta_time: f32) {
        // tell clients to update
        self.time_since_last_update += delta_time;

        let mut messages = vec![];

        for client in &self.clients {
            if let Some(msg) = client.borrow_mut().update(delta_time) {
                messages.push(msg);
            }
        }

        for msg in messages {
            self.network.send(0.0, msg); // Process outside of client loop
        }

        // do server updates
        // self.processInputs();
        // self.sendWorldState();
    }
}

struct Client {
    server: Weak<RefCell<Server>>, // Weak reference to avoid circular dependency
    update_interval: f32,
    time_since_last_update: f32,
    key_left: bool,
    key_right: bool,
    last_time: f64,
    input_sequence_number: u32,
    entity_id: u32,
    lag_network: LagNetwork,
}

impl Client {
    fn new(server: Weak<RefCell<Server>>, update_interval: f32) -> Self {
        // Get the current time as SystemTime
        let now = SystemTime::now();

        // Convert SystemTime to seconds since the Unix epoch
        let duration_since_epoch = now.duration_since(UNIX_EPOCH).expect("Time went backwards");

        // Convert the duration to seconds as a f64
        let last_time = duration_since_epoch.as_secs_f64();

        // Set the entity id to length of the clients
        let entity_id = server.upgrade().unwrap().borrow().clients.len() as u32;

        Client {
            server,
            update_interval,
            time_since_last_update: 0.0,
            key_left: false,
            key_right: false,
            last_time, // Set the current epoch time as last_time
            input_sequence_number: 0,
            entity_id: entity_id,
            lag_network: LagNetwork { messages: vec![] },
        }
    }

    fn get_server(&self) -> Option<Rc<RefCell<Server>>> {
        self.server.upgrade()
    }

    fn process_input(&mut self) -> Option<Message> {
        //current time
        let now = SystemTime::now();
        // Convert SystemTime to seconds since the Unix epoch
        let duration_since_epoch = now.duration_since(UNIX_EPOCH).expect("Time went backwards");
        // Convert the duration to seconds as a f64 (like Python's time.time())
        let seconds = duration_since_epoch.as_secs_f64();

        let delta_time = seconds - self.last_time;
        self.last_time = seconds;

        if self.key_left {
            println!("Client moving left! Delta time: {}", delta_time);
        } else if self.key_right {
            println!("Client moving right! Delta time: {}", delta_time);
        } else {
            return None;
        }

        // Create a movement input
        let movement_input = MovementInput {
            press_time: seconds as f32,
            entity_id: self.entity_id,
            input_sequence_number: self.input_sequence_number,
        };

        // Increment the input sequence number
        self.input_sequence_number += 1;

        // Return the movement input as a message
        Some(Message::Movement(movement_input))
    }

    fn proccessServerMessage(&mut self) {}

    fn update(&mut self, delta_time: f32) -> Option<Message> {
        // Accumulate time for the client
        self.time_since_last_update += delta_time;

        // Update the client only if the update interval has passed
        if self.time_since_last_update >= self.update_interval {
            self.time_since_last_update -= self.update_interval; // Reset time
                                                                 // println!("Client updated!");
            println!("Client updated!");
            // Perform client update tasks, such as processing input
            self.process_input()
        } else {
            None
        }
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
        draw_line(40.0, 40.0, 100.0, 200.0, 15.0, BLUE);
        draw_rectangle(screen_width() / 2.0 - 60.0, 100.0, 120.0, 60.0, GREEN);
        draw_circle(screen_width() - 30.0, screen_height() - 30.0, 15.0, YELLOW);

        draw_text("HELLO", 20.0, 20.0, 30.0, DARKGRAY);

        // Update server and clients at their respective intervals
        server.borrow_mut().update(delta_time);

        // Wait for the next frame
        next_frame().await;
    }
}
