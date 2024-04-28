use std::net::SocketAddr;

use as_any::AsAny;

use super::{
    net_utils::{FromPacket, NetworkError, ToPacket},
    p2p::{
        communicate::{recieve_p2p_packet, send_p2p_packet},
        P2pRequest, P2pRequestPacket, P2pResponse, P2pResponsePacket, PieceColor,
    },
};

pub trait NetworkSendable: ToPacket + FromPacket + AsAny + Send + Sync {}

/// Queue for outgoing packets. Follows First in First out principle.
/// Each item in the queue is a tuple of two items: The outgoing packet, and a closure that runs when
/// the outgoing packet has gotten a response.
/// If the packet isn't meant to get a response, set the closure to `None`.
static mut OUTGOING_QUEUE: Vec<(
    Box<dyn NetworkSendable>,
    Option<Box<dyn Fn(anyhow::Result<Box<dyn NetworkSendable>>)>>,
)> = vec![];

pub fn push_outgoing_queue<S: NetworkSendable + 'static, R: NetworkSendable + Sized + 'static>(
    data: S,
    closure: Option<Box<dyn Fn(anyhow::Result<Box<&R>>)>>,
) {
    unsafe {
        OUTGOING_QUEUE.push((
            Box::new(data),
            match closure {
                Some(f) => Some(Box::new(
                    move |recieved: anyhow::Result<Box<(dyn NetworkSendable + 'static)>>| {
                        if let Some(response) = recieved.unwrap().as_any().downcast_ref::<R>() {
                            f(Ok(Box::new(response)));
                        } else {
                            f(Err(NetworkError::ResponseTypeError.into()))
                        }
                    },
                )),
                None => None,
            },
        ))
    }
}

/// Pops and returns the next item in the outgoing network queue.
pub fn pop_outgoing_queue<S: NetworkSendable, R: NetworkSendable>() -> Option<(
    Box<dyn NetworkSendable>,
    Option<Box<dyn Fn(anyhow::Result<Box<dyn NetworkSendable>>)>>,
)> {
    unsafe { OUTGOING_QUEUE.pop() }
}

static CLIENT_ADDR: Option<SocketAddr> = None;

fn get_client_addr() -> Option<SocketAddr> {
    unsafe { CLIENT_ADDR }
}

fn set_client_addr(addr: SocketAddr) {
    unsafe { CLIENT_ADDR = Some(addr) }
}

pub async fn host_network_loop(socket: &tokio::net::UdpSocket) {
    loop {
        // Handle incoming responses
        {
            let (incoming_packet, addr) = recieve_p2p_packet::<P2pRequest>(&socket).await.unwrap();

            let packet = match incoming_packet.packet {
                P2pRequestPacket::Ping => P2pResponsePacket::Pong,
                P2pRequestPacket::Connect {
                    join_code: _,
                    username: _,
                } => P2pResponsePacket::Connect {
                    client_color: PieceColor::White,
                    host_username: "Atle".to_string(),
                },
                _ => P2pResponsePacket::Resync { board: vec![] },
            };

            let response = P2pResponse::new(incoming_packet.session_id, packet);
            send_p2p_packet(&socket, response, addr).await.unwrap();
        }

        let client_addr = match get_client_addr() {
            Some(addr) => addr,
            None => continue,
        };

        if let Some((data, callback)) = pop_outgoing_queue() {
            send_p2p_packet(&socket, data.to_packet(), client_addr)
                .await
                .unwrap();

            if let Some(callback) = callback {
                let (response, new_addr) = recieve_p2p_packet(&socket).await.unwrap();

                callback()
            }
        }
    }
}

pub async fn client_network_loop(pings: usize) {}
