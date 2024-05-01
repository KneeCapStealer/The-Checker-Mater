use std::net::SocketAddr;

pub mod net_loop;
pub mod net_utils;
pub mod p2p;

pub const CONNECT_SESSION_ID: u16 = 0x15f4;

#[derive(Clone, Copy)]
pub enum ConnectionStatus {
    Disconnected,
    Pending,
    Reconnecting { tries: u8 },
    Connected { ping: u128 },
}

pub struct ConnectionData {
    status: ConnectionStatus,
    other_addr: Option<SocketAddr>,
    join_code: Option<String>,
    session_id: Option<u16>,
}
impl ConnectionStatus {
    pub fn connected(ping: u128) -> Self {
        Self::Connected { ping }
    }
    pub fn reconnecting(tries: u8) -> Self {
        Self::Reconnecting { tries }
    }
    pub fn is_connected(&self) -> bool {
        if let Self::Connected { ping: _ } = self {
            true
        } else {
            false
        }
    }
    pub fn is_reconnecting(&self) -> bool {
        if let Self::Reconnecting { tries: _ } = self {
            true
        } else {
            false
        }
    }
}

static mut CONNECTION_DATA: ConnectionData = ConnectionData {
    status: ConnectionStatus::Disconnected,
    other_addr: None,
    join_code: None,
    session_id: None,
};

pub fn get_other_addr() -> Option<SocketAddr> {
    unsafe { CONNECTION_DATA.other_addr }
}

pub fn set_other_addr(addr: Option<SocketAddr>) {
    unsafe { CONNECTION_DATA.other_addr = addr }
}

pub fn get_connection_status() -> ConnectionStatus {
    unsafe { CONNECTION_DATA.status }
}

pub fn get_connection_status_mut() -> &'static mut ConnectionStatus {
    unsafe { &mut CONNECTION_DATA.status }
}

pub fn set_connection_status(status: ConnectionStatus) {
    unsafe { CONNECTION_DATA.status = status }
}

pub fn get_connection_ping() -> Option<u128> {
    unsafe {
        if let ConnectionStatus::Connected { ping } = CONNECTION_DATA.status {
            Some(ping)
        } else {
            None
        }
    }
}

pub fn set_connection_ping(new_ping: u128) {
    unsafe {
        if let ConnectionStatus::Connected { ping } = &mut CONNECTION_DATA.status {
            *ping = new_ping;
        }
    }
}

pub fn get_join_code() -> Option<String> {
    unsafe { CONNECTION_DATA.join_code.clone() }
}

pub fn set_join_code(code: &str) {
    unsafe { CONNECTION_DATA.join_code = Some(code.to_string()) }
}

pub fn get_session_id() -> Option<u16> {
    unsafe { CONNECTION_DATA.session_id }
}

pub fn set_session_id(session_id: u16) {
    unsafe { CONNECTION_DATA.session_id = Some(session_id) }
}
