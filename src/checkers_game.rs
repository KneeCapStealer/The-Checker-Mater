use slint::Model;
use std::rc::Rc;


slint::include_modules!();

pub struct Move {
    start: usize,
    end: usize,
    captured: Option<usize>,
}

pub type Tile = Option<PieceData>;

#[derive(Default)]
pub struct Board {
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
            *tile = if i < 24 {
                Some(PieceData {color: enemy_color, index: i as i32, is_king: false})
            } else if i < 24 + 8 {
                None
            } else {
                Some(PieceData {color: player_color, index: i as i32, is_king: false})
            }
        }

        tiles
    }

    pub fn start_new_game(&mut self, color: PieceColor) {
        todo!();
    }

    pub fn tile_is_empty(&self, idx: usize) -> bool {
        todo!();
    }

    pub fn tile_is_player(&self, idx: usize) -> bool {
        todo!();
    }
    
    pub fn tile_is_enemy(&self, idx: usize) -> bool {
        todo!();
    }

    pub fn move_piece(&mut self, piece: usize, destination: usize) {
        todo!();
    }

    pub fn get_legal_moves(&self) -> Vec<Move> {
        todo!();
    }
}
