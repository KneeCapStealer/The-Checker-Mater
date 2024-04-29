use slint::{Model, Weak};
use std::rc::Rc;

slint::include_modules!();

impl PieceColor {
    /// Get the opposite color
    const fn get_opposite(&self) -> Self {
        match self {
            Self::White => Self::Black,
            Self::Black => Self::White,
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
    fn get_value(&self, index: usize) -> i32 {
        let row_type = (((index % 8) / 4) as f32).floor() as i32;
        *self as i32 + row_type
    }

    /// Returns wether the direction is left
    fn is_left(&self) -> bool {
        use Direction::*;
        matches!(self, UpLeft | DownLeft)
    }

    /// Returns wether the direction is right
    fn is_right(&self) -> bool {
        use Direction::*;
        matches!(self, UpRight | DownRight)
    }

    /// Returns wether the direction is down
    fn is_down(&self) -> bool {
        use Direction::*;
        matches!(self, DownRight | DownLeft)
    }

    /// Returns wether the direction is up
    fn is_up(&self) -> bool {
        use Direction::*;
        matches!(self, UpRight | UpLeft)
    }
}

/// Struct holding gamestate of the checkers board
#[derive(Default, Clone)]
pub struct Board {
    game: Weak<GameWindow>,
    pieces: Rc<slint::VecModel<PieceData>>,
    player_color: PieceColor,
    squares: Rc<slint::VecModel<BoardSquare>>,
}

impl Board {
    pub fn new(game: &GameWindow) -> Board {
        let pieces = Rc::new(slint::VecModel::from(vec![]));

        let squares: Vec<BoardSquare> = vec![BoardSquare { marked: false }; 32];
        let squares = Rc::new(slint::VecModel::from(squares));
        game.set_squares(squares.clone().into());

        Board {
            game: game.as_weak(),
            pieces,
            squares,
            ..Default::default()
        }
    }

    /// Returns the starting setup of a checkers board based off `player_color`
    fn default_setup(player_color: PieceColor) -> Vec<PieceData> {
        let enemy_color = player_color.get_opposite();

        let mut tiles: Vec<PieceData> = vec![
            PieceData {
                is_active: true,
                color: enemy_color,
                is_king: false,
            };
            12
        ];

        for i in 12..32 {
            if i < 20 {
                tiles.push(PieceData {
                    is_active: false,
                    color: enemy_color,
                    is_king: false,
                });
                continue;
            }

            tiles.push(PieceData {
                is_active: true,
                color: player_color,
                is_king: false,
            });
        }

        tiles
    }

    /// Resets the board to starting state based off `player_color`
    pub fn start_new_game(&mut self, color: PieceColor) {
        self.player_color = color;
        self.pieces = Rc::new(slint::VecModel::from(Board::default_setup(color)));

        let game = self.game.unwrap();
        game.set_pieces(self.pieces.clone().into());

        self.reset_squares();
        if let Some(moves) = self.get_legal_moves() {
            let mark_indicies: Vec<usize> = moves.iter().map(|mov| mov.end).collect();
            self.mark_squares(mark_indicies.as_slice());
        }
    }

    /// Takes a `Move` struct and performs the move described within
    pub fn move_piece(&mut self, mov: Move) {
        let start_data = self.pieces.row_data(mov.start).unwrap();
        let end_data = self.pieces.row_data(mov.end).unwrap();

        self.pieces.set_row_data(mov.end, start_data);
        self.pieces.set_row_data(mov.start, end_data);

        if let Some(captured) = mov.captured {
            self.pieces.set_row_data(
                captured,
                PieceData {
                    is_active: false,
                    ..self.pieces.row_data(captured).unwrap()
                },
            )
        }
    }

    /// Gives all the squares in `indices` the "marked" color
    pub fn mark_squares(&mut self, indices: &[usize]) {
        for index in indices {
            self.squares
                .set_row_data(*index, BoardSquare { marked: true });
        }
    }

    /// Turns all squares back to their original color
    pub fn reset_squares(&mut self) {
        for index in 0..32 {
            self.squares
                .set_row_data(index, BoardSquare { marked: false });
        }
    }

    /// Returns true if the `index` corresponds to an active piece on the board
    pub fn piece_is_empty(&self, index: usize) -> bool {
        assert!(index < self.pieces.row_count());
        !self.pieces.row_data(index).unwrap().is_active
    }

    /// Returns true if the `index` corresponds to a player piece on the board
    pub fn piece_is_player(&self, index: usize) -> bool {
        assert!(index < self.pieces.row_count());
        self.pieces.row_data(index).unwrap().color == self.player_color
    }

    /// Returns true if the `index` corresponds to a non-player piece on the board
    pub fn piece_is_enemy(&self, index: usize) -> bool {
        assert!(index < self.pieces.row_count());
        self.pieces.row_data(index).unwrap().color != self.player_color
    }

    /// Get's all the legal moves for the given piece
    /// This works for both enemy pieces and player pieces
    pub fn get_legal_moves_piece(&self, index: usize) -> Option<Vec<Move>> {
        assert!(index < self.pieces.row_count());
        let piece = self.pieces.row_data(index)?;
        if !piece.is_active {
            return None;
        }

        fn check_move(
            tiles: Rc<slint::VecModel<PieceData>>,
            start: usize,
            index: usize,
            enemy_color: PieceColor,
            is_king: bool,
            direction: &Direction,
            is_taking: bool,
        ) -> Option<Vec<Move>> {
            let next = index as i32 + direction.get_value(index);
            if next < 0 || next > tiles.row_count() as i32 {
                return None;
            }

            let next_tile = tiles.row_data(next as usize)?;
            // If the next tile is an enemy check if the tile behind it is empty
            // If so this piece can be taken
            if next_tile.is_active {
                if next_tile.color != enemy_color || is_taking {
                    return None;
                }

                return check_move(
                    tiles,
                    start,
                    next as usize,
                    enemy_color,
                    is_king,
                    direction,
                    true,
                );
            }

            // If we are taking a piece, since the next tile is empty
            // We return the available move to take the piece
            if is_taking {
                return Some(vec![Move {
                    start,
                    end: next as usize,
                    captured: Some(index),
                }]);
            }

            // If we aren't taking a piece, and this tile is piece is empty
            // We add this move to a list of possible moves
            let mut moves = vec![Move {
                start,
                end: next as usize,
                captured: None,
            }];

            // If the current piece is a king, it may be able to keep moving
            if is_king {
                if let Some(mut next_moves) = check_move(
                    tiles,
                    start,
                    next as usize,
                    enemy_color,
                    is_king,
                    direction,
                    false,
                ) {
                    moves.append(&mut next_moves);
                }
            }

            // Return all the available moves
            // 1 move if normal piece, x amount if king piece
            Some(moves)
        }

        let mut moves: Option<Vec<Move>> = None;
        for direction in Direction::values() {
            // Check if the piece is on the edge of the direction
            if index % 8 < 4 && direction.is_left() && index % 4 == 0 {
                continue;
            }

            if index % 8 > 4 && direction.is_right() && index % 4 == 3 {
                continue;
            }

            // If the piece isn't a king it cant move backwards
            if !piece.is_king {
                if direction.is_down() && self.piece_is_player(index) {
                    continue;
                }

                if direction.is_up() && self.piece_is_enemy(index) {
                    continue;
                }
            }

            // Since the direction is valid, run the check move algorithm
            if let Some(mut next_moves) = check_move(
                self.pieces.clone(),
                index,
                index,
                piece.color.get_opposite(),
                piece.is_king,
                direction,
                false,
            ) {
                moves.get_or_insert(vec![]).append(&mut next_moves);
            }
        }
        moves
    }

    /// Returns all legal moves for the `player_color`
    pub fn get_legal_moves(&self) -> Option<Vec<Move>> {
        let mut moves = None;
        for index in 0..self.pieces.row_count() {
            if self.pieces.row_data(index)?.color != self.player_color {
                continue;
            }

            if let Some(mut legal_moves) = self.get_legal_moves_piece(index) {
                moves.get_or_insert(vec![]).append(&mut legal_moves);
            }
        }

        moves
    }
}
