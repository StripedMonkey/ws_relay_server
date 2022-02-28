use std::{
    borrow::Cow,
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, Mutex, RwLock},
};

use futures_channel::mpsc::{unbounded, UnboundedSender};
use futures_util::{future, pin_mut, stream::TryStreamExt, SinkExt, StreamExt};
use room::RoomID;

use tokio::net::TcpStream;
use tungstenite::protocol::{frame::coding::CloseCode, CloseFrame, Message};

use crate::room::{self, Room, RoomRequest};
pub(crate) type Tx = UnboundedSender<Message>;
pub(crate) type RoomMap = Arc<RwLock<HashMap<RoomID, Arc<Mutex<room::Room>>>>>;
pub(crate) type PeerMap = Arc<RwLock<HashMap<SocketAddr, RoomID>>>;

pub(crate) async fn handle_connection(
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
    let (rq, room_id) = match parse_room_request(&incoming.next().await) {
        Some(rq) => rq,
        None => {
            // TODO: Figure out how to tell the websocket we're closing instead of just dropping it
            //       When we have an invalid request
            println!("Invalid Room Request");
            outgoing
                .reunite(incoming)
                .unwrap()
                .send(Message::Close(Some(CloseFrame {
                    code: CloseCode::Invalid,
                    reason: "Invalid Room Request".into(),
                })))
                .await
                .unwrap();
            return;
        }
    };

    match rq {
        room::RoomRequest::NewRoom => match join_new_room(&room_map, &room_id, addr, &tx) {
            Ok(_) => {
                println!("{addr} created room '{room_id}'");
            }
            Err(_) => {
                println!("{addr} failed to create room '{room_id}'");
                // Creating new room failed
                outgoing
                    .reunite(incoming)
                    .unwrap()
                    .send(Message::Close(Some(CloseFrame {
                        code: CloseCode::Error,
                        reason: "Creating Room Failed".into(),
                    })))
                    .await
                    .unwrap();
                return;
            }
        },
        room::RoomRequest::JoinRoom(_) => match join_existing_room(&room_map, &room_id, addr, tx) {
            Ok(_) => {
                println!("{addr} joining existing room '{room_id}'");
            }
            Err(_) => {
                println!("{addr} tried joining a nonexistent room '{room_id}'");
                // Attempted to join invalid room, figure out how to abort cleanly
                outgoing
                    .reunite(incoming)
                    .unwrap()
                    .send(Message::Close(Some(CloseFrame {
                        code: CloseCode::Invalid,
                        reason: "Room does not exist".into(),
                    })))
                    .await
                    .unwrap();
                return;
            }
        },
    };

    peer_map.write().unwrap().insert(addr, room_id.clone());

    let broadcast_incoming = incoming.try_for_each(|msg| {
        println!(
            "Received a message from {} for room {}: {:#}",
            addr,
            &peer_map.read().unwrap().get(&addr).unwrap(),
            msg.to_text().unwrap()
        );
        // We want to broadcast the message to everyone in the room except ourselves.
        let _broadcast_recipients = &room_map
            .read() // Oh
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
    match rmm
        .get(&room_id)
        .unwrap()
        .lock()
        .unwrap()
        .remove_client(&addr)
    {
        Ok(_worked) => { /*Working as intended */ }
        Err(_) => {
            print!("Warning! Tried to remove a client that wasn't there!")
        }
    };
    if rmm
        .get(&room_id)
        .unwrap()
        .lock()
        .unwrap()
        .clients
        .is_empty()
    {
        println!("Room '{room_id}' has no more peers. Removing...");
        rmm.remove(&room_id);
    }
}

fn join_existing_room(
    room_map: &RoomMap,
    room_id: &String,
    addr: SocketAddr,
    tx: UnboundedSender<Message>,
) -> Result<Arc<Mutex<Room>>, ()> {
    match room_map.read().unwrap().get(room_id) {
        Some(real_room) => match real_room.clone().lock().unwrap().add_client(addr, tx) {
            Ok(_) => Ok(real_room.clone()),
            Err(_) => {
                // Already present in the room!
                Err(())
            }
        },
        None => {
            // Joined nonexistent room!
            Err(())
        }
    }
}

fn join_new_room(
    room_map: &RoomMap,
    room_id: &String,
    addr: SocketAddr,
    tx: &UnboundedSender<Message>,
) -> Result<Arc<Mutex<Room>>, ()> {
    println!("{addr} setting up new room '{room_id}'");
    let mut room_hashmap = room_map.write().unwrap();
    if room_hashmap.contains_key(room_id) {
        return Err(());
    }
    room_hashmap.insert(
        room_id.clone(),
        Arc::new(Mutex::new(Room::new(addr, tx.clone()))),
    );
    tx.unbounded_send(Message::Text(
        serde_json::to_string(&RoomRequest::JoinRoom(room_id.clone())).unwrap(),
    ))
    .unwrap();
    let x = room_hashmap.get(room_id).unwrap().clone();
    Ok(x)
}

fn parse_room_request(
    incoming: &Option<Result<Message, tungstenite::Error>>,
) -> Option<(RoomRequest, String)> {
    match incoming {
        Some(result) => match result {
            Ok(msg) => match msg {
                Message::Text(st) => match room::room_setup(st.clone()) {
                    Ok(setup) => Some(setup),
                    Err(_) => None,
                },
                _ => None, // Incorrect message body
            },
            Err(_) => todo!(),
        },
        None => todo!(),
    }
}
