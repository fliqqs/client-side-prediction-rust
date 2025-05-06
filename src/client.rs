use crate::server::Server;
use crate::{get_time_ms, Entity, LagNetwork, Message, MovementInput};
use macroquad::time::get_time;
use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::Hash;
use std::rc::{Rc, Weak};

pub(crate) struct Client {
    pub server: Weak<RefCell<Server>>, // Weak reference to avoid circular dependency
    pub update_interval: f32,
    pub time_since_last_update: f32,
    pub key_left: bool,
    pub key_right: bool,
    pub last_time: f64,
    pub input_sequence_number: u32,
    pub entity_id: u32,
    pub network: LagNetwork,
    pub entities: HashMap<u32, Entity>,
    pub client_side_prediction: bool,
    pub server_reconciliation: bool,
    pub pending_inputs: Vec<MovementInput>,
    pub latency_to_server: f32,
    pub entity_interpolation: bool,
}

impl Client {
    pub fn new(server: Weak<RefCell<Server>>, update_interval: f32) -> Self {
        // Get the current time as SystemTime

        // Convert the duration to seconds as a f64
        let last_time = get_time();

        // Set the entity id to length of the clients
        let entity_id = server.upgrade().unwrap().borrow().clients.len() as u32 + 1;

        Client {
            server,
            update_interval,
            time_since_last_update: 0.0,
            key_left: false,
            key_right: false,
            last_time, // Set the current epoch time as last_time
            input_sequence_number: 0,
            entity_id: entity_id,
            network: LagNetwork { messages: vec![] },
            entities: HashMap::new(),
            client_side_prediction: false,
            server_reconciliation: false,
            pending_inputs: Vec::new(),
            latency_to_server: 250.0,
            entity_interpolation: false,
        }
    }

    pub fn get_server(&self) -> Option<Rc<RefCell<Server>>> {
        self.server.upgrade()
    }

    pub fn process_input(&mut self) -> Option<Message> {
        let seconds = get_time();
        let mut delta_seconds = ((seconds - self.last_time) / 1000.0) as f32;

        self.last_time = seconds;

        if self.key_left {
            // println!("Client moving left! Delta time: {}", delta_seconds);
            delta_seconds = -delta_seconds;
        } else if self.key_right {
            // println!("Client moving right! Delta time: {}", delta_seconds);
        } else {
            return None;
        }

        // Create a movement input
        let movement_input = MovementInput {
            press_time: delta_seconds as f32,
            entity_id: self.entity_id,
            input_sequence_number: self.input_sequence_number,
        };

        // Increment the input sequence number
        self.input_sequence_number += 1;

        if (self.client_side_prediction) {
            // Apply the movement input to the entity immediately for client-side prediction
            if let Some(entity) = self.entities.get_mut(&self.entity_id) {
                entity.applyInput(movement_input.clone());
            }
        }

        // add to pending inputs
        self.pending_inputs.push(movement_input.clone());

        // Return the movement input as a message
        Some(Message::Movement(movement_input))
    }

    pub fn proccessServerMessages(&mut self) {
        // Process messages from the server
        // println!("Processing server message...");

        while true {
            if let Some(msg) = self.network.receive() {
                match msg {
                    Message::WorldState(world_state) => {
                        for world_state in world_state.world_state {
                            // if this is first time we see this entity, add it to the list
                            if !self.entities.contains_key(&world_state.entity_id) {
                                let entity = Entity::new(world_state.entity_id);
                                self.entities.insert(world_state.entity_id, entity);
                            }

                            if let Some(entity) = self.entities.get_mut(&world_state.entity_id) {
                                if world_state.entity_id == self.entity_id {
                                    entity.x = world_state.position;

                                    if self.server_reconciliation {
                                        // re-apply the pending inputs
                                        let mut j = 0;
                                        while (j < self.pending_inputs.len()) {
                                            let input = self.pending_inputs[j].clone();
                                            if input.input_sequence_number
                                                <= world_state.last_processed_input as u32
                                            {
                                                self.pending_inputs.remove(j);
                                            } else {
                                                // apply the input to the entity
                                                entity.applyInput(input.clone());
                                                j += 1;
                                            }
                                        }
                                    } else {
                                        self.pending_inputs.clear();
                                    }
                                } else {
                                    if !self.entity_interpolation {
                                        entity.x = world_state.position;
                                    } else {
                                        let in_ms: u128 = get_time_ms();
                                        entity.position_buffer.push((in_ms, world_state.position));
                                    }
                                }
                            }
                        }
                    }
                    Message::Movement(movement_input) => {
                        // clients wont get this
                    }
                }
            } else {
                // No more messages to process
                break;
            }
        }
    }

    pub fn interpolateEntities(&mut self, server_update_interval: f32) {
        let in_ms = get_time_ms();

        let render_timestamp = in_ms - (1000.0 * server_update_interval as f32).floor() as u128;

        for (id, entity) in &mut self.entities {
            if id == &self.entity_id {
                continue;
            }

            // Find the two authoritative positions surrounding the rendering timestamp.
            let mut buffer = entity.position_buffer.clone();

            while buffer.len() >= 2 && buffer[1].0 <= render_timestamp {
                buffer.remove(0);
            }

            if (buffer.len() >= 2
                && buffer[0].0 <= render_timestamp
                && render_timestamp <= buffer[1].0)
            {
                let t =
                    (render_timestamp - buffer[0].0) as f32 / (buffer[1].0 - buffer[0].0) as f32;
                let interpolated_position = buffer[0].1 + t * (buffer[1].1 - buffer[0].1);
                entity.x = interpolated_position;
            } else if (buffer.len() == 1 && buffer[0].0 <= render_timestamp) {
                entity.x = buffer[0].1;
            } else {
                // println!("No interpolation needed for entity {}", id);
            }
        }
    }

    pub fn update(&mut self, delta_time: f32, server_update_interval: f32) -> Option<Message> {
        // Accumulate time for the client
        self.time_since_last_update += delta_time;

        // Update the client only if the update interval has passed
        if self.time_since_last_update >= self.update_interval {
            self.time_since_last_update -= self.update_interval; // Reset time
                                                                 // println!("Client updated!");
                                                                 // println!("Client updated!");
                                                                 // Perform client update tasks, such as processing input
            self.proccessServerMessages();

            if self.entity_interpolation {
                self.interpolateEntities(server_update_interval);
            }

            self.process_input()
        } else {
            None
        }
    }
}
