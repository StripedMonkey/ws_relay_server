use std::io::Error as IoError;

use clap::Parser;
use log::{info, LevelFilter};
use tokio::net::TcpListener;

use crate::room_context::RoomContext;

mod client;
mod room;
mod room_context;
mod word_chooser;
mod ws_handler;

#[derive(Debug, Parser)]
#[clap(author, version, about, long_about = None)]
struct CMDLineArgs {
    #[clap(default_value = "127.0.0.1:8080")]
    #[clap(short, long)]
    address: String,
    #[clap(default_value_t = LevelFilter::Info)]
    #[clap(short, long)]
    debug: log::LevelFilter,
}

#[tokio::main]
async fn main() -> Result<(), IoError> {
    let args = CMDLineArgs::parse();

    pretty_env_logger::formatted_timed_builder()
        .filter_level(args.debug)
        .filter_module("tungstenite", LevelFilter::Error)
        .init();

    let room_context = RoomContext::new();

    // Create the event loop and TCP listener we'll accept connections on.
    let try_socket = TcpListener::bind(&args.address).await;
    let listener = try_socket.expect("Failed to bind");
    info!("Listening on: {}", args.address);

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
