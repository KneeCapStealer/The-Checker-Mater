use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use as_any::AsAny;

use crate::net::{
    get_connection_status_mut, get_join_code, p2p::P2pError, set_connection_status, set_other_addr,
    set_session_id, ConnectionStatus, CONNECT_SESSION_ID,
};

use super::{
    get_connection_status, get_other_addr, get_session_id,
    net_utils::{FromPacket, NetworkError, ToPacket},
    p2p::{
        communicate::{
            recieve_p2p_packet, recieve_packet_as_bytes, send_p2p_packet, send_packet_as_trait,
        },
        P2pRequest, P2pRequestPacket, P2pResponse, P2pResponsePacket, PieceColor,
    },
    set_connection_ping,
};

pub trait NetworkSendable: ToPacket + FromPacket + AsAny + Send + Sync {}

const REQUEST_TIMEOUT_MS: u128 = 2_500;
const DISCONNECT_TIME_MS: u128 = 5_000;
const RECONNECT_TRIES: u32 = 10;

/// Queue for outgoing packets. Follows First in First out principle.
/// Each item in the queue is a tuple of two items: The outgoing packet, and a closure that runs when
/// the outgoing packet has gotten a response.
/// If the packet isn't meant to get a response, set the closure to `None`.
static mut OUTGOING_QUEUE: Vec<(
    Box<dyn NetworkSendable>,
    Option<Arc<dyn Fn(anyhow::Result<Vec<u8>>) + Send + Sync>>,
)> = vec![];

pub fn push_outgoing_queue<S: NetworkSendable + 'static, R: NetworkSendable + Sized + 'static>(
    data: S,
    closure: Option<Arc<dyn Fn(anyhow::Result<R>) + Send + Sync>>,
) {
    unsafe {
        OUTGOING_QUEUE.push((
            Box::new(data),
            match closure {
                Some(f) => Some(Arc::new(move |recieved: anyhow::Result<Vec<u8>>| {
                    match recieved {
                        Ok(data) => {
                            if let Ok(response) = R::from_packet(data) {
                                f(Ok(response));
                            } else {
                                f(Err(NetworkError::ResponseTypeError.into()))
                            }
                        }
                        Err(_) => f(Err(NetworkError::ResponseTypeError.into())),
                    };
                })),
                None => None,
            },
        ))
    }
}

/// Pops and returns the next item in the outgoing network queue.
pub fn pop_outgoing_queue() -> Option<(
    Box<dyn NetworkSendable>,
    Option<Arc<dyn Fn(anyhow::Result<Vec<u8>>) + Send + Sync>>,
)> {
    unsafe { OUTGOING_QUEUE.pop() }
}

pub fn get_outgoing_queue_len() -> usize {
    unsafe { OUTGOING_QUEUE.len() }
}

/// The async network loop for the host.
/// The loop goes though the following points:
///     - Check for incoming messages and respond accordingly.
///     - If connected with the client:
///         - Send the next item in the Outgoing queue to the host.
///
/// # Examples
/// ```
/// tokio::spawn(async || host_network_loop(&socket));
/// ```
pub async fn host_network_loop(socket: tokio::net::UdpSocket) {
    let socket = Arc::new(socket);
    // Handle incoming responses
    tokio::spawn({
        let new_sock = socket.clone();
        async move {
            let mut time_since_ping = Instant::now();
            loop {
                // Get incoming
                match tokio::time::timeout(
                    Duration::from_secs(2),
                    recieve_p2p_packet::<P2pRequest>(&new_sock),
                )
                .await
                {
                    Ok(data) => {
                        time_since_ping = Instant::now();

                        let (incoming_packet, addr) = data.unwrap();

                        let packet = match incoming_packet.packet {
                            P2pRequestPacket::Ping => P2pResponsePacket::Pong,
                            P2pRequestPacket::Connect {
                                join_code,
                                username,
                            } => {
                                if get_other_addr().is_some() {
                                    println!(
                                        "Failed join attempt from {:?} - Game session full.",
                                        addr
                                    );
                                    P2pResponsePacket::error(P2pError::FullGameSession)
                                } else if join_code != get_join_code().unwrap() {
                                    println!(
                                        "Failed join attempt from {:?} - Wrong join code.",
                                        addr
                                    );
                                    P2pResponsePacket::error(P2pError::InvalidJoinCode)
                                } else if incoming_packet.session_id != CONNECT_SESSION_ID {
                                    println!(
                                        "Failed join attempt from {:?} - Wrong session code.",
                                        addr
                                    );
                                    P2pResponsePacket::error(P2pError::InvalidSessionId)
                                } else {
                                    println!("{} at {:?} Joined the game!", username, addr);

                                    set_session_id(rand::random::<u16>());
                                    set_connection_status(ConnectionStatus::Connected { ping: 0 });
                                    set_other_addr(Some(addr));

                                    P2pResponsePacket::Connect {
                                        client_color: PieceColor::White,
                                        host_username: "Atle".to_owned(),
                                    }
                                }
                            }
                            P2pRequestPacket::Resync => P2pResponsePacket::resync(vec![None; 64]),
                            _ => P2pResponsePacket::Resync { board: vec![] },
                        };
                        let session_id = get_session_id().unwrap_or(0);
                        let response = P2pResponse::new(session_id, packet);
                        send_p2p_packet(&new_sock, response, addr).await.unwrap();
                    }
                    Err(e) => {
                        println!("Timeout: {}", e.to_string());
                    }
                }
                if time_since_ping.elapsed().as_millis() >= DISCONNECT_TIME_MS
                    && get_other_addr().is_some()
                {
                    println!("Client at {:?} disconnected!", get_other_addr().unwrap());
                    set_other_addr(None);
                    set_session_id(CONNECT_SESSION_ID);
                }
            }
        }
    });

    // Handle outgoing queue
    tokio::spawn({
        let new_sock = socket.clone();
        async move {
            loop {
                let client_addr = match get_other_addr() {
                    Some(addr) => addr,
                    None => continue,
                };
                if !get_connection_status().is_connected() {
                    continue;
                }
                if let Some((data, callback)) = pop_outgoing_queue() {
                    println!("Sending Packet...");
                    send_packet_as_trait(&new_sock, data, client_addr)
                        .await
                        .unwrap();

                    if let Some(callback) = callback {
                        loop {
                            match recieve_packet_as_bytes(&new_sock).await {
                                Ok((bytes, _)) => {
                                    if let Ok(resp) = P2pResponse::from_packet(bytes.clone()) {
                                        if resp.packet == P2pResponsePacket::Pong {
                                            continue;
                                        }
                                    }
                                    callback(Ok(bytes));
                                }
                                Err(e) => callback(Err(NetworkError::recieve_error(
                                    &e.to_string(),
                                )
                                .into())),
                            }
                        }
                    }
                }
            }
        }
    });
}

/// The async network loop for the client.
/// The loop goes through the following points:
///     - Send the next item in the Outgoing queue to the host.
///     - If connected with the host:
///         - Send a ping.
///         - Check for incoming messages and respond accordingly.
///
/// When entering, it requires the open  UdpSocket, as well as how many pings pr. second the client
/// should send.
///
/// # Examples
/// ```
/// tokio::spawn(async || client_network_loop(&socket, 10));
/// ```
pub async fn client_network_loop(socket: tokio::net::UdpSocket, pings: usize) {
    let socket = Arc::new(socket);
    // Ping host
    tokio::spawn({
        let new_sock = socket.clone();
        let mut interval = tokio::time::interval(Duration::from_millis((1000 / pings) as u64));
        async move {
            loop {
                interval.tick().await;

                if !get_connection_status().is_connected()
                    && !get_connection_status().is_reconnecting()
                {
                    continue;
                }
                let host_addr = match get_other_addr() {
                    Some(addr) => addr,
                    None => {
                        continue;
                    }
                };
                let time = Instant::now();

                let session_id = get_session_id().unwrap();
                let ping = P2pRequest::new(session_id, P2pRequestPacket::Ping);
                send_p2p_packet::<P2pRequest>(&new_sock, ping, host_addr)
                    .await
                    .unwrap();

                match tokio::time::timeout(
                    Duration::from_millis(REQUEST_TIMEOUT_MS as u64),
                    recieve_p2p_packet::<P2pResponse>(&new_sock),
                )
                .await
                {
                    Ok(data) => {
                        let (pong, _) = data.unwrap();
                        if pong.packet == P2pResponsePacket::Pong {
                            let elapsed_ns = time.elapsed().as_nanos();
                            set_connection_status(ConnectionStatus::connected(0));
                            set_connection_ping(elapsed_ns);
                        }
                    }
                    Err(e) => {
                        println!("Ping request time out: {}", e.to_string());
                        let status = get_connection_status_mut();
                        if let ConnectionStatus::Reconnecting { tries } = status {
                            if *tries >= RECONNECT_TRIES as u8 {
                                *status = ConnectionStatus::Disconnected;
                                println!("Disconnected from host");
                            } else {
                                *tries += 1;
                            }
                        } else {
                            *status = ConnectionStatus::reconnecting(0);
                        }
                    }
                }
            }
        }
    });
    // Outgoing queue handling
    tokio::spawn({
        let new_sock = socket.clone();
        async move {
            loop {
                let host_addr = match get_other_addr() {
                    Some(addr) => addr,
                    None => continue,
                };
                if let Some((data, callback)) = pop_outgoing_queue() {
                    send_packet_as_trait(&new_sock, data, host_addr)
                        .await
                        .unwrap();

                    if let Some(callback) = callback {
                        loop {
                            match recieve_packet_as_bytes(&new_sock).await {
                                Ok((bytes, _)) => {
                                    if let Ok(resp) = P2pResponse::from_packet(bytes.clone()) {
                                        if resp.packet == P2pResponsePacket::Pong {
                                            continue;
                                        }
                                    }
                                    callback(Ok(bytes));
                                }
                                Err(e) => callback(Err(NetworkError::recieve_error(
                                    &e.to_string(),
                                )
                                .into())),
                            }
                        }
                    }
                }
            }
        }
    });
    // Handle incoming responses
    tokio::spawn({
        let new_sock = socket.clone();
        async move {
            loop {
                if !get_connection_status().is_connected() {
                    continue;
                }

                let (incoming_packet, addr) =
                    recieve_p2p_packet::<P2pRequest>(&new_sock).await.unwrap();

                let packet = match incoming_packet.packet {
                    P2pRequestPacket::Ping => P2pResponsePacket::Pong,
                    P2pRequestPacket::Connect {
                        join_code: _,
                        username: _,
                    } => P2pResponsePacket::error(P2pError::WrongDirection),
                    _ => P2pResponsePacket::Resync { board: vec![] },
                };

                let response = P2pResponse::new(incoming_packet.session_id, packet);
                send_p2p_packet(&new_sock, response, addr).await.unwrap();
            }
        }
    });
}
