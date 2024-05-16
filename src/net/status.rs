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
        matches!(self, Self::Connected { ping: _ })
    }
    pub fn is_reconnecting(&self) -> bool {
        matches!(self, Self::Reconnecting { tries: _ })
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
    status: Mutex<ConnectionStatus>,
    other_addr: Mutex<Option<SocketAddr>>,
    other_username: Mutex<Option<String>>,
    join_code: Mutex<Option<String>>,
    session_id: Mutex<u16>,
}

lazy_static! {
    static ref CONNECTION_DATA: Arc<ConnectionData> = Arc::new(ConnectionData {
        status: Mutex::new(ConnectionStatus::Disconnected),
        other_addr: Mutex::new(None),
        other_username: Mutex::new(None),
        join_code: Mutex::new(None),
        session_id: Mutex::new(CONNECT_SESSION_ID),
    });
}

pub async fn get_other_addr() -> Option<SocketAddr> {
    *CONNECTION_DATA.other_addr.lock().await
}

pub async fn set_other_addr(addr: SocketAddr) {
    *CONNECTION_DATA.other_addr.lock().await = Some(addr)
}

pub async fn remove_other_addr() {
    *CONNECTION_DATA.other_addr.lock().await = None
}

pub async fn get_other_username() -> Option<String> {
    CONNECTION_DATA.other_username.lock().await.clone()
}

pub async fn set_other_username(name: &str) {
    *CONNECTION_DATA.other_username.lock().await = Some(name.to_owned())
}

pub async fn remove_other_username() {
    *CONNECTION_DATA.other_username.lock().await = None
}

pub async fn get_connection_status() -> ConnectionStatus {
    *CONNECTION_DATA.status.lock().await
}

pub async fn set_connection_status(status: ConnectionStatus) {
    *CONNECTION_DATA.status.lock().await = status
}

pub async fn get_connection_ping() -> Option<u128> {
    match CONNECTION_DATA.status.lock().await.clone() {
        ConnectionStatus::Connected { ping } => Some(ping),
        _ => None,
    }
}

pub async fn set_connection_ping(new_ping: u128) {
    if let ConnectionStatus::Connected { ping } = &mut *CONNECTION_DATA.status.lock().await {
        *ping = new_ping;
    }
}
pub async fn set_reconnect_tries(new_tries: u8) {
    if let ConnectionStatus::Reconnecting { tries } = &mut *CONNECTION_DATA.status.lock().await {
        *tries = new_tries;
    }
}

pub async fn get_join_code() -> Option<String> {
    CONNECTION_DATA.join_code.lock().await.clone()
}

pub async fn set_join_code(code: &str) {
    *CONNECTION_DATA.join_code.lock().await = Some(code.to_string())
}

pub async fn get_session_id() -> u16 {
    *CONNECTION_DATA.session_id.lock().await
}

pub async fn set_session_id(session_id: u16) {
    *CONNECTION_DATA.session_id.lock().await = session_id
}
