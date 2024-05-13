slint::include_modules!();

mod board;
pub mod data;

impl PieceColor {
    /// Get the opposite color
    const fn get_opposite(&self) -> Self {
        match self {
            Self::White => Self::Black,
            Self::Black => Self::White,
        }
    }
}

impl PieceData {
    const fn const_default() -> Self {
        PieceData {
            is_king: false,
            is_active: false,
            color: PieceColor::White,
        }
    }
}

/// An enum which holds the possible actions a user can make in the game.
#[derive(Clone, Copy, Debug)]
pub enum GameAction {
    /// Move a piece, by its current position, and its target position.
    /// It is not guarenteed that this move is valid yet, so it should be validated before use.
    MovePiece(Move),
    /// Indicates that the player wants to end the game by surrender
    Surrender,
}
impl GameAction {
    /// Creates a `GameAction::MovePiece`.
    /// * `start` - The start location of the piece.
    /// * `end` - The end location of the piece.
    /// * `captured` - If the move has captured a piece, this holds the index of that piece.
    pub fn move_piece(start: usize, end: usize, captured: Option<usize>) -> Self {
        Self::MovePiece(Move::new(start, end, captured))
    }
}

/// Struct defining what pieces are moved
/// and an optional captured piece
#[derive(Clone, Copy, Debug)]
pub struct Move {
    pub start: usize,
    pub end: usize,
    pub captured: Option<usize>,
}
impl Move {
    /// Creates a new instance of `Move`.
    pub fn new(start: usize, end: usize, captured: Option<usize>) -> Self {
        Self {
            start,
            end,
            captured,
        }
    }
}

#[derive(Clone, Copy)]
enum Direction {
    UpLeft = -5,
    UpRight = -4,
    DownLeft = 3,
    DownRight = 4,
}

impl Direction {
    /// Returns an array to iterate over all enum values
    const fn values() -> &'static [Direction; 4] {
        use Direction::*;
        &[UpRight, UpLeft, DownLeft, DownRight]
    }

    /// Get's the value used to traverse the array of tiles in
    /// the Board struct, based off the index of the piece
    const fn get_value(&self, index: usize) -> i32 {
        let row_type: i32 = ((index % 8) / 4) as i32;
        *self as i32 + row_type
    }

    /// Returns wether the direction is left
    const fn is_left(&self) -> bool {
        use Direction::*;
        matches!(self, UpLeft | DownLeft)
    }

    /// Returns wether the direction is right
    const fn is_right(&self) -> bool {
        use Direction::*;
        matches!(self, UpRight | DownRight)
    }

    /// Returns wether the direction is down
    const fn is_down(&self) -> bool {
        use Direction::*;
        matches!(self, DownRight | DownLeft)
    }

    /// Returns wether the direction is up
    const fn is_up(&self) -> bool {
        use Direction::*;
        matches!(self, UpRight | UpLeft)
    }
}
