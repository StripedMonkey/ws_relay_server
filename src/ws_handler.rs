use std::net::SocketAddr;

use futures_channel::mpsc::{unbounded, UnboundedSender};
use futures_util::{
    future, pin_mut,
    stream::{SplitStream, TryStreamExt},
    StreamExt,
};
use log::{debug, error, info};
use serde::Serialize;
use tokio::net::TcpStream;
use tokio_tungstenite::WebSocketStream;
use tungstenite::protocol::Message;

use crate::{
    client::RoomClient,
    room::RoomRequest,
    room_context::{RoomContext, WrappedRoom},
};

#[derive(Debug, Serialize)]
struct UserJoined {
    ip_joined: SocketAddr,
}

#[derive(Debug, Serialize)]
struct UserLeft {
    ip_left: SocketAddr,
}

pub(crate) type Tx = UnboundedSender<Message>;

pub(crate) async fn handle_connection(
    room_context: RoomContext,
    raw_stream: TcpStream,
    address: SocketAddr,
) {
    info!("Incoming TCP connection from: {}", address);

    let ws_stream = tokio_tungstenite::accept_async(raw_stream)
        .await
        .expect("Error during the websocket handshake occurred");
    info!("WebSocket connection established: {}", address);

    let (tx, rx) = unbounded();
    let (outgoing, mut incoming) = ws_stream.split();
    let client = RoomClient { address, send: tx };
    // First message through the websocket sets up the room as per our "protocol"
    // TODO: Define more clearly what and how this works
    let room: WrappedRoom;
    if let Ok(value) = initialize_client(&mut incoming, &room_context, &client).await {
        room = value;
    } else {
        return;
    };
    broadcast_message(
        &room,
        address,
        Message::Text(serde_json::to_string(&UserJoined { ip_joined: address }).unwrap()),
    );
    
    let boadcast_handler = incoming.try_for_each(|msg| {
        debug!(
            "Room {room:?}: From {address}: {contents:#?}",
            room = &room_context.peer_map.read().unwrap().get(&address).unwrap(),
            contents = msg.to_text().unwrap()
        );
        // We want to broadcast the message to everyone in the room except ourselves.
        broadcast_message(&room, address, msg.clone());
        future::ok(())
    });

    let recieve_handler = rx.map(Ok).forward(outgoing);

    pin_mut!(boadcast_handler, recieve_handler);
    future::select(boadcast_handler, recieve_handler).await;

    info!("{} disconnected", &client.address);
    cleanup_connection(room_context, address, room, client);
}

enum RoomInitializationError {
    Unknown,
    InvalidJsonMessage(serde_json::Error),
    InvalidMessage,
    ClientInitError,
}

async fn initialize_client(
    incoming: &mut SplitStream<WebSocketStream<TcpStream>>,
    room_context: &RoomContext,
    client: &RoomClient,
) -> Result<WrappedRoom, RoomInitializationError> {
    let message: RoomRequest = match incoming.next().await {
        None => {
            error!("Failed to get a first message!");
            // I don't actually know exactly what the requirements are for us to get None.
            return Err(RoomInitializationError::Unknown);
        }
        Some(result) => match result {
            Ok(msg) => match msg {
                Message::Text(content) => {
                    info!("Parsing message {content:?}");
                    match serde_json::from_str::<RoomRequest>(&content) {
                        Ok(req) => req,
                        Err(e) => {
                            error!("Failed to parse message from client: {e:?}");
                            return Err(RoomInitializationError::InvalidJsonMessage(e));
                        }
                    }
                }
                unhandled => {
                    error!("Unexpected first message type from client! {:?}", unhandled);
                    return Err(RoomInitializationError::InvalidMessage);
                }
            },
            Err(err) => {
                error!(
                    "Failed to correctly receive initializing message from client! {:?}",
                    err
                );
                return Err(RoomInitializationError::InvalidMessage);
            }
        },
    };
    let room: WrappedRoom = match room_context.init_client(client.clone(), message) {
        Ok(room) => {
            let room_name = room.lock().unwrap().room_id.clone();
            client
                .send
                .unbounded_send(Message::Text(
                    serde_json::to_string(&RoomRequest::JoinRoom(room_name)).unwrap(),
                ))
                .unwrap();
            room
        }
        Err(e) => {
            error!("Failed to initialize client: {client}! Error {e:?}");
            return Err(RoomInitializationError::ClientInitError);
        }
    };
    Ok(room)
}

fn broadcast_message(room: &WrappedRoom, ignored_address: SocketAddr, msg: Message) {
    let _broadcast_recipients = room // God
        .lock() // it's
        .unwrap() // Too
        .clients // Long
        .iter() // Save
        .filter(|(peer_addr, _)| **peer_addr != ignored_address) // Me
        .map(|(_, ws_sink)| ws_sink) // Please
        .for_each(|recp| recp.unbounded_send(msg.clone()).unwrap());
}

fn cleanup_connection(
    room_context: RoomContext,
    address: SocketAddr,
    room: WrappedRoom,
    client: RoomClient,
) {
    room_context.peer_map.write().unwrap().remove(&address);
    room.lock().unwrap().drop_client(&client.address);
    if room.lock().unwrap().clients.is_empty() {
        let room_id = &room.lock().unwrap().room_id;
        info!("Room '{room_id}' has no more peers. Removing...",);
        room_context.room_map.write().unwrap().remove(room_id);
    } else {
        broadcast_message(
            &room,
            address,
            Message::Text(serde_json::to_string(&UserLeft { ip_left: address }).unwrap()),
        )
    }
}
