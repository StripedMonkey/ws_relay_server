use log::{debug, error, info, trace};

use crate::{
    client::RoomClient,
    room::{Room, RoomID, RoomRequest},
    word_chooser::generate_room_name,
};
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, Mutex, RwLock},
};

#[derive(Debug)]
pub(crate) enum RoomContextError {
    NonexistentRoom,
    AlreadyExists,
}

pub(crate) type WrappedRoom = Arc<Mutex<Room>>;

#[derive(Clone, Default)]
pub(crate) struct RoomContext {
    pub peer_map: Arc<RwLock<HashMap<SocketAddr, RoomID>>>,
    pub room_map: Arc<RwLock<HashMap<RoomID, WrappedRoom>>>,
}

impl RoomContext {
    pub(crate) fn new() -> RoomContext {
        RoomContext::default()
    }

    pub fn get_room_by_addr(&self, address: SocketAddr) -> Option<WrappedRoom> {
        if let Some(room_name) = self.peer_map.read().unwrap().get(&address) {
            self.room_map.read().unwrap().get(room_name).cloned()
        } else {
            None
        }
    }

    fn get_room(&self, init_type: RoomRequest) -> Result<WrappedRoom, RoomContextError> {
        let mut room_map = self.room_map.write().unwrap();
        match init_type {
            RoomRequest::NewRoom => {
                info!("Generating new room...");
                let mut room_name: String = generate_room_name();
                let mut tries: usize = 0;
                while room_map.contains_key(&room_name) && tries < 10 {
                    log::debug!("Generated a room that was already taken! Try: {tries}");
                    room_name = generate_room_name();
                    tries += 1;
                }
                let room: WrappedRoom = Arc::new(Mutex::new(Room::new(room_name.clone())));
                room_map.insert(room_name, room.clone());
                Ok(room)
            }
            RoomRequest::JoinRoom(room) => {
                info!("Trying to join existing room...");
                match room_map.get(&room) {
                    Some(room) => Ok(room.clone()),
                    None => {
                        error!("Failed to get room. Doesn't exist?");
                        return Err(RoomContextError::NonexistentRoom);
                    }
                }
            }
            RoomRequest::JoinWithCode(room_name) => {
                if room_map.contains_key(&room_name) {
                    return Err(RoomContextError::AlreadyExists);
                }
                let room: WrappedRoom = Arc::new(Mutex::new(Room::new(room_name.clone())));
                room_map.insert(room_name, room.clone());
                Ok(room)
            }
        }
    }

    pub(crate) fn init_client(
        &self,
        client: RoomClient,
        init_type: RoomRequest,
    ) -> Result<Arc<std::sync::Mutex<Room>>, RoomContextError> {
        let room: WrappedRoom = self.get_room(init_type)?;

        // Add the client to the room and client maps
        self.peer_map
            .write()
            .unwrap()
            .insert(client.address, room.lock().unwrap().room_id.clone());
        room.lock().unwrap().add_client(client);
        Ok(room) // Return the room
    }
}
