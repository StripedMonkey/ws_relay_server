mod room;

use std::{
    collections::HashMap,
    env,
    io::Error as IoError,
    net::SocketAddr,
    sync::{Arc, Mutex, RwLock},
};

use futures_channel::mpsc::{unbounded, UnboundedSender};
use futures_util::{future, pin_mut, stream::TryStreamExt, StreamExt};

use room::RoomID;
use tokio::net::{TcpListener, TcpStream};
use tungstenite::protocol::Message;

use crate::room::{Room, RoomRequest};
type Tx = UnboundedSender<Message>;
type RoomMap = Arc<RwLock<HashMap<RoomID, Arc<Mutex<room::Room>>>>>;
type PeerMap = Arc<RwLock<HashMap<SocketAddr, RoomID>>>;
async fn handle_connection(
    peer_map: PeerMap,
    room_map: RoomMap,
    raw_stream: TcpStream,
    addr: SocketAddr,
) {
    println!("Incoming TCP connection from: {}", addr);

    let ws_stream = tokio_tungstenite::accept_async(raw_stream)
        .await
        .expect("Error during the websocket handshake occurred");
    println!("WebSocket connection established: {}", addr);

    let (tx, rx) = unbounded();
    let (outgoing, mut incoming) = ws_stream.split();

    // The first message from the client tells if it's creating a new session or joining a session
    let (rq, room_id) = match &incoming.next().await {
        Some(result) => match result {
            Ok(msg) => match msg {
                Message::Text(st) => room::RoomGenerator::room_setup(st.clone()).unwrap(),
                _ => todo!("Handle incorrect first messages!"),
            },
            Err(_) => todo!(),
        },
        None => todo!(),
    };

    let room = match rq {
        room::RoomRequest::NewRoom => {
            println!("{addr} setting up new room '{room_id}'");

            // TODO: This will silently replace the room if it already exists.
            room_map.write().unwrap().insert(
                room_id.clone(),
                Arc::new(Mutex::new(Room::new(addr, tx.clone()))),
            );
            tx.unbounded_send(Message::Text(
                serde_json::to_string(&RoomRequest::JoinRoom(room_id.clone())).unwrap(),
            ))
            .unwrap();

            // TODO: The second unwrap will fail if you try to join a room that does not exist.
            room_map.read().unwrap().get(&room_id).unwrap().clone()
        }
        room::RoomRequest::JoinRoom(_) => {
            println!("{addr} joining existing room '{room_id}'");
            let x = room_map.read().unwrap().get(&room_id).unwrap().clone();
            x.lock().unwrap().add_client(addr, tx).unwrap();
            x
        }
    };
    peer_map.write().unwrap().insert(addr, room_id.clone());

    let broadcast_incoming = incoming.try_for_each(|msg| {
        println!(
            "Received a message from {}: {}",
            addr,
            msg.to_text().unwrap()
        );
        // We want to broadcast the message to everyone in the room except ourselves.
        let _broadcast_recipients = &room_map
            .read()// Oh
            .unwrap() // My
            .get(&room_id) // God
            .unwrap() // This
            .lock() // Is
            .unwrap() // Too
            .clients // Long
            .iter() // Save
            .filter(|(peer_addr, _)| peer_addr != &&addr) // Me
            .map(|(_, ws_sink)| ws_sink) // Please
            .for_each(|recp| recp.unbounded_send(msg.clone()).unwrap());
        future::ok(())
    });

    let receive_from_others = rx.map(Ok).forward(outgoing);

    pin_mut!(broadcast_incoming, receive_from_others);
    future::select(broadcast_incoming, receive_from_others).await;

    println!("{addr} disconnected");
    peer_map.write().unwrap().remove(&addr);
    let mut rmm = room_map.write().unwrap();
    rmm.get(&room_id)
        .unwrap()
        .lock()
        .unwrap()
        .remove_client(&addr)
        .unwrap();
    if rmm
        .get(&room_id)
        .unwrap()
        .lock()
        .unwrap()
        .clients
        .is_empty()
    {
        println!("Room {room_id} has no more peers. Removing...");
        rmm.remove(&room_id);
    }
}

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
        tokio::spawn(handle_connection(
            peer_map.clone(),
            room_map.clone(),
            stream,
            addr,
        ));
    }

    Ok(())
}
