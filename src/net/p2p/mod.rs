pub mod communicate;
pub mod net_loop;
pub mod queue;

use anyhow::anyhow;

use super::net_utils::{FromPacket, PacketError, ToByte, ToPacket};

use crate::game::{GameAction, PieceColor, PieceData, Move};

#[derive(Clone, Debug)]
pub enum P2pPacket {
    Request(P2pRequest),
    Response(P2pResponse),
}

impl P2pPacket {
    pub fn is_request(&self) -> bool {
        match self {
            Self::Request(_) => true,
            _ => false,
        }
    }
    pub fn is_response(&self) -> bool {
        match self {
            Self::Response(_) => true,
            _ => false,
        }
    }
}

impl ToPacket for P2pPacket {
    fn to_packet(&self) -> Vec<u8> {
        match self {
            Self::Request(req) => req.to_packet(),
            Self::Response(resp) => resp.to_packet(),
        }
    }
}

impl FromPacket for P2pPacket {
    fn from_packet(packet: Vec<u8>) -> anyhow::Result<Self> {
        match packet[0] {
            0 => match P2pRequest::from_packet(packet) {
                Ok(req) => Ok(Self::Request(req)),
                Err(e) => Err(e),
            },
            1 => match P2pResponse::from_packet(packet) {
                Ok(resp) => Ok(Self::Response(resp)),
                Err(e) => Err(e),
            },
            _ => Err(PacketError::InavlidType.into()),
        }
    }
}

/// A request for P2P (Peer to Peer) connection. This moves mostly from client to host, but the
/// host will send requests to the client, when it makes an update to the board.
#[derive(Clone, Debug)]
pub struct P2pRequest {
    /// The sessions ID set by the host. Is set to 0 if it is the first time the client is talking
    /// with the host.
    pub session_id: u16,
    /// This specific transactions ID
    pub transaction_id: u16,
    /// The main packet of the request.
    pub packet: P2pRequestPacket,
}

impl P2pRequest {
    /// Create a new `P2pRequest` from the sessions ID and the packet.
    pub fn new(session_id: u16, transaction_id: u16, packet: P2pRequestPacket) -> Self {
        Self {
            session_id,
            transaction_id,
            packet,
        }
    }
}

impl ToPacket for P2pRequest {
    fn to_packet(&self) -> Vec<u8> {
        let mut bytes = vec![];
        bytes.append(&mut 0u8.to_be_bytes().to_vec());
        bytes.append(&mut self.session_id.to_be_bytes().to_vec());
        bytes.append(&mut self.transaction_id.to_be_bytes().to_vec());
        bytes.append(&mut self.packet.to_packet());

        bytes
    }
}

impl FromPacket for P2pRequest {
    fn from_packet(packet: Vec<u8>) -> anyhow::Result<Self> {
        if packet.len() < 6 {
            return Err(PacketError::invalid_length(6, packet.len()).into());
        }
        if packet[0] != 0 {
            return Err(PacketError::InavlidType.into());
        }
        let session_id = u16::from_be_bytes(packet[1..3].try_into().unwrap());
        let transaction_id = u16::from_be_bytes(packet[3..5].try_into().unwrap());
        let packet = P2pRequestPacket::from_packet(packet[5..].to_vec())?;

        Ok(Self {
            session_id,
            transaction_id,
            packet,
        })
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
        /// The games join code. Calculated by HEX encoding the hosts IP and PORT. When on LAN, its
        /// the code given to the client by the host.
        join_code: String,
        /// The clients username. Set by the clients user.
        username: String,
    },
    /// Ask the host for a copy of the correct board, so the client can resync theirs.
    Resync,
    /// Perform a game action
    GameAction { action: GameAction },
}

impl P2pRequestPacket {
    /// Request to connect to the host. `join_code` is the HEX encoded IP and port of the host,
    /// which is the same as the join code if working over LAN. 'username' is the username the
    /// client wishes to use.
    pub fn connect(join_code: &str, username: &str) -> Self {
        Self::Connect {
            join_code: join_code.to_owned(),
            username: username.to_owned(),
        }
    }
    /// Perform a game action
    pub fn game_action(action: GameAction) -> Self {
        Self::GameAction { action }
    }
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

                bytes.append(&mut join_code.as_bytes().to_vec());
                bytes.append(&mut username.as_bytes().to_vec());
            }
            Self::Resync => {
                bytes.append(&mut self.to_u8().to_be_bytes().to_vec()); // Packet type code
            }
            Self::GameAction { action } => {
                bytes.append(&mut self.to_u8().to_be_bytes().to_vec()); // Packet type code

                bytes.append(&mut action.to_packet());
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
            // Ping
            1 => Ok(Self::Ping),
            // Connect
            2 => {
                if packet.len() < 14 {
                    return Err(PacketError::invalid_length(14, packet.len()).into());
                }
                let join_code = match String::from_utf8(packet[1..13].to_vec()) {
                    Ok(string) => string,
                    Err(_) => {
                        return Err(PacketError::data_error(
                            "Invalid UFT8 encoded values for username",
                        )
                        .into())
                    }
                };
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
            // Resync
            3 => Ok(Self::Resync),
            // Game Action
            4 => {
                if packet.len() < 2 {
                    return Err(PacketError::invalid_length(2, packet.len()).into());
                }
                let action = GameAction::from_packet(packet[1..].to_vec()).unwrap();

                Ok(Self::GameAction { action })
            }
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
            Self::GameAction { action: _ } => 4,
        }
    }
}

/// A response to the `P2pResonse` struct.
#[derive(Clone, Debug)]
pub struct P2pResponse {
    /// The sessions ID set randomly by the host.
    pub session_id: u16,
    /// This specific transactions ID
    pub transaction_id: u16,
    /// The main packet of the response.
    pub packet: P2pResponsePacket,
}

impl P2pResponse {
    /// Create a new `P2pResponse` from the sessions ID and the packet.
    pub fn new(session_id: u16, transaction_id: u16, packet: P2pResponsePacket) -> Self {
        Self {
            session_id,
            transaction_id,
            packet,
        }
    }
}

impl ToPacket for P2pResponse {
    fn to_packet(&self) -> Vec<u8> {
        let mut bytes = vec![];
        bytes.append(&mut 1u8.to_be_bytes().to_vec());
        bytes.append(&mut self.session_id.to_be_bytes().to_vec());
        bytes.append(&mut self.transaction_id.to_be_bytes().to_vec());
        bytes.append(&mut self.packet.to_packet());
        bytes
    }
}

impl FromPacket for P2pResponse {
    fn from_packet(packet: Vec<u8>) -> anyhow::Result<Self> {
        if packet.len() < 6 {
            return Err(PacketError::invalid_length(6, packet.len()).into());
        }

        if packet[0] != 1 {
            return Err(PacketError::InavlidType.into());
        }

        let session_id = u16::from_be_bytes(packet[1..3].try_into().unwrap());
        let transaction_id = u16::from_be_bytes(packet[3..5].try_into().unwrap());
        let packet = P2pResponsePacket::from_packet(packet[5..].to_vec())?;

        Ok(Self {
            session_id,
            transaction_id,
            packet,
        })
    }
}

/// The different types of packets you can send as a response to the other peer.
#[derive(Clone, Debug, PartialEq)]
pub enum P2pResponsePacket {
    /// The packet for if an error has occured.
    Error {
        /// The errors kind.
        kind: P2pError,
    },
    /// The reponse to `P2pRequestPacket::Ping`.
    Pong,
    /// Response to `P2pRequestPacket::Connect`.
    Connect {
        /// The board color that the client will be assigned to.
        client_color: PieceColor,
        /// The hosts username, set by the Hosts user.
        host_username: String,
    },
    /// A response to `P2pRequestPacket::Resync`, features the hosts version of the game board.
    Resync {
        /// The hosts version of the game board, which the client will copy.
        board: Vec<PieceData>,
    },
    /// A simple acknowledge.
    Acknowledge,
}

impl P2pResponsePacket {
    /// The packet for if an error has occured.
    pub fn error(kind: P2pError) -> Self {
        Self::Error { kind }
    }
    /// Response to `P2pRequestPacket::Connect`.
    pub fn connect(client_color: PieceColor, host_username: String) -> Self {
        Self::Connect {
            client_color,
            host_username,
        }
    }
    /// A response to `P2pRequestPacket::Resync`, features the hosts version of the game board.
    pub fn resync(board: Vec<PieceData>) -> Self {
        Self::Resync { board }
    }
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
                    bytes.append(&mut vec![tile.to_u8()]);
                }
            }
            Self::Acknowledge => {
                bytes.append(&mut self.to_u8().to_be_bytes().to_vec());
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
                    match PieceData::try_from(byte) {
                        Ok(piece) => board.push(piece),
                        Err(e) => return Err(PacketError::data_error(&e.to_string()).into()),
                    }
                }

                Ok(Self::Resync { board })
            }
            // Ok
            4 => Ok(Self::Acknowledge),
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
            Self::Acknowledge => 4,
        }
    }
}

impl ToPacket for GameAction {
    fn to_packet(&self) -> Vec<u8> {
        let mut bytes = self.to_u8().to_be_bytes().to_vec();
        match self {
            Self::MovePiece(move_action) => {
                bytes.push(move_action.index as u8);
                bytes.push(move_action.end as u8);

                if let Some(captured) = &move_action.captured {
                    for piece in captured {
                        bytes.push(*piece as u8);
                    }
                }
            }
            _ => {}
        }
        bytes
    }
}

impl FromPacket for GameAction {
    fn from_packet(packet: Vec<u8>) -> anyhow::Result<Self> {
        if packet.len() == 0 {
            return Err(PacketError::invalid_length(1, 0).into());
        }
        match Self::from(packet[0]) {
            Self::MovePiece(_) => {
                if packet.len() < 3 {
                    return Err(PacketError::invalid_length(4, packet.len()).into());
                }
                let to = packet[1] as usize;
                let from = packet[2] as usize;

                let mut captured: Option<Vec<usize>> = None;
                if packet.len() > 3 {
                    captured = Some(vec![]);
                    for i in 3..packet.len() {
                        unsafe {captured.as_mut().unwrap_unchecked().push(packet[i] as usize)}
                    }
                }

                Ok(Self::move_piece(from, to, captured))
            }
            Self::Surrender => {
                if packet.len() != 1 {
                    return Err(PacketError::invalid_length(1, packet.len()).into());
                }
                Ok(Self::Surrender)
            }
            Self::Stalemate => {
                if packet.len() != 1 {
                    return Err(PacketError::invalid_length(1, packet.len()).into());
                }
                Ok(Self::Stalemate)
            }
            _ => Err(
                PacketError::data_error(&format!("Not valid packet type: {}", packet[0])).into(),
            ),
        }
    }
}

impl From<u8> for GameAction {
    fn from(value: u8) -> Self {
        match value {
            0 => {
                Self::MovePiece(Move {index: 0, end: 0, captured: None})
            }
            1 => {
                Self::Stalemate
            }
            2 => {
                Self::Surrender
            }
            _ => {
                panic!("Not valid Gameaction value in 'From' cast")
            }
        }
    }
}

impl ToByte for GameAction {
    fn to_u8(&self) -> u8 {
        match self {
            Self::MovePiece(_) => 0,
            Self::Stalemate => 1,
            Self::Surrender => 2,
        }
    }
}

/// The error used by `P2pResponsePacket`
#[derive(Clone, Debug, PartialEq)]
pub enum P2pError {
    /// This errorkind is caused by the client having an outdated, or invalid board. An example of
    /// when this error is thrown, is when the clients wants to move a piece to an invalid
    /// position.
    InvalidBoard,
    /// This errorkind is caused by the client sending a package with a wrong Join code.
    InvalidJoinCode,
    /// This errorkind is caused by tge client sending a package with an invalid session Id.
    InvalidSessionId,
    /// This errorkind is caused by the client attempting jo join a game that is already full.
    FullGameSession,
    /// THis errorkind is caused by data flowing the wrong direction. E.g. when a Host tries to
    /// send a `P2pRequest::Connect` to the client.
    WrongDirection,
}

impl ToByte for P2pError {
    fn to_u8(&self) -> u8 {
        match self {
            Self::InvalidBoard => 0,
            Self::InvalidJoinCode => 1,
            Self::InvalidSessionId => 2,
            Self::FullGameSession => 3,
            Self::WrongDirection => 4,
        }
    }
}

impl TryFrom<u8> for P2pError {
    type Error = anyhow::Error;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::InvalidBoard),
            1 => Ok(Self::InvalidJoinCode),
            2 => Ok(Self::InvalidSessionId),
            3 => Ok(Self::FullGameSession),
            4 => Ok(Self::WrongDirection),
            _ => Err(anyhow!(
                "Can only take values in range 0..=4 for P2p Error, got {}",
                value
            )),
        }
    }
}

/// THIS IS A TEMP ENUM
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
impl ToByte for PieceData {
    fn to_u8(&self) -> u8 {
        let mut byte: u8 = 0;

        if self.is_active {
            return byte;
        }

        match self.color {
            PieceColor::White => {
                byte |= 0b001;
            }
            PieceColor::Black => {
                byte |= 0b010;
            }
        }

        if self.is_king {
            byte |= 0b100;
        }

        byte
    }
}

impl TryFrom<u8> for PieceData {
    type Error = anyhow::Error;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value == 0 {
            let piece = Self {
                color: PieceColor::White,
                is_active: false,
                is_king: false,
            };
            return Ok(piece);
        }

        if (value & 0b11).count_ones() != 1 {
            return Err(anyhow!("Got byte in wrong format"));
        }

        let color = if value & 0b01 == 1 {
            PieceColor::White
        } else {
            PieceColor::Black
        };

        let is_king = value & 0b100 == 1;

        let piece = Self {
            color,
            is_active: true,
            is_king,
        };
        Ok(piece)
    }
}
