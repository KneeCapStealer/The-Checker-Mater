use std::net::{IpAddr, SocketAddr};

use crate::net::{
    net_utils::hex_decode_ip,
    p2p::{
        net_loop::client_network_loop,
        queue::{new_transaction_id, push_outgoing_queue, wait_for_response},
        P2pPacket, P2pRequest, P2pRequestPacket,
    },
    status::{
        get_connection_status, get_session_id, set_connection_status, set_other_addr,
        set_session_id, ConnectionStatus,
    },
};

use super::{
    net_utils::{get_available_port, get_local_ip, hex_encode_ip},
    p2p::net_loop::host_network_loop,
    status::set_join_code,
};

/// An enum which holds the possible actions a user can make in the game.
pub enum GameAction {
    MovePiece { to: usize, from: usize },
    Surrender,
}

/// Start the client network peer on a LAN connection.
/// * `join_code` - The join code sent by the host.
pub async fn start_lan_client(join_code: &str) {
    let port = get_available_port().await.unwrap();
    let socket = tokio::net::UdpSocket::bind(("0.0.0.0", port))
        .await
        .unwrap();

    set_join_code(join_code);
    let host_addr = hex_decode_ip(join_code).unwrap();
    set_other_addr(Some(host_addr));

    // Start client network loop, with 10 pings pr. second
    client_network_loop(socket, 10);

    let join_request = P2pRequest::new(
        get_session_id(),
        new_transaction_id().await,
        P2pRequestPacket::Connect {
            join_code: join_code.to_owned(),
            username: "Client".to_owned(),
        },
    );
    println!("Asking to join Host at {:?}", host_addr);

    set_connection_status(ConnectionStatus::PendingConnection);

    let id = push_outgoing_queue(P2pPacket::Request(join_request)).await;

    let resp = wait_for_response(id).await;

    if let P2pPacket::Response(resp) = resp {
        println!("Got response");

        set_connection_status(ConnectionStatus::connected());
        set_session_id(resp.session_id);

        dbg!(&resp);
    }
}

/// Start the host network peer on a LAN connection.
/// Returns the join code for the client.
pub async fn start_lan_host() -> String {
    let port = get_available_port().await.unwrap();
    let socket = tokio::net::UdpSocket::bind(("0.0.0.0", port))
        .await
        .unwrap();

    let local_ip = get_local_ip().unwrap();

    let encoded_ip = hex_encode_ip(SocketAddr::new(IpAddr::V4(local_ip), port)).unwrap();
    set_join_code(&encoded_ip);

    host_network_loop(socket);

    encoded_ip
}

/// Gets the other users username
pub fn get_other_username() -> String {
    todo!()
}

/// Get the next game action from other computer.
fn get_next_game_action() -> Option<GameAction> {
    todo!()
}

/// Non blocking
fn send_game_action(action: GameAction, on_response: fn(anyhow::Result<()>)) {
    todo!()
}
