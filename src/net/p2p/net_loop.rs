use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use crate::{
    game::GameAction,
    net::{
        p2p::{
            communicate::{recieve_p2p_packet, send_p2p_packet},
            queue::{self, get_incoming_gameaction_len, push_incoming_gameaction},
            P2pError, P2pPacket, P2pRequest, P2pRequestPacket, P2pResponse, P2pResponsePacket,
            PieceColor,
        },
        status::{
            get_connection_status, get_join_code, get_other_addr, get_session_id,
            remove_other_addr, remove_other_username, set_connection_ping, set_connection_status,
            set_other_addr, set_reconnect_tries, set_session_id, ConnectionStatus,
            CONNECT_SESSION_ID,
        },
    },
};

use super::queue::{new_transaction_id, push_outgoing_queue, wait_for_response};

pub const REQUEST_TIMEOUT_MS: u128 = 500;
const DISCONNECT_TIME_MS: u128 = 5_000;
const RECONNECT_TRIES: u32 = 10;

/// The async network loop for the host.
/// The loop goes though the following points:
///     - Check for incoming messages and respond accordingly.
///     - If connected with the client:
///         - Send the next item in the Outgoing queue to the host.
pub fn host_network_loop(socket: tokio::net::UdpSocket) {
    let socket = Arc::new(socket);
    // Handle outgoing queue
    tokio::spawn({
        println!("Starting Host Handle outgoing queue");
        let new_sock = socket.clone();
        async move {
            loop {
                let client_addr = match get_other_addr().await {
                    Some(addr) => addr,
                    None => continue,
                };
                if let Some((data, id)) = queue::pop_outgoing_queue().await {
                    println!("Sending Packet with ID {}... ({:?})", id, data);
                    send_p2p_packet(&new_sock, data, client_addr).await.unwrap();
                }
            }
        }
    });
    // Handle incoming responses
    tokio::spawn({
        println!("Starting Host handle incoming responses");
        let new_sock = socket.clone();
        async move {
            let mut time_since_ping = Instant::now();
            loop {
                if time_since_ping.elapsed().as_millis() >= DISCONNECT_TIME_MS
                    && get_other_addr().await.is_some()
                {
                    println!(
                        "Client at {:?} disconnected!",
                        get_other_addr().await.unwrap()
                    );
                    remove_other_addr().await;
                    remove_other_username().await;
                    set_session_id(CONNECT_SESSION_ID).await;
                }
                // Get incoming
                let timeout_result = tokio::time::timeout(
                    Duration::from_millis(REQUEST_TIMEOUT_MS as u64),
                    recieve_p2p_packet(&new_sock),
                )
                .await;

                let (incoming_packet, addr) = match timeout_result {
                    Ok(packet_result) => match packet_result {
                        Ok(packet) => packet,
                        Err(_) => continue,
                    },
                    Err(_) => continue,
                };

                if let P2pPacket::Request(req) = incoming_packet {
                    let packet = match req.packet {
                        P2pRequestPacket::Ping => P2pResponsePacket::Pong,
                        P2pRequestPacket::Connect {
                            join_code,
                            username,
                        } => {
                            if get_other_addr().await.is_some() {
                                println!(
                                    "Failed join attempt from {:?} - Game session full.",
                                    addr
                                );
                                P2pResponsePacket::error(P2pError::FullGameSession)
                            } else if join_code != get_join_code().await.unwrap() {
                                println!("Failed join attempt from {:?} - Wrong join code.", addr);
                                P2pResponsePacket::error(P2pError::InvalidJoinCode)
                            } else if req.session_id != CONNECT_SESSION_ID {
                                println!(
                                    "Failed join attempt from {:?} - Wrong session code.",
                                    addr
                                );
                                P2pResponsePacket::error(P2pError::InvalidSessionId)
                            } else {
                                println!("{} at {:?} Joined the game!", username, addr);

                                set_session_id(rand::random::<u16>()).await;
                                set_connection_status(ConnectionStatus::connected()).await;
                                set_other_addr(addr).await;

                                P2pResponsePacket::Connect {
                                    client_color: PieceColor::White,
                                    host_username: "Atle".to_owned(),
                                }
                            }
                        }
                        P2pRequestPacket::Resync => P2pResponsePacket::resync(vec![]),
                        P2pRequestPacket::GameAction { action } => {
                            match action {
                                GameAction::Surrender => {
                                    // TODO: Verify Surrender
                                    push_incoming_gameaction(action).await;
                                    P2pResponsePacket::Acknowledge
                                }
                                GameAction::Stalemate => {
                                    // TODO: Verify Stalemate
                                    push_incoming_gameaction(action).await;
                                    P2pResponsePacket::Acknowledge
                                }
                                GameAction::MovePiece(_) => {
                                    // TODO: Verify move
                                    push_incoming_gameaction(action).await;
                                    P2pResponsePacket::Acknowledge
                                }
                            }
                        }
                    };
                    let session_id = get_session_id().await;
                    let response = P2pResponse::new(session_id, req.transaction_id, packet);
                    queue::push_outgoing_queue(P2pPacket::Response(response), None).await;
                    time_since_ping = Instant::now();
                } else if let P2pPacket::Response(resp) = incoming_packet {
                    if !queue::check_transaction_id(resp.transaction_id).await {
                        continue;
                    }
                    queue::set_response(resp.transaction_id, Some(P2pPacket::Response(resp))).await;
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
        println!("Starting Client Ping Host");
        let mut interval = tokio::time::interval(Duration::from_millis((1000 / pings) as u64));
        async move {
            loop {
                interval.tick().await;

                let connection_status = get_connection_status().await;
                if !connection_status.is_connected() && !connection_status.is_reconnecting() {
                    continue;
                }
                if get_other_addr().await.is_none() {
                    continue;
                }

                let time = Instant::now();

                let session_id = get_session_id().await;

                let ping_id = new_transaction_id().await;
                let ping = P2pRequest::new(session_id, ping_id, P2pRequestPacket::Ping);

                push_outgoing_queue(P2pPacket::Request(ping), None).await;

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
                            println!("ping: {} ns", elapsed_ns);
                            if !get_connection_status().await.is_connected() {
                                set_connection_status(ConnectionStatus::connected()).await;
                            }
                            set_connection_ping(elapsed_ns).await;
                        }
                    }
                    Err(e) => {
                        if let ConnectionStatus::Reconnecting { tries } =
                            get_connection_status().await
                        {
                            println!("Trying to reconnect... ({} / {})", tries, RECONNECT_TRIES);
                            if tries >= RECONNECT_TRIES as u8 {
                                set_connection_status(ConnectionStatus::Disconnected).await;
                                remove_other_addr().await;
                                remove_other_username().await;
                                println!("Disconnected from host");
                            } else {
                                set_reconnect_tries(tries + 1).await;
                            }
                        } else {
                            println!("Ping request time out: {}", e.to_string());
                            set_connection_status(ConnectionStatus::reconnecting()).await;
                        }
                    }
                }
            }
        }
    });
    // Handle outgoing queue
    tokio::spawn({
        println!("Starting Client Handle outgoing queue");
        let new_sock = socket.clone();
        async move {
            loop {
                let client_addr = match get_other_addr().await {
                    Some(addr) => addr,
                    None => continue,
                };
                if let Some((data, id)) = queue::pop_outgoing_queue().await {
                    println!("Sending Packet with ID {}... ({:?})", id, data);
                    send_p2p_packet(&new_sock, data, client_addr).await.unwrap();
                }
            }
        }
    });
    // Handle incoming responses
    tokio::spawn({
        println!("Starting Client Handle incoming responses");
        let new_sock = socket.clone();
        async move {
            loop {
                let timeout_result = tokio::time::timeout(
                    Duration::from_millis(REQUEST_TIMEOUT_MS as u64),
                    recieve_p2p_packet(&new_sock),
                )
                .await;

                let (incoming_packet, addr) = match timeout_result {
                    Ok(Ok(packet)) => packet,
                    _ => continue,
                };
                if addr != get_other_addr().await.unwrap() {
                    continue;
                }
                if let P2pPacket::Request(req) = incoming_packet {
                    let packet = match req.packet {
                        P2pRequestPacket::Ping => P2pResponsePacket::Pong,
                        P2pRequestPacket::GameAction { action } => {
                            match action {
                                GameAction::Surrender => {
                                    // TODO: Verify Surrender
                                    push_incoming_gameaction(action).await;
                                    println!(
                                        "Incoming action len: {}",
                                        get_incoming_gameaction_len().await
                                    );
                                    P2pResponsePacket::Acknowledge
                                }
                                GameAction::Stalemate => {
                                    // TODO: Verify stalemate
                                    push_incoming_gameaction(action).await;
                                    println!(
                                        "Incoming action len: {}",
                                        get_incoming_gameaction_len().await
                                    );
                                    P2pResponsePacket::Acknowledge
                                }
                                GameAction::MovePiece(_) => {
                                    // TODO: Verify move
                                    push_incoming_gameaction(action).await;
                                    println!(
                                        "Incoming action len: {}",
                                        get_incoming_gameaction_len().await
                                    );
                                    P2pResponsePacket::Acknowledge
                                }
                            }
                        }
                        _ => P2pResponsePacket::error(P2pError::WrongDirection),
                    };
                    let response = P2pResponse::new(req.session_id, req.transaction_id, packet);
                    send_p2p_packet(&new_sock, response, addr).await.unwrap();
                    println!("Sent package");
                } else if let P2pPacket::Response(resp) = incoming_packet {
                    // if !queue::check_transaction_id(resp.transaction_id).await {
                    //     continue;
                    // }
                    queue::set_response(resp.transaction_id, Some(P2pPacket::Response(resp))).await;
                }
            }
        }
    });
}
