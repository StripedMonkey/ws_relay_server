use std::{collections::HashMap, net::SocketAddr};

use serde::{Deserialize, Serialize};
use tungstenite::Message;

use crate::client::RoomClient;
use crate::ws_handler::Tx;

pub type RoomID = String;

trait RoomAssists<'s> {
    fn add_client(&self, client: RoomClient) -> Result<(), ()>;
    fn remove_client(&self, address: &SocketAddr) -> Result<(), ()>;
    fn filtered_announce<P>(&'s self, filter_by: P, message: Message) -> Result<(), ()>;
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) enum RoomRequest {
    NewRoom,
    JoinRoom(String),
}

// Represents a room a particular peer might be in
pub(crate) struct Room {
    pub clients: HashMap<SocketAddr, Tx>,
    pub room_id: String,
}

impl Room {
    pub fn new(room_id: RoomID) -> Room {
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
