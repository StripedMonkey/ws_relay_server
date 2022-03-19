use std::sync::{Arc, Mutex, RwLock};
use std::{collections::HashMap, net::SocketAddr};

use log;
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

pub(crate) fn room_setup(msg: String) -> Result<(RoomRequest, RoomID), serde_json::Error> {
    match serde_json::from_str::<RoomRequest>(&msg) {
        Ok(req) => match req {
            RoomRequest::NewRoom => {
                let room_id: RoomID = generate_room_name();
                Ok((RoomRequest::NewRoom, room_id))
            }
            // TODO: This isn't the smartest interface in the world
            RoomRequest::JoinRoom(room) => Ok((RoomRequest::JoinRoom(room.clone()), room)),
        },
        Err(parse_err) => Err(parse_err),
    }
}

// Represents a room a particular peer might be in
pub(crate) struct Room {
    pub clients: HashMap<SocketAddr, Tx>,
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
