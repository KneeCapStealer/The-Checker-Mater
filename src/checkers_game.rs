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
    UpRight = -3,
    UpLeft = -4,
    DownRight = 4,
    DownLeft = 5,
}

impl Direction {
    fn values() -> [Direction; 4] {
        use Direction::*;
        [UpRight, UpLeft, DownLeft, DownRight]
    }
}

#[derive(Default, Clone)]
pub struct Board {
    game: Weak<GameWindow>,
    tiles: Rc<slint::VecModel<PieceData>>,
    player_color: PieceColor,
    squares: Rc<slint::VecModel<BoardSquare>>,
    #[allow(unused)]
    board_white_color: slint::Brush,
    #[allow(unused)]
    board_black_color: slint::Brush,
    #[allow(unused)]
    piece_white_color: slint::Brush,
    #[allow(unused)]
    piece_black_color: slint::Brush,
}

impl Board {
    pub fn new(game: &GameWindow) -> Board {
        let tiles_vec: Vec<PieceData> = vec![];
        let tiles = Rc::new(slint::VecModel::from(tiles_vec));

        let squares: Vec<BoardSquare> = vec![BoardSquare { marked: false }; 32];

        let squares = Rc::new(slint::VecModel::from(squares));
        game.set_squares(squares.clone().into());

        Board {
            game: game.as_weak(),
            tiles,
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
        self.tiles = Rc::new(slint::VecModel::from(Board::default_setup(color)));

        let game = self.game.unwrap();
        game.set_pieces(self.tiles.clone().into());

        if let Some(moves) = self.get_legal_moves() {
            let mark_indicies: Vec<usize> = moves.iter().map(|mov| mov.end).collect();
            self.mark_squares(mark_indicies.as_slice());
        }
    }

    pub fn move_piece(&mut self, mov: Move) {
        let start_data = self.tiles.row_data(mov.start).unwrap();
        let end_data = self.tiles.row_data(mov.end).unwrap();

        self.tiles.set_row_data(mov.end, start_data);
        self.tiles.set_row_data(mov.start, end_data);

        if let Some(captured) = mov.captured {
            self.tiles.set_row_data(
                captured,
                PieceData {
                    is_active: false,
                    ..self.tiles.row_data(captured).unwrap()
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

    pub fn tile_is_empty(&self, idx: usize) -> bool {
        assert!(idx < self.tiles.row_count());
        !self.tiles.row_data(idx).unwrap().is_active
    }

    pub fn tile_is_player(&self, idx: usize) -> bool {
        assert!(idx < self.tiles.row_count());
        self.tiles.row_data(idx).unwrap().color == self.player_color
    }

    pub fn tile_is_enemy(&self, idx: usize) -> bool {
        assert!(idx < self.tiles.row_count());
        self.tiles.row_data(idx).unwrap().color != self.player_color
    }

    pub fn get_legal_moves_piece(&self, idx: usize) -> Option<Vec<Move>> {
        assert!(idx < self.tiles.row_count());
        let piece = self.tiles.row_data(idx).unwrap();
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
            let next = idx as i32 + direction as i32;
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
            if !piece.is_king {
                if (direction as i32) > 0 && self.tile_is_player(idx) {
                    continue;
                }

                if (direction as i32) < 0 && self.tile_is_enemy(idx) {
                    continue;
                }
            }

            if let Some(mut next_moves) = check_move(
                self.tiles.clone(),
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
        for idx in 0..self.tiles.row_count() {
            if self.tiles.row_data(idx)?.color != self.player_color {
                continue;
            }

            if let Some(mut legal_moves) = self.get_legal_moves_piece(idx) {
                moves.get_or_insert(vec![]).append(&mut legal_moves);
            }
        }

        moves
    }
}
