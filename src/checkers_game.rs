use slint::Model;
use std::rc::Rc;

slint::include_modules!();

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

        // TODO: Generate default tiles array

        Board {
            pieces_model,
            ..Default::default()
        }
    }

    fn default_setup() -> [Tile; 32] {
        todo!();
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

    pub fn get_legal_moves(&self) -> Vec<usize> {
        todo!();
    }

    pub fn get_piece_legal_moves(&self, piece: usize) -> Vec<usize> {
        todo!();
    }
}
