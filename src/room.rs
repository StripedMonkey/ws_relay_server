use std::{collections::HashMap, net::SocketAddr};

use serde::{Deserialize, Serialize};

use crate::{word_chooser::generate_room_name, ws_handler::Tx};

pub type RoomID = String;

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
    pub fn new(initiating_address: SocketAddr, send: Tx) -> Room {
        let mut clients = HashMap::new();
        clients.insert(initiating_address, send);
        Room { clients }
    }

    pub fn add_client(&mut self, address: SocketAddr, send: Tx) -> Result<(), (SocketAddr, Tx)> {
        match self.clients.insert(address, send) {
            Some(already_existing_tx) => Err((address, already_existing_tx)),
            None => Ok(()),
        }
    }

    pub fn remove_client(&mut self, address: &SocketAddr) -> Result<(SocketAddr, Tx), ()> {
        match self.clients.remove_entry(address) {
            Some(entry) => Ok(entry),
            None => Err(()),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) enum RoomRequest {
    NewRoom,
    JoinRoom(String),
}
