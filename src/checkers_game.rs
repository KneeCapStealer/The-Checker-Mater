use slint::{Model, Weak};
use std::rc::Rc;

slint::include_modules!();

impl PieceColor {
    const fn as_opposite(&self) -> Self {
        match self {
            Self::White => Self::Black,
            Self::Black => Self::White,
        }
    }
}

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
    fn values() -> [Direction; 4] {
        use Direction::*;
        [UpRight, UpLeft, DownLeft, DownRight]
    }

    fn from_index(&self, index: usize) -> i32 {
        let row_type = (((index % 8) / 4) as f32).floor() as i32;
        *self as i32 + row_type
    }

    fn is_left(&self) -> bool {
        use Direction::*;
        matches!(self, UpLeft | DownLeft)
    }

    fn is_right(&self) -> bool {
        use Direction::*;
        matches!(self, UpRight | DownRight)
    }

    fn is_down(&self) -> bool {
        use Direction::*;
        matches!(self, DownRight | DownLeft)
    }

    fn is_up(&self) -> bool {
        use Direction::*;
        matches!(self, UpRight | UpLeft)
    }
}

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

    fn default_setup(player_color: PieceColor) -> Vec<PieceData> {
        let enemy_color = player_color.as_opposite();

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

    pub fn mark_squares(&mut self, indices: &[usize]) {
        for idx in indices {
            self.squares
                .set_row_data(*idx, BoardSquare { marked: true });
        }
    }

    pub fn reset_squares(&mut self) {
        for idx in 0..32 {
            self.squares
                .set_row_data(idx, BoardSquare { marked: false });
        }
    }

    pub fn piece_is_empty(&self, idx: usize) -> bool {
        assert!(idx < self.pieces.row_count());
        !self.pieces.row_data(idx).unwrap().is_active
    }

    pub fn piece_is_player(&self, idx: usize) -> bool {
        assert!(idx < self.pieces.row_count());
        self.pieces.row_data(idx).unwrap().color == self.player_color
    }

    pub fn piece_is_enemy(&self, idx: usize) -> bool {
        assert!(idx < self.pieces.row_count());
        self.pieces.row_data(idx).unwrap().color != self.player_color
    }

    pub fn get_legal_moves_piece(&self, idx: usize) -> Option<Vec<Move>> {
        assert!(idx < self.pieces.row_count());
        let piece = self.pieces.row_data(idx).unwrap();
        if !piece.is_active {
            return None;
        }

        fn check_move(
            tiles: Rc<slint::VecModel<PieceData>>,
            start: usize,
            idx: usize,
            enemy_color: PieceColor,
            is_king: bool,
            direction: Direction,
            is_taking: bool,
        ) -> Option<Vec<Move>> {
            let next = idx as i32 + direction.from_index(idx);
            if next < 0 || next > tiles.row_count() as i32 {
                return None;
            }

            let next_tile = tiles.row_data(next as usize)?;
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

            if is_taking {
                return Some(vec![Move {
                    start,
                    end: next as usize,
                    captured: Some(idx),
                }]);
            }

            let mut moves = vec![Move {
                start,
                end: next as usize,
                captured: None,
            }];

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

            Some(moves)
        }

        let mut moves: Option<Vec<Move>> = None;
        for direction in Direction::values() {
            if (((idx % 8) / 4) as f32).floor() as i32 == 0 && direction.is_left() && idx % 4 == 0 {
                continue;
            }

            if (((idx % 8) / 4) as f32).floor() as i32 == 1 && direction.is_right() && idx % 4 == 3
            {
                continue;
            }

            if !piece.is_king {
                if direction.is_down() && self.piece_is_player(idx) {
                    continue;
                }

                if direction.is_up() && self.piece_is_enemy(idx) {
                    continue;
                }
            }

            if let Some(mut next_moves) = check_move(
                self.pieces.clone(),
                idx,
                idx,
                piece.color.as_opposite(),
                piece.is_king,
                direction,
                false,
            ) {
                moves.get_or_insert(vec![]).append(&mut next_moves);
            }
        }
        moves
    }

    pub fn get_legal_moves(&self) -> Option<Vec<Move>> {
        let mut moves = None;
        for idx in 0..self.pieces.row_count() {
            if self.pieces.row_data(idx)?.color != self.player_color {
                continue;
            }

            if let Some(mut legal_moves) = self.get_legal_moves_piece(idx) {
                moves.get_or_insert(vec![]).append(&mut legal_moves);
            }
        }

        moves
    }
}
