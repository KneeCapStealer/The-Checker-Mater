use std::net::SocketAddr;

use crate::net::net_utils::{FromPacket, NetworkError, ToPacket};

pub async fn send_p2p_packet<T: ToPacket>(
    socket: tokio::net::UdpSocket,
    packet: T,
    to: SocketAddr,
) -> anyhow::Result<usize> {
    match socket.send_to(packet.to_packet().as_slice(), to).await {
        Ok(bytes) => Ok(bytes),
        Err(e) => Err(NetworkError::send_error(&e.to_string()).into()),
    }
}

pub async fn recieve_p2p_packet<T: FromPacket>(
    socket: tokio::net::UdpSocket,
) -> anyhow::Result<(T, SocketAddr)> {
    let mut buffer = vec![0; 1024];
    match socket.recv_from(&mut buffer).await {
        Ok((len, addr)) => {
            buffer.resize(len, 0);
            let response = T::from_packet(buffer.to_vec())?;
            Ok((response, addr))
        }
        Err(e) => {
            return Err(NetworkError::recieve_error(&e.to_string()).into());
        }
    }
}
