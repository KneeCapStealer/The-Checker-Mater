use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use anyhow::anyhow;
use local_ip_address::local_ip;
use thiserror::Error;

/// Turn the data into bytes ready to be sent over the network. The packet is in BE (Big Endian)
/// order.
pub trait ToPacket {
    fn to_packet(&self) -> Vec<u8>;
}
/// Turn BE (Big  Endian) bytes into data.
pub trait FromPacket {
    fn from_packet(packet: Vec<u8>) -> anyhow::Result<Self>
    where
        Self: Sized;
}

pub trait ToByte {
    fn to_u8(&self) -> u8;
}

#[derive(Error, Debug)]
pub enum PacketError {
    #[error("Invalid packet length. Expected {expected} bytes, got {got} bytes")]
    InvalidLength { expected: usize, got: usize },
    #[error("Invalid packet type.")]
    InavlidType,
    #[error("Got empty packet")]
    Empty,
    #[error("Data error. Reason: {reason:?}")]
    DataError { reason: String },
}
impl PacketError {
    pub fn invalid_length(expected: usize, got: usize) -> Self {
        Self::InvalidLength { expected, got }
    }
    pub fn data_error(reason: &str) -> Self {
        Self::DataError {
            reason: reason.to_string(),
        }
    }
}

#[derive(Error, Debug)]
pub enum NetworkError {
    #[error("Couldn't find an available port in range 6000..=7000")]
    PortBindingError,
    #[error("Failed to get local IPv4 address")]
    GetIpV4Error,
    #[error("Error occured while sending data: {details:?}")]
    SendError { details: String },
    #[error("Error occured while recieving data: {details:?}")]
    RecieveError { details: String },
    #[error("Response Type Error: Got wrong data type in return")]
    ResponseTypeError,
}
impl NetworkError {
    pub fn send_error(details: &str) -> Self {
        Self::SendError {
            details: details.to_string(),
        }
    }
    pub fn recieve_error(details: &str) -> Self {
        Self::RecieveError {
            details: details.to_string(),
        }
    }
}

pub async fn get_available_port() -> anyhow::Result<u16> {
    for port_id in 6000..=7000 {
        if (tokio::net::UdpSocket::bind(("0.0.0.0", port_id)).await).is_ok() {
            return Ok(port_id);
        }
    }
    Err(NetworkError::PortBindingError.into())
}

pub fn get_local_ip() -> anyhow::Result<Ipv4Addr> {
    local_ip_address::list_afinet_netifas()
        .unwrap()
        .iter()
        .for_each(|x| println!("{:#?}", x.1));
    if let Ok(IpAddr::V4(ip)) = local_ip() {
        Ok(ip)
    } else {
        Err(NetworkError::GetIpV4Error.into())
    }
}

pub fn hex_encode_ip(addr: SocketAddr) -> anyhow::Result<String> {
    if let IpAddr::V4(ip) = addr.ip() {
        let ip_u32: u32 = ip.into();

        let mut bytes = vec![];
        bytes.append(&mut ip_u32.to_be_bytes().to_vec());
        bytes.append(&mut addr.port().to_be_bytes().to_vec());
        Ok(hex::encode(bytes))
    } else {
        Err(NetworkError::GetIpV4Error.into())
    }
}

pub fn hex_decode_ip(data: &str) -> anyhow::Result<SocketAddr> {
    let bytes = match hex::decode(data) {
        Ok(bytes) => bytes,
        Err(_) => return Err(anyhow!("Couldn't decode hex data")),
    };

    if bytes.len() != 6 {
        return Err(anyhow!("Wrong data length"));
    }

    let ip = u32::from_be_bytes(bytes[..4].try_into().unwrap());
    let port = u16::from_be_bytes(bytes[4..].try_into().unwrap());

    Ok(SocketAddr::new(IpAddr::V4(ip.into()), port))
}
