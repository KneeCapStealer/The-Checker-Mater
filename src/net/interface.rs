use std::{
    net::{IpAddr, SocketAddr},
    sync::Arc,
    time::Duration,
};

use anyhow::anyhow;
use futures::executor;
use tokio::sync::Mutex;

use crate::net::{
    net_utils::hex_decode_ip,
    p2p::{
        net_loop::client_network_loop,
        queue::{new_transaction_id, push_outgoing_queue},
        P2pPacket, P2pRequest, P2pRequestPacket,
    },
    status::{
        get_session_id, set_connection_status, set_other_addr, set_session_id, ConnectionStatus,
        CONNECT_SESSION_ID,
    },
};

use super::{
    net_utils::{get_available_port, get_local_ip, hex_encode_ip},
    p2p::{
        net_loop::host_network_loop,
        queue::{check_for_response, pop_incoming_gameaction},
        P2pResponse, P2pResponsePacket, PieceColor,
    },
    status::set_join_code,
};

/// An enum which holds the possible actions a user can make in the game.
#[derive(Clone, Copy, Debug)]
pub enum GameAction {
    /// Move a piece, by its current position, and its target position.
    /// It is not guarenteed that this move is valid yet, so it should be validated before use.
    MovePiece { to: usize, from: usize },
    /// Indicates that the player wants to end the game by surrender
    Surrender,
}
impl GameAction {
    /// Creates a `GameAction::MovePiece`.
    /// * `from` - The start location of the piece.
    /// * `to` - The end location of the piece.
    pub fn move_piece(from: usize, to: usize) -> Self {
        Self::MovePiece { to, from }
    }
}

/// Start the host network peer on a LAN connection.
/// Returns the join code for the client
pub async fn start_lan_host() -> String {
    let port = get_available_port().await.unwrap();
    let socket = tokio::net::UdpSocket::bind(("0.0.0.0", port))
        .await
        .unwrap();

    let local_ip = get_local_ip().unwrap();

    let encoded_ip = hex_encode_ip(SocketAddr::new(IpAddr::V4(local_ip), port)).unwrap();
    set_join_code(&encoded_ip).await;

    host_network_loop(socket);

    encoded_ip
}

/// Start the client network peer on a LAN connection.
pub async fn start_lan_client() {
    let port = get_available_port().await.unwrap();
    let socket = tokio::net::UdpSocket::bind(("0.0.0.0", port))
        .await
        .unwrap();

    // Start client network loop, with 10 pings pr. second
    client_network_loop(socket, 10);
}

/// Sends a join request to the host.
/// This function should only be called by the client, and only after the client network loop has
/// started, via. `start_lan_client()`.
///
/// ## Params
/// * `join_code` - The join code sent by the host.
pub async fn send_join_request(join_code: &str, username: &str) -> u16 {
    set_join_code(join_code).await;
    let host_addr = hex_decode_ip(join_code).unwrap();
    set_other_addr(Some(host_addr)).await;

    let join_request = P2pRequest::new(
        CONNECT_SESSION_ID,
        new_transaction_id().await,
        P2pRequestPacket::Connect {
            join_code: join_code.to_owned(),
            username: username.to_owned(),
        },
    );
    println!("Asking to join Host at {:?}", host_addr);

    set_connection_status(ConnectionStatus::PendingConnection).await;

    let id = push_outgoing_queue(P2pPacket::Request(join_request.clone()), None).await;

    // if let Ok(resp) = tokio::time::timeout(Duration::from_millis(1000), wait_for_response(id)).await
    // {
    //     println!("GOT ANSWER!!!!");
    //     dbg!(&resp);
    //     if let P2pPacket::Response(resp) = resp {
    //         println!("Got response");
    //
    //         println!("Setting connection to Connected!");
    //         set_connection_status(ConnectionStatus::connected()).await;
    //         println!("Connection sat to connected!!");
    //
    //         set_session_id(resp.session_id).await;
    //
    //         dbg!(&resp);
    //     }
    // }
    id
}

/// Check if the connection request sent with `send_join_request()` has gotten an response.
/// If a packet has been recieved, and if that packet is a correct response, the function will
/// return the clients assigned piece color, as well as the hosts username.
///
/// ## Params
/// * `transaction_id` - The id of the join request
pub async fn check_for_connection_resp(
    transaction_id: u16,
) -> Option<anyhow::Result<(PieceColor, String)>> {
    match check_for_response(transaction_id).await {
        Some(resp) => match resp {
            P2pPacket::Response(resp) => match resp.packet {
                P2pResponsePacket::Connect {
                    client_color,
                    host_username,
                } => {
                    set_connection_status(ConnectionStatus::connected()).await;
                    set_session_id(resp.session_id).await;
                    return Some(Ok((client_color, host_username)));
                }
                _ => Some(Err(anyhow!("Got wrong response Packet"))),
            },
            _ => Some(Err(anyhow!("Got request packet instead of response"))),
        },
        None => None,
    }
}

pub async fn connect_to_host_loop(join_code: &str, username: &str) -> (PieceColor, String) {
    let mut connection_tick = tokio::time::interval(Duration::from_millis(500));
    loop {
        let join_id = send_join_request(join_code, username).await;

        for _ in 0..25 {
            connection_tick.tick().await;
            if let Some(resp) = check_for_connection_resp(join_id).await {
                return resp;
            }
        }
    }
}

/// Gets the other users username
pub fn get_other_username() -> String {
    todo!()
}

/// Get the next game action from the other user.
pub async fn get_next_game_action() -> Option<GameAction> {
    pop_incoming_gameaction().await
}

/// Send a game action to the other user.
/// The function is not blocking the thread until it gets a response.
///
/// ## Params:
/// * `action` - The game action you want to send, is of type `GameAction`
/// * `on_response` - The closure that will be called when the `GameAction` request gets a
/// response.
///
/// ## Examples:
/// ```
/// let action = GameAction::Surrender;
///
/// let callback = |res: anyhow::Result<()>| {
///     match res {
///         Ok(_) => println!("Hell yea!!"),
///         Err(_) => println!("Hell no!!"),
///     };
/// }
///
/// send_game_action(action, callback);
/// ```
pub fn send_game_action<F>(action: GameAction, mut on_response: F)
where
    F: FnMut(anyhow::Result<()>) + Send + Sync + 'static,
{
    let closure = Arc::new(Mutex::new(move |resp: P2pResponse| {
        if let P2pResponsePacket::Error { kind: _ } = resp.packet {
            on_response(Err(anyhow::anyhow!("Recieved error")));
        } else {
            on_response(Ok(()));
        }
    }));

    let request = P2pRequest {
        session_id: executor::block_on(get_session_id()),
        transaction_id: executor::block_on(new_transaction_id()),
        packet: P2pRequestPacket::game_action(action),
    };
    executor::block_on(push_outgoing_queue(
        P2pPacket::Request(request),
        Some(closure),
    ));
}
