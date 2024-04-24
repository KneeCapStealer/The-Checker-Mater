use anyhow::anyhow;

use super::net_utils::{FromPacket, PacketError, ToByte, ToPacket};

/// A request for P2P (Peer to Peer) connection. This moves mostly from client to host, but the
/// host will send requests to the client, when it makes an update to the board.
pub struct P2pRequest {
    session_id: u16,
    packet: P2pRequestPacket,
}
impl ToPacket for P2pRequest {
    fn to_packet(&self) -> Vec<u8> {
        let mut bytes = vec![];
        bytes.append(&mut self.session_id.to_be_bytes().to_vec());
        bytes.append(&mut self.packet.to_packet());

        bytes
    }
}

/// The different types of packets you can send as a request to the other peer.
#[derive(Clone, Debug)]
pub enum P2pRequestPacket {
    /// Ping the other peer, to uphold the connection. This must be done often.
    Ping,
    /// Request to connect to the host. `join_code` is the HEX encoded IP and port of the host,
    /// which is the same as the join code if working over LAN. 'username' is the username the
    /// client wishes to use.
    Connect {
        join_code: [u8; 12],
        username: String,
    },
    /// Ask the host for a copy of the correct board, so the client can resync theirs.
    Resync,
    /// Tell the other peer that you have moved your piece. This will automatically be a success
    /// if done by the host, but a peer must wait for the hosts reply, to check if the move was
    /// valid. If not valid, request a resync.
    MovePiece { from: u8, to: u8 },
    /// Tell the other peer that you will end your turn. This will automatically be a success
    /// if done by the host, but a peer must wait for the hosts reply, to check if it is able to
    /// end their turn. If not valid, request a resync.
    EndTurn,
}
impl ToPacket for P2pRequestPacket {
    fn to_packet(&self) -> Vec<u8> {
        let mut bytes = vec![];
        match self {
            Self::Ping => {
                bytes.append(&mut self.to_u8().to_be_bytes().to_vec()); // Packet type code
            }
            Self::Connect {
                join_code,
                username,
            } => {
                bytes.append(&mut self.to_u8().to_be_bytes().to_vec()); // Packet type code

                bytes.append(&mut join_code.to_vec());
                bytes.append(&mut username.as_bytes().to_vec());
            }
            Self::Resync => {
                bytes.append(&mut self.to_u8().to_be_bytes().to_vec()); // Packet type code
            }
            Self::MovePiece { from, to } => {
                bytes.append(&mut self.to_u8().to_be_bytes().to_vec()); // Packet type code

                bytes.append(&mut from.to_be_bytes().to_vec());
                bytes.append(&mut to.to_be_bytes().to_vec());
            }
            Self::EndTurn => {
                bytes.append(&mut self.to_u8().to_be_bytes().to_vec());
            }
        }
        bytes
    }
}
impl FromPacket for P2pRequestPacket {
    fn from_packet(packet: Vec<u8>) -> anyhow::Result<Self> {
        if packet.len() == 0 {
            return Err(PacketError::Empty.into());
        }
        match packet[0] {
            // Connect
            1 => Ok(Self::Ping),
            // Ping
            2 => {
                if packet.len() < 14 {
                    return Err(PacketError::invalid_length(14, packet.len()).into());
                }
                let join_code: [u8; 12] = packet[1..13].try_into().unwrap();
                let username = match String::from_utf8(packet[13..].to_vec()) {
                    Ok(string) => string,
                    Err(_) => {
                        return Err(PacketError::data_error(
                            "Invalid UFT8 encoded values for username",
                        )
                        .into())
                    }
                };

                Ok(Self::Connect {
                    join_code,
                    username,
                })
            }
            3 => Ok(Self::Resync),
            4 => {
                if packet.len() != 3 {
                    return Err(PacketError::invalid_length(3, packet.len()).into());
                }

                let from = packet[1];
                let to = packet[2];

                Ok(Self::MovePiece { from, to })
            }
            5 => Ok(Self::EndTurn),
            _ => Err(
                PacketError::data_error(&format!("Not valid packet type: {}", packet[0])).into(),
            ),
        }
    }
}
impl ToByte for P2pRequestPacket {
    fn to_u8(&self) -> u8 {
        match self {
            Self::Ping => 1,
            Self::Connect {
                join_code: _,
                username: _,
            } => 2,
            Self::Resync => 3,
            Self::MovePiece { from: _, to: _ } => 4,
            Self::EndTurn => 5,
        }
    }
}

/// A response to the `P2pResonse` struct.
#[derive(Clone, Debug)]
pub struct P2pResponse {
    session_id: u16,
    packet: P2pResponsePacket,
}
impl ToPacket for P2pResponse {
    fn to_packet(&self) -> Vec<u8> {
        let mut bytes = vec![];
        bytes.append(&mut self.session_id.to_be_bytes().to_vec());
        bytes.append(&mut self.packet.to_packet());
        bytes
    }
}
impl FromPacket for P2pResponse {
    fn from_packet(packet: Vec<u8>) -> anyhow::Result<Self> {
        if packet.len() < 3 {
            return Err(PacketError::invalid_length(3, packet.len()).into());
        }

        let session_id = u16::from_be_bytes(packet[1..3].try_into().unwrap());
        let packet = P2pResponsePacket::from_packet(packet[3..].to_vec())?;

        Ok(Self { session_id, packet })
    }
}

/// The different types of packets you can send as a response to the other peer.
#[derive(Clone, Debug)]
pub enum P2pResponsePacket {
    Error {
        kind: P2pError,
    },
    Pong,
    Connect {
        client_color: PieceColor,
        host_username: String,
    },
    Resync {
        board: Vec<Tile>,
    },
}
impl ToPacket for P2pResponsePacket {
    fn to_packet(&self) -> Vec<u8> {
        let mut bytes = vec![];

        match self {
            Self::Error { kind } => {
                bytes.append(&mut self.to_u8().to_be_bytes().to_vec()); // Packet type code
                bytes.append(&mut kind.to_u8().to_be_bytes().to_vec());
            }
            Self::Pong => {
                bytes.append(&mut self.to_u8().to_be_bytes().to_vec()); // Packet type code
            }
            Self::Connect {
                client_color,
                host_username,
            } => {
                bytes.append(&mut self.to_u8().to_be_bytes().to_vec()); // Packet type code

                bytes.append(&mut client_color.to_u8().to_be_bytes().to_vec());
                bytes.append(&mut host_username.as_bytes().to_vec());
            }
            Self::Resync { board } => {
                bytes.append(&mut self.to_u8().to_be_bytes().to_vec()); // Packet type code

                for tile in board {
                    if let Some(piece) = tile {
                        bytes.append(&mut vec![piece.to_u8()]);
                    } else {
                        bytes.append(&mut 0u8.to_be_bytes().to_vec());
                    }
                }
            }
        }

        bytes
    }
}
impl FromPacket for P2pResponsePacket {
    fn from_packet(packet: Vec<u8>) -> anyhow::Result<Self> {
        if packet.len() == 0 {
            return Err(PacketError::Empty.into());
        }
        match packet[0] {
            // Error
            0 => {
                if packet.len() != 2 {
                    return Err(PacketError::invalid_length(2, packet.len()).into());
                }
                let kind = match P2pError::try_from(packet[1]) {
                    Ok(kind) => kind,
                    Err(e) => {
                        return Err(PacketError::data_error(&e.to_string()).into());
                    }
                };

                Ok(Self::Error { kind })
            }
            // Pong
            1 => Ok(Self::Pong),
            // Connect
            2 => {
                if packet.len() < 3 {
                    return Err(PacketError::invalid_length(2, packet.len()).into());
                }

                let client_color = match PieceColor::try_from(packet[1]) {
                    Ok(color) => color,
                    Err(e) => return Err(PacketError::data_error(&e.to_string()).into()),
                };

                let host_username = match String::from_utf8(packet[2..].to_vec()) {
                    Ok(string) => string,
                    Err(_) => {
                        return Err(PacketError::data_error(
                            "Invalid UFT8 encoded values for username",
                        )
                        .into())
                    }
                };

                Ok(Self::Connect {
                    client_color,
                    host_username,
                })
            }
            // Resync
            3 => {
                if packet.len() < 65 {
                    return Err(PacketError::invalid_length(65, packet.len()).into());
                }
                let mut board = vec![];
                for byte in packet[1..].to_vec() {
                    match Piece::try_from(byte) {
                        Ok(piece) => board.push(Some(piece)),
                        Err(e) => return Err(PacketError::data_error(&e.to_string()).into()),
                    }
                }

                Ok(Self::Resync { board })
            }
            _ => Err(
                PacketError::data_error(&format!("Not valid packet type: {}", packet[0])).into(),
            ),
        }
    }
}
impl ToByte for P2pResponsePacket {
    fn to_u8(&self) -> u8 {
        match self {
            Self::Error { kind: _ } => 0,
            Self::Pong => 1,
            Self::Connect {
                client_color: _,
                host_username: _,
            } => 2,
            Self::Resync { board: _ } => 3,
        }
    }
}

#[derive(Clone, Debug)]
pub enum P2pError {
    InvalidBoard,
}
impl ToByte for P2pError {
    fn to_u8(&self) -> u8 {
        match self {
            Self::InvalidBoard => 0,
        }
    }
}
impl TryFrom<u8> for P2pError {
    type Error = anyhow::Error;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::InvalidBoard),
            _ => Err(anyhow!("Can only take 0 for P2p Error, got {}", value)),
        }
    }
}

/// THIS IS A TEMP ENUM
#[derive(Clone, Debug)]
pub enum PieceColor {
    White,
    Black,
}
impl ToByte for PieceColor {
    fn to_u8(&self) -> u8 {
        match self {
            Self::White => 1,
            Self::Black => 2,
        }
    }
}
impl TryFrom<u8> for PieceColor {
    type Error = anyhow::Error;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::White),
            2 => Ok(Self::Black),
            _ => Err(anyhow!(
                "Can only take 1 or 2 for Piece Color, got {}",
                value
            )),
        }
    }
}
/// THIS IS A TEMP STRUCT
#[derive(Clone, Debug)]
pub struct Piece {
    color: PieceColor,
    is_king: bool,
}
impl ToByte for Piece {
    fn to_u8(&self) -> u8 {
        unimplemented!()
    }
}
impl TryFrom<u8> for Piece {
    type Error = anyhow::Error;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        unimplemented!()
    }
}
/// THIS IS A TEMP TYPE
type Tile = Option<Piece>;
