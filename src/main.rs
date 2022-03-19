use std::{env, io::Error as IoError};

use log::info;
use tokio::net::TcpListener;

use crate::room_context::RoomContext;

mod client;
mod room;
mod room_context;
mod word_chooser;
mod ws_handler;

#[tokio::main]
async fn main() -> Result<(), IoError> {
    pretty_env_logger::init();
    let addr = env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:8080".to_string());

    let room_context = RoomContext::new();

    // Create the event loop and TCP listener we'll accept connections on.
    let try_socket = TcpListener::bind(&addr).await;
    let listener = try_socket.expect("Failed to bind");
    info!("Listening on: {}", addr);

    // Let's spawn the handling of each connection in a separate task.
    while let Ok((stream, addr)) = listener.accept().await {
        tokio::spawn(ws_handler::handle_connection(
            room_context.clone(),
            stream,
            addr,
        ));
    }

    Ok(())
}
