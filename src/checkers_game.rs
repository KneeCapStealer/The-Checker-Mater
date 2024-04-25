use slint::{Model, Weak};
use std::rc::Rc;

slint::include_modules!();

impl PieceColor {
    const fn as_oposite(&self) -> Self {
        match self {
            Self::White => Self::Black,
            Self::Black => Self::White,
        }
    }
}

#[derive(Clone, Copy)]
pub struct Move {
    start: usize,
    end: usize,
    captured: Option<usize>,
}

type Tile = Option<PieceData>;

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

#[derive(Default)]
pub struct Board {
    game: Weak<GameWindow>,
    pieces_model: Rc<slint::VecModel<PieceData>>,
    tiles: [Tile; 32],
    player_color: PieceColor,
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
        let pieces_vec: Vec<PieceData> = game.get_pieces().iter().collect();
        let pieces_model = Rc::new(slint::VecModel::from(pieces_vec));

        let tiles = Board::default_setup(PieceColor::default());

        Board {
            game: game.as_weak(),
            pieces_model,
            tiles,
            ..Default::default()
        }
    }

    fn default_setup(player_color: PieceColor) -> [Tile; 32] {
        let enemy_color = match player_color {
            PieceColor::White => PieceColor::Black,
            PieceColor::Black => PieceColor::White,
        };

        let mut tiles: [Tile; 32] = Default::default();
        for (i, tile) in tiles.iter_mut().enumerate() {
            *tile = if i < 4 * 3 {
                Some(PieceData {
                    color: enemy_color,
                    index: i as i32,
                    is_king: false,
                })
            } else if i >= 4 * 5 {
                Some(PieceData {
                    color: player_color,
                    index: i as i32,
                    is_king: false,
                })
            } else {
                None
            }
        }

        tiles
    }

    fn get_pieces_vec(&self) -> Vec<PieceData> {
        self.tiles
            .as_ref()
            .iter()
            .filter_map(|tile| tile.clone())
            .collect()
    }

    pub fn start_new_game(&mut self, color: PieceColor) {
        self.player_color = color;
        self.tiles = Board::default_setup(color);
        self.pieces_model = Rc::new(slint::VecModel::from(self.get_pieces_vec()));

        let game = self.game.unwrap();
        game.set_pieces(self.pieces_model.clone().into());
    }

    pub fn tile_is_empty(&self, idx: usize) -> bool {
        assert!(idx < self.tiles.len());
        self.tiles[idx].is_none()
    }

    pub fn tile_is_player(&self, idx: usize) -> bool {
        assert!(idx < self.tiles.len());
        self.tiles[idx]
            .as_ref()
            .is_some_and(|x| x.color == self.player_color)
    }

    pub fn tile_is_enemy(&self, idx: usize) -> bool {
        assert!(idx < self.tiles.len());
        self.tiles[idx]
            .as_ref()
            .is_some_and(|x| x.color != self.player_color)
    }

    pub fn move_piece(&mut self, mov: Move) {
        todo!();
    }

    pub fn get_legal_moves_piece(&self, idx: usize) -> Option<Vec<Move>> {
        assert!(idx < self.tiles.len());
        let piece = match &self.tiles[idx] {
            Some(val) => val,
            None => return None,
        };

        fn check_move(
            tiles: &[Tile; 32],
            start: usize,
            idx: usize,
            enemy_color: PieceColor,
            is_king: bool,
            direction: Direction,
            is_taking: bool,
        ) -> Option<Vec<Move>> {
            let next = idx as i32 + direction as i32;
            if next < 0 {
                return None;
            }

            match &tiles[next as usize] {
                Some(tile) => {
                    if tile.color != enemy_color {
                        return None;
                    }
                    if is_taking {
                        return None;
                    }

                    check_move(
                        tiles,
                        start,
                        next as usize,
                        enemy_color,
                        is_king,
                        direction,
                        true,
                    )
                }
                None => {
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
            }
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
                &self.tiles,
                idx,
                idx,
                piece.color.as_oposite(),
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
        let pieces = self.get_pieces_vec();
        let player_pieces = pieces.iter().filter(|tile| tile.color == self.player_color);

        let mut moves = None;
        for piece in player_pieces {
            if let Some(mut legal_moves) = self.get_legal_moves_piece(piece.index as usize) {
                moves.get_or_insert(vec![]).append(&mut legal_moves);
            }
        }

        moves
    }
}
