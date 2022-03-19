use std::net::SocketAddr;

use futures_channel::mpsc::{unbounded, UnboundedSender};
use futures_util::{future, pin_mut, stream::TryStreamExt, StreamExt};
use log::{debug, error, info};
use tokio::net::TcpStream;
use tungstenite::protocol::Message;

use crate::{
    client::RoomClient,
    room::RoomRequest,
    room_context::{RoomContext, WrappedRoom},
};

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
    let message: RoomRequest = match incoming.next().await {
        None => {
            error!("Failed to get a first message!");
            return;
        }
        Some(result) => match result {
            Ok(msg) => match msg {
                Message::Text(content) => {
                    info!("Parsing message {content}");
                    match serde_json::from_str::<RoomRequest>(&content) {
                        Ok(req) => req,
                        Err(e) => {
                            error!("Failed to parse message from client: {e}");
                            return;
                        }
                    }
                }
                unhandled => {
                    error!("Unexpected first message type from client! {:?}", unhandled);
                    return;
                }
            },
            Err(err) => {
                error!(
                    "Failed to correctly receive initializing message from client! {:?}",
                    err
                );
                return;
            }
        },
    };

    // The first message from the client tells if it's creating a new session or joining a session
    let room = match room_context.init_client(client.clone(), message) {
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
            error!("Failed to initialize client: {client}! Error {e:?}"); // Currently impossible?
            return;
        }
    };

    let broadcast_incoming = incoming.try_for_each(|msg| {
        debug!(
            "Received a message from {} for room {}: {:#?}",
            address,
            &room_context.peer_map.read().unwrap().get(&address).unwrap(),
            msg.to_text().unwrap()
        );
        // We want to broadcast the message to everyone in the room except ourselves.
        let _broadcast_recipients =
            room // God
                .lock() // Is
                .unwrap() // Too
                .clients // Long
                .iter() // Save
                .filter(|(peer_addr, _)| peer_addr != &&address) // Me
                .map(|(_, ws_sink)| ws_sink) // Please
                .for_each(|recp| recp.unbounded_send(msg.clone()).unwrap());
        future::ok(())
    });

    let receive_from_others = rx.map(Ok).forward(outgoing);

    pin_mut!(broadcast_incoming, receive_from_others);
    future::select(broadcast_incoming, receive_from_others).await;

    info!("{} disconnected", &client.address);
    cleanup_connection(room_context, address, room, client);
}

fn cleanup_connection(
    room_context: RoomContext,
    address: SocketAddr,
    room: WrappedRoom,
    client: RoomClient,
) {
    room_context.peer_map.write().unwrap().remove(&address);
    match room.lock().unwrap().remove_client(&client.address) {
        Ok(_worked) => { /*Working as intended */ }
        Err(_) => {
            error!("Warning! Tried to remove a client that wasn't there!")
        }
    };
    if room.lock().unwrap().clients.is_empty() {
        let room_id = &room.lock().unwrap().room_id;
        info!("Room '{room_id}' has no more peers. Removing...",);
        room_context.room_map.write().unwrap().remove(room_id);
    }
}
