use crate::client::Client;
use crate::{world_state, Entity, LagNetwork, Message, WorldStateMessage};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub(crate) struct Server {
    pub(crate) clients: Vec<Rc<RefCell<Client>>>,
    network: LagNetwork,
    time_since_last_update: f32,
    pub(crate) update_interval: f32, // 20ms for server update interval
    pub(crate) entities: HashMap<u32, Entity>,
    pub(crate) last_processed_inputs: HashMap<u32, f32>,
}

impl Server {
    pub(crate) fn new() -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Self {
            clients: Vec::new(),
            network: LagNetwork { messages: vec![] },
            time_since_last_update: 0.0,
            update_interval: 0.1, // 100 ms
            entities: HashMap::new(),
            last_processed_inputs: HashMap::new(),
        }))
    }

    pub(crate) fn add_client(server: Rc<RefCell<Self>>) -> Rc<RefCell<Client>> {
        let client = Rc::new(RefCell::new(Client::new(
            Rc::downgrade(&server), // weak reference to the server
            0.02,                   // 20 ms
        )));

        server.borrow_mut().clients.push(client.clone());

        // Create an entity for the client
        let entity_id = server.borrow().clients.len() as u32;

        println!("Creating entity for client: with entity id: {}", entity_id);

        let entity = Entity::new(entity_id);
        server.borrow_mut().entities.insert(entity_id, entity);

        client
    }

    pub(crate) fn list_clients(&self) {
        println!("Server has {} clients.", self.clients.len());
    }

    fn processInputs(&mut self) {
        while true {
            if let Some(msg) = self.network.receive() {
                match msg {
                    Message::Movement(movement_input) => {
                        // update the entry if it exists
                        if let Some(entity) = self.entities.get_mut(&movement_input.entity_id) {
                            self.last_processed_inputs.insert(
                                movement_input.entity_id,
                                movement_input.input_sequence_number as f32,
                            );
                            entity.applyInput(movement_input);
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
        // println!("Sending world state to clients...");

        let mut world_state = Vec::new();
        for (id, entity) in &self.entities {
            world_state.push(world_state {
                entity_id: *id,
                position: entity.x,
                last_processed_input: self.last_processed_inputs.get(id).unwrap_or(&0.0).clone(),
            });
        }

        let world_state_message = WorldStateMessage { world_state };

        // Send the world state to all clients
        for client in &self.clients {
            let mut client = client.borrow_mut();
            let latency = client.latency_to_server;
            client
                .network
                .send(latency, Message::WorldState(world_state_message.clone()));
        }
    }

    pub(crate) fn update(&mut self, delta_time: f32) {
        // tell clients to update
        self.time_since_last_update += delta_time;

        let server_update_interval = self.update_interval;

        for client in &self.clients {
            let mut messages = vec![];
            if let Some(msg) = client
                .borrow_mut()
                .update(delta_time, server_update_interval)
            {
                messages.push(msg);
            }
            for msg in messages {
                let client_latency = client.borrow().latency_to_server;
                self.network.send(client_latency, msg); // Process outside of client loop
            }
        }

        // do server updates
        self.time_since_last_update += delta_time;

        if self.time_since_last_update >= self.update_interval {
            self.time_since_last_update -= self.update_interval; // Reset time
                                                                 // println!("Server updated!");
                                                                 // println!("Server updated!");
                                                                 // Process inputs and send world state
            self.processInputs();
            self.sendWorldState();
        }
    }
}
