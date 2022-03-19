use crate::ws_handler::Tx;
use std::{fmt::Display, net::SocketAddr};

#[derive(Clone)]
pub(crate) struct RoomClient {
    pub address: SocketAddr,
    pub send: Tx,
}

impl Display for RoomClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.address.fmt(f)
    }
}
