use std::sync::{Arc, Mutex, RwLock};
use std::{collections::HashMap, net::SocketAddr};

use serde::{Deserialize, Serialize};

use crate::client::RoomClient;
use crate::{word_chooser::generate_room_name, ws_handler::Tx};

pub type RoomID = String;
pub(crate) type WrappedRoom = Arc<Mutex<Room>>;

#[derive(Serialize, Deserialize, Debug)]
pub(crate) enum RoomRequest {
    NewRoom,
    JoinRoom(String),
}

#[derive(Clone, Default)]
pub(crate) struct RoomContext {
    pub peer_map: Arc<RwLock<HashMap<SocketAddr, RoomID>>>,
    pub room_map: Arc<RwLock<HashMap<RoomID, Arc<Mutex<Room>>>>>,
}

impl RoomContext {
    pub(crate) fn new() -> RoomContext {
        RoomContext::default()
    }

    fn get_room(&self, init_type: RoomRequest) -> Result<WrappedRoom, ()> {
        let mut room_map = self.room_map.write().unwrap();
        match init_type {
            RoomRequest::NewRoom => {
                let mut room_name = generate_room_name();
                let mut tries = 0;
                while !room_map.contains_key(&room_name) || tries < 10 {
                    log::debug!("Generated a room that was already taken! Try: {tries}");
                    room_name = generate_room_name();
                    tries += 1;
                }
                let room = Arc::new(Mutex::new(Room::new(room_name.clone())));
                room_map.insert(room_name, room.clone());
                Ok(room)
            }
            RoomRequest::JoinRoom(room) => match room_map.get(&room) {
                Some(room) => Ok(room.clone()),
                None => Err(()),
            },
        }
    }

    pub(crate) fn init_client(
        &self,
        client: RoomClient,
        init_type: RoomRequest,
    ) -> Result<Arc<std::sync::Mutex<Room>>, ()> {
        let room: WrappedRoom = self.get_room(init_type)?;

        // Add the client to the room and client maps
        self.peer_map
            .write()
            .unwrap()
            .insert(client.address, room.lock().unwrap().room_id.clone());
        room.lock().unwrap().add_client(client).unwrap();
        Ok(room) // Return the room
    }
}

// Represents a room a particular peer might be in
pub(crate) struct Room {
    pub clients: HashMap<SocketAddr, Tx>,
    pub room_id: String,
}

impl Room {
    pub fn new(room_id: String) -> Room {
        let clients = HashMap::new();
        Room { clients, room_id }
    }

    pub fn add_client(&mut self, client: RoomClient) -> Result<(), ()> {
        match self.clients.insert(client.address, client.send) {
            Some(_) => Err(()),
            None => Ok(()),
        }
    }

    pub fn remove_client(&mut self, address: &SocketAddr) -> Result<(), ()> {
        match self.clients.remove_entry(address) {
            Some(_) => Ok(()),
            None => Err(()),
        }
    }
}
