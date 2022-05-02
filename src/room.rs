use std::{collections::HashMap, net::SocketAddr};

use log::{debug, warn};
use serde::{Deserialize, Serialize};

use crate::client::RoomClient;
use crate::ws_handler::Tx;

pub type RoomID = String;

#[derive(Debug)]
pub(crate) enum RoomError {
    NotFound,
    AlreadyExists,
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) enum RoomRequest {
    NewRoom,
    JoinRoom(String),
    JoinWithCode(String),
}

// Represents a room a particular peer might be in
#[derive(Debug)]
pub(crate) struct Room {
    pub clients: HashMap<SocketAddr, Tx>,
    pub room_id: String,
}

impl Room {
    pub fn new(room_id: RoomID) -> Room {
        let clients = HashMap::new();
        Room { clients, room_id }
    }

    pub fn add_client(&mut self, client: RoomClient) -> Option<RoomError> {
        debug!(
            "Adding client {} to room {:?}, clients currently in room {}",
            client,
            self,
            self.clients.len()
        );
        if let Some(_) = self.clients.insert(client.address, client.send) {
            warn!("The client already exists in room!");
            return Some(RoomError::AlreadyExists);
        }
        None
    }

    pub fn drop_client(&mut self, address: &SocketAddr) -> Option<RoomError> {
        debug!(
            "Removing client {} to room {:?}, clients currently in room {}",
            address,
            self,
            self.clients.len()
        );

        if let None = self.clients.remove_entry(address) {
            warn!("Tried to remove client {address} that wasn't there!");
            return Some(RoomError::NotFound);
        }
        None
    }
}
