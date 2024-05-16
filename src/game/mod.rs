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

#[derive(Clone, Debug)]
pub struct Move {
    pub index: usize,
    pub end: usize,
    pub promoted: bool,
    pub captured: Option<Vec<usize>>,
}

impl Move {
    fn reverse(&self) -> Self {
        let captured = self.captured.as_ref().map(|captured| {
            let mut captured = captured.clone();
            captured.iter_mut().for_each(|piece| *piece = 31 - *piece);

            captured
        });

        Self {
            index: 31 - self.index,
            end: 31 - self.end,
            promoted: self.promoted,
            captured,
        }
    }
}

/// An enum which holds the possible actions a user can make in the game.
#[derive(Clone, Debug)]
pub enum GameAction {
    /// Move a piece, by its current position, and its target position.
    /// It is not guarenteed that this move is valid yet, so it should be validated before use.
    MovePiece(Move),
    /// Indicated that the player want's to suggest a stalemate
    Stalemate,
    /// Indicates that the player want's to end the game by surrender
    Surrender,
}

impl GameAction {
    /// Creates a `GameAction::MovePiece`.
    /// * `start` - The start location of the piece.
    /// * `end` - The end location of the piece.
    /// * `captured` - If the move has captured a piece(s), this holds the index of the piece(s).
    pub fn move_piece(
        index: usize,
        end: usize,
        captured: Option<Vec<usize>>,
        promoted: bool,
    ) -> Self {
        Self::MovePiece(Move {
            index,
            end,
            captured,
            promoted,
        })
    }
}

#[derive(Clone, Copy, Debug)]
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
