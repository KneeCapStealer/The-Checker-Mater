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

/// Struct defining what pieces are moved
/// and an optional captured piece
#[derive(Clone, Copy, Debug)]
pub struct Move {
    pub start: usize,
    pub end: usize,
    pub captured: Option<usize>,
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
