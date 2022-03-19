mod room;
mod word_chooser;
mod ws_handler;

use std::{
    collections::HashMap,
    env,
    io::Error as IoError,
    sync::{Arc, RwLock},
};

use tokio::net::TcpListener;

use crate::ws_handler::RoomMap;
mod client;

#[tokio::main]
async fn main() -> Result<(), IoError> {
    let addr = env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:8080".to_string());

    let peer_map = Arc::new(RwLock::new(HashMap::new()));
    let room_map = RoomMap::new(RwLock::new(HashMap::new()));

    // Create the event loop and TCP listener we'll accept connections on.
    let try_socket = TcpListener::bind(&addr).await;
    let listener = try_socket.expect("Failed to bind");
    println!("Listening on: {}", addr);

    // Let's spawn the handling of each connection in a separate task.
    while let Ok((stream, addr)) = listener.accept().await {
        tokio::spawn(ws_handler::handle_connection(
            peer_map.clone(),
            room_map.clone(),
            stream,
            addr,
        ));
    }

    Ok(())
}
