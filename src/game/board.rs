use super::{BoardSquare, Direction, GameWindow, Move, PieceColor, PieceData};
use slint::ComponentHandle;
use slint::{Model, Weak};
use std::rc::Rc;

/// Struct holding gamestate of the checkers board
#[derive(Default, Clone)]
pub struct Board {
    game: Weak<GameWindow>,
    pieces: Rc<slint::VecModel<PieceData>>,
    player_color: PieceColor,
    squares: Rc<slint::VecModel<BoardSquare>>,
    pub selected_square: i32,
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
                tiles.push(PieceData::const_default());
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
        let mut start_data = self.pieces.row_data(mov.index).unwrap();

        // Promotion to king
        if self.piece_is_player(mov.index) && mov.end < 4 {
            start_data.is_king = true;
        }

        if self.piece_is_enemy(mov.index) && mov.end >= 32 - 4 {
            start_data.is_king = true;
        }

        self.pieces.set_row_data(mov.end, start_data);
        self.pieces
            .set_row_data(mov.index, PieceData::const_default());

        if let Some(captured) = mov.captured {
            for piece in captured {
                self.pieces
                    .set_row_data(piece, PieceData::const_default())
            }
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
        assert!(
            index < self.pieces.row_count(),
            "index ({}) is greater than row_count ({})",
            index,
            self.pieces.row_count()
        );
        let piece = self.pieces.row_data(index).unwrap();
        piece.color == self.player_color && piece.is_active
    }

    /// Returns true if the `index` corresponds to a non-player piece on the board
    pub fn piece_is_enemy(&self, index: usize) -> bool {
        assert!(
            index < self.pieces.row_count(),
            "index ({}) is greater than row_count ({})",
            index,
            self.pieces.row_count()
        );
        let piece = self.pieces.row_data(index).unwrap();
        piece.color != self.player_color && piece.is_active
    }

    pub fn get_player_piece_count(&self) -> u8 {
        let mut count = 0;
        for i in 0..32 {
            count += self.piece_is_player(i) as u8;
        }
        count
    }

    pub fn get_enemy_piece_count(&self) -> u8 {
        let mut count = 0;
        for i in 0..32 {
            count += self.piece_is_enemy(i) as u8;
        }
        count
    }

    pub fn get_empty_piece_count(&self) -> u8 {
        let mut count = 0;
        for i in 0..32 {
            count += self.piece_is_empty(i) as u8;
        }
        count
    }

    /// Get's all the legal moves for the given piece
    /// This works for both enemy pieces and player pieces
    pub fn get_legal_moves_piece(&self, index: usize) -> Option<(Vec<Move>, bool)> {
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
        ) -> Option<(Vec<Move>, bool)> {
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

                return if let Some(mut next_move) = check_move(
                    tiles,
                    start,
                    next as usize,
                    enemy_color,
                    is_king,
                    direction,
                    true,
                ) {
                    if next_move.1 {
                        for i in 0..next_move.0.len() {
                            if next_move.0[i].captured.is_none() {
                                next_move.0.remove(i);
                            }
                        }
                    }

                    Some(next_move)
                } else {
                    None
                };
            }

            // If we are taking a piece, since the next tile is empty
            // We return the available move to take the piece
            if is_taking {
                return Some((vec![Move {
                    index: start,
                    end: next as usize,
                    captured: Some(vec![index]),
                }], false))
            }

            // If we aren't taking a piece, and this tile is piece is empty
            // We add this move to a list of possible moves
            let mut moves = vec![];
            let mut is_taking = false;

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
                    moves.append(&mut next_moves.0);
                    is_taking = next_moves.1;
                }
            }

            if !is_taking {
                moves.push(Move {
                    index: start,
                    end: next as usize,
                    captured: None,
                });
            }

            // Return all the available moves
            // 1 move if normal piece, x amount if king piece
            Some((moves, is_taking))
        }

        let mut moves: Option<Vec<Move>> = None;
        let mut is_taking = false;
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
                is_taking |= next_moves.1;

                if next_moves.1 == is_taking {
                    moves.get_or_insert(vec![]).append(&mut next_moves.0);
                }
            }
        }

        match moves {
            Some(mut moves) => {
                if !is_taking {
                    return Some((moves, is_taking));
                }
                // Remove all non-capturing moves
                    moves = moves
                        .iter()
                        .filter_map(|mov| match &mov.captured {
                            Some(_) => Some(mov.clone()),
                            None => None
                        })
                        .collect();
                Some((moves, is_taking))
            }
            None => None,
        }
    }

    /// Returns all legal moves for the `player_color`
    pub fn get_legal_moves(&self) -> Option<Vec<Move>> {
        let mut moves = None;
        let mut is_taking = false;
        for index in 0..self.pieces.row_count() {
            if self.pieces.row_data(index)?.color != self.player_color {
                continue;
            }

            if let Some(mut legal_moves) = self.get_legal_moves_piece(index) {
                is_taking |= legal_moves.1;
                if legal_moves.1 == is_taking {
                    moves.get_or_insert(vec![]).append(&mut legal_moves.0);
                }
            }
        }
        match moves {
            Some(moves) => {
                if !is_taking {
                    return Some(moves);
                }

                Some(
                    moves
                        .iter()
                        .filter_map(|mov| match &mov.captured {
                            Some(_) => Some(mov.clone()),
                            None => None
                        })
                        .collect(),
                )
            }
            None => None,
        }
    }
}
