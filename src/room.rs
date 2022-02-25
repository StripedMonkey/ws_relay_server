use std::{collections::HashMap, net::SocketAddr};

use rand::{self, rngs::OsRng, seq::SliceRandom};
use serde::{Deserialize, Serialize};
use serde_json;

use crate::Tx;
pub type RoomID = String;

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

pub(crate) struct RoomGenerator {
    first: Vec<String>,
    second: Vec<String>,
    third: Vec<String>,
}
impl RoomGenerator {
    pub(crate) fn room_setup(msg: String) -> Result<(RoomRequest, RoomID), serde_json::Error> {
        match serde_json::from_str::<RoomRequest>(&msg) {
            Ok(req) => match req {
                RoomRequest::NewRoom => {
                    let room_id: RoomID = "Amazing".to_string();
                    Ok((RoomRequest::NewRoom, room_id))
                }
                // TODO: This isn't the smartest interface in the world
                RoomRequest::JoinRoom(room) => Ok((RoomRequest::JoinRoom(room.clone()), room)),
            },
            Err(parse_err) => Err(parse_err),
        }
    }

    pub fn new(first: Vec<&str>, second: Vec<&str>, third: Vec<&str>) -> RoomGenerator {
        RoomGenerator {
            first: first.iter().map(|s| s.to_string()).collect(),
            second: second.iter().map(|s| s.to_string()).collect(),
            third: third.iter().map(|s| s.to_string()).collect(),
        }
    }

    pub fn generate_room_name(&mut self) -> String {
        let mut s = String::new();
        s.push_str(&self.first.choose(&mut OsRng).unwrap());
        s.push_str(&self.second.choose(&mut OsRng).unwrap());
        s.push_str(&self.third.choose(&mut OsRng).unwrap());
        s
    }
}

#[cfg(test)]
mod test {
    use crate::room::RoomRequest;

    #[test]
    fn room_format() {
        let new_room = RoomRequest::NewRoom;

        println!("New room: {}", serde_json::to_string(&new_room).unwrap());

        let existing_room = RoomRequest::JoinRoom("Amazing".to_string());
        print!("Existing room: {}", serde_json::to_string(&existing_room).unwrap());
    }
}
