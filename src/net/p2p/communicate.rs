use std::{net::SocketAddr, sync::Arc};

use crate::net::net_utils::{FromPacket, NetworkError, ToPacket};

use super::P2pPacket;

/// Send a packet to the other machine over a P2P UDP protocol.
/// # Example:
/// ```
/// let socket = tokio::net::UdpSocket::bind(("0.0.0.0", 1000)).await?;
///
/// let to_address = SocketAddr::new(IpAddr::from_str("0.0.0.0")?, 2000));
///
/// let request = P2pRequest::new(0, P2pRequestPacket::Ping);
///
/// send_p2p_packet::<P2pRequest>(socket, request, to_address)?;
/// ```
pub async fn send_p2p_packet<T: ToPacket>(
    socket: &Arc<tokio::net::UdpSocket>,
    packet: T,
    to: SocketAddr,
) -> anyhow::Result<usize> {
    match socket.send_to(packet.to_packet().as_slice(), to).await {
        Ok(bytes) => Ok(bytes),
        Err(e) => Err(NetworkError::send_error(&e.to_string()).into()),
    }
}

/// Recieve a packet from the other machine over a P2P UDP protocol.
/// Returns a tuple of the data struct, and the `SocketAddr` that you got the data from.
/// # Example:
/// ```
/// let socket = tokio::net::UdpSocket::bind(("0.0.0.0", 8080)).await?;
///
/// let (response, addr) = recieve_p2p_packet::<P2pResponse>(socket)?;
/// ```
pub async fn recieve_p2p_packet(
    socket: &Arc<tokio::net::UdpSocket>,
) -> anyhow::Result<(P2pPacket, SocketAddr)> {
    let mut buffer = vec![0; 1024];
    match socket.recv_from(&mut buffer).await {
        Ok((len, addr)) => {
            buffer.resize(len, 0);
            let response = P2pPacket::from_packet(buffer.to_vec())?;
            Ok((response, addr))
        }
        Err(e) => {
            return Err(NetworkError::recieve_error(&e.to_string()).into());
        }
    }
}
