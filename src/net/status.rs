use std::{net::SocketAddr, sync::Arc};

use lazy_static::lazy_static;
use tokio::sync::Mutex;

pub const CONNECT_SESSION_ID: u16 = 0x15f4;

#[derive(Clone, Copy, Debug)]
pub enum ConnectionStatus {
    Disconnected,
    PendingConnection,
    Reconnecting { tries: u8 },
    Connected { ping: u128 },
}

impl ConnectionStatus {
    /// A `ConnectionStatus::Connected` with `ping` set to `0`
    pub fn connected() -> Self {
        Self::Connected { ping: 0 }
    }
    /// A `ConnectionStatus::Reconnecting` with `tries` set to `0`
    pub fn reconnecting() -> Self {
        Self::Reconnecting { tries: 0 }
    }
    pub fn is_connected(&self) -> bool {
        match self {
            Self::Connected { ping: _ } => true,
            _ => false,
        }
    }
    pub fn is_reconnecting(&self) -> bool {
        match self {
            Self::Reconnecting { tries: _ } => true,
            _ => false,
        }
    }
    pub fn can_send(&self) -> bool {
        match self {
            Self::Disconnected => false,
            Self::PendingConnection => true,
            Self::Reconnecting { tries: _ } => true,
            Self::Connected { ping: _ } => true,
        }
    }
}
pub struct ConnectionData {
    status: ConnectionStatus,
    other_addr: Option<SocketAddr>,
    other_username: Option<String>,
    join_code: Option<String>,
    session_id: u16,
}

lazy_static! {
    static ref CONNECTION_DATA: Arc<Mutex<ConnectionData>> = Arc::new(Mutex::new(ConnectionData {
        status: ConnectionStatus::Disconnected,
        other_addr: None,
        other_username: None,
        join_code: None,
        session_id: CONNECT_SESSION_ID,
    }));
}

pub async fn get_other_addr() -> Option<SocketAddr> {
    CONNECTION_DATA.lock().await.other_addr.clone()
}

pub async fn set_other_addr(addr: SocketAddr) {
    CONNECTION_DATA.lock().await.other_addr = Some(addr.clone())
}

pub async fn remove_other_addr() {
    CONNECTION_DATA.lock().await.other_addr = None
}

pub async fn get_other_username() -> Option<String> {
    CONNECTION_DATA.lock().await.other_username.clone()
}

pub async fn set_other_username(name: &str) {
    CONNECTION_DATA.lock().await.other_username = Some(name.to_owned())
}

pub async fn remove_other_username() {
    CONNECTION_DATA.lock().await.other_username = None
}

pub async fn get_connection_status() -> ConnectionStatus {
    CONNECTION_DATA.lock().await.status.clone()
}

pub async fn set_connection_status(status: ConnectionStatus) {
    CONNECTION_DATA.lock().await.status = status
}

pub async fn get_connection_ping() -> Option<u128> {
    match CONNECTION_DATA.lock().await.status {
        ConnectionStatus::Connected { ping } => Some(ping),
        _ => None,
    }
}

pub async fn set_connection_ping(new_ping: u128) {
    if let ConnectionStatus::Connected { ping } = &mut CONNECTION_DATA.lock().await.status {
        *ping = new_ping;
    }
}
pub async fn set_reconnect_tries(new_tries: u8) {
    if let ConnectionStatus::Reconnecting { tries } = &mut CONNECTION_DATA.lock().await.status {
        *tries = new_tries;
    }
}

pub async fn get_join_code() -> Option<String> {
    CONNECTION_DATA.lock().await.join_code.clone()
}

pub async fn set_join_code(code: &str) {
    CONNECTION_DATA.lock().await.join_code = Some(code.to_string())
}

pub async fn get_session_id() -> u16 {
    CONNECTION_DATA.lock().await.session_id
}

pub async fn set_session_id(session_id: u16) {
    CONNECTION_DATA.lock().await.session_id = session_id
}
