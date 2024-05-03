use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use crate::net::{
    p2p::{
        communicate::{recieve_p2p_packet, send_p2p_packet},
        queue::{self, get_outgoing_queue_len},
        P2pError, P2pPacket, P2pRequest, P2pRequestPacket, P2pResponse, P2pResponsePacket,
        PieceColor,
    },
    status::{
        get_connection_status, get_connection_status_mut, get_join_code, get_other_addr,
        get_session_id, set_connection_ping, set_connection_status, set_other_addr, set_session_id,
        ConnectionStatus, CONNECT_SESSION_ID,
    },
};

use super::queue::{new_transaction_id, push_outgoing_queue, wait_for_response};

const REQUEST_TIMEOUT_MS: u128 = 500;
const DISCONNECT_TIME_MS: u128 = 5_000;
const RECONNECT_TRIES: u32 = 10;

/// The async network loop for the host.
/// The loop goes though the following points:
///     - Check for incoming messages and respond accordingly.
///     - If connected with the client:
///         - Send the next item in the Outgoing queue to the host.
pub fn host_network_loop(socket: tokio::net::UdpSocket) {
    let socket = Arc::new(socket);
    // Handle incoming responses
    tokio::spawn({
        let new_sock = socket.clone();
        async move {
            let mut time_since_ping = Instant::now();
            loop {
                // Get incoming
                let timeout_result = tokio::time::timeout(
                    Duration::from_millis(REQUEST_TIMEOUT_MS as u64),
                    recieve_p2p_packet(&new_sock),
                )
                .await;

                if let Err(_e) = &timeout_result {
                    continue;
                }
                let (incoming_packet, addr) = timeout_result.unwrap().unwrap();
                if addr != get_other_addr().unwrap_or(addr) {
                    continue;
                }
                println!("GOT PACKET:");
                dbg!(&incoming_packet);
                match incoming_packet {
                    P2pPacket::Request(req) => {
                        let packet = match req.packet {
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
                                } else if req.session_id != CONNECT_SESSION_ID {
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
                            _ => P2pResponsePacket::error(P2pError::InvalidBoard),
                        };
                        let session_id = get_session_id();
                        let response = P2pResponse::new(session_id, req.transaction_id, packet);
                        queue::push_outgoing_queue(P2pPacket::Response(response), None).await;
                        println!("QUEUE LEN: {}", get_outgoing_queue_len());
                        time_since_ping = Instant::now();
                    }
                    P2pPacket::Response(resp) => {
                        if !queue::check_transaction_id(resp.transaction_id).await {
                            continue;
                        }
                        queue::set_response(resp.transaction_id, Some(P2pPacket::Response(resp)))
                            .await;
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
                if !get_connection_status().can_send() {
                    continue;
                }
                let client_addr = match get_other_addr() {
                    Some(addr) => addr,
                    None => continue,
                };
                if let Some((data, _, id)) = queue::pop_outgoing_queue() {
                    println!("Sending Packet with ID {}...", id);
                    send_p2p_packet(&new_sock, data, client_addr).await.unwrap();
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
pub fn client_network_loop(socket: tokio::net::UdpSocket, pings: usize) {
    let socket = Arc::new(socket);
    // Ping host
    tokio::spawn({
        let mut interval = tokio::time::interval(Duration::from_millis((1000 / pings) as u64));
        async move {
            loop {
                interval.tick().await;

                if !get_connection_status().is_connected()
                    && !get_connection_status().is_reconnecting()
                {
                    continue;
                }
                let time = Instant::now();

                let session_id = get_session_id();

                let ping_id = new_transaction_id().await;
                let ping = P2pRequest::new(session_id, ping_id, P2pRequestPacket::Ping);

                push_outgoing_queue(P2pPacket::Request(ping)).await;

                match tokio::time::timeout(
                    Duration::from_millis(REQUEST_TIMEOUT_MS as u64),
                    wait_for_response(ping_id),
                )
                .await
                {
                    Ok(data) => {
                        if let P2pPacket::Response(pong) = data {
                            if pong.packet != P2pResponsePacket::Pong {
                                println!("Got wrong packet, expected pong, got: ");
                                dbg!(&pong);
                            }
                            let elapsed_ns = time.elapsed().as_nanos();
                            print!("\rping: {} ns", elapsed_ns);
                            set_connection_status(ConnectionStatus::connected());
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
                            *status = ConnectionStatus::reconnecting();
                        }
                    }
                }
            }
        }
    });
    // Handle outgoing queue
    tokio::spawn({
        let new_sock = socket.clone();
        async move {
            loop {
                if !get_connection_status().can_send() {
                    println!("Hey :J");
                    continue;
                }
                let client_addr = match get_other_addr() {
                    Some(addr) => addr,
                    None => continue,
                };
                if let Some((data, _transaction_id)) = queue::pop_outgoing_queue() {
                    println!("Sending Packet...");
                    send_p2p_packet(&new_sock, data, client_addr).await.unwrap();
                }
            }
        }
    });
    // Handle incoming responses
    tokio::spawn({
        let new_sock = socket.clone();
        async move {
            loop {
                let timeout_result = tokio::time::timeout(
                    Duration::from_millis(REQUEST_TIMEOUT_MS as u64),
                    recieve_p2p_packet(&new_sock),
                )
                .await;

                if let Err(_e) = &timeout_result {
                    continue;
                }
                let (incoming_packet, addr) = timeout_result.unwrap().unwrap();
                if addr != get_other_addr().unwrap() {
                    continue;
                }
                match incoming_packet {
                    P2pPacket::Request(req) => {
                        let packet = match req.packet {
                            P2pRequestPacket::EndTurn => P2pResponsePacket::Pong,
                            _ => P2pResponsePacket::error(P2pError::WrongDirection),
                        };

                        let response = P2pResponse::new(req.session_id, req.transaction_id, packet);
                        send_p2p_packet(&new_sock, response, addr).await.unwrap();
                    }
                    P2pPacket::Response(resp) => {
                        if !queue::check_transaction_id(resp.transaction_id).await {
                            continue;
                        }
                        queue::set_response(resp.transaction_id, Some(P2pPacket::Response(resp)))
                            .await;
                    }
                }
            }
        }
    });
}
