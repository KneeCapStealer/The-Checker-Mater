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
