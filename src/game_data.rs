use slint::ComponentHandle;

use crate::checkers_game::{Board, GameWindow, PieceColor, WindowType};
use std::cell::{RefCell, RefMut};
use std::rc::Rc;

pub struct GameData {
    window: GameWindow,
    board: Rc<RefCell<Board>>,
    is_host: Option<bool>,
}

impl GameData {
    pub fn new() -> Result<Self, slint::PlatformError> {
        let window = GameWindow::new()?;
        let board = Rc::new(RefCell::new(Board::new(&window)));

        Ok(GameData {
            window,
            board,
            is_host: None,
        })
    }

    #[inline]
    pub fn get_window(&self) -> &GameWindow {
        &self.window
    }

    fn get_board(&self) -> RefMut<Board> {
        self.board.as_ref().borrow_mut()
    }

    pub fn start_new_game(&mut self) {
        self.get_board().start_new_game(PieceColor::default());
    }

    pub fn load_start_screen(&self) {
        self.window.set_window_state(WindowType::Start);
    }

    pub fn load_game_screen(&self) {
        self.window.set_window_state(WindowType::Game);
    }

    pub fn on_join_game(&self) -> impl FnMut() + 'static {
        let window_weak = self.window.as_weak();
        move || {
            let window = window_weak.upgrade().unwrap();
            window.set_window_state(WindowType::Game);
        }
    }

    pub fn on_host_game(&self) -> impl FnMut() + 'static {
        self.on_join_game()
    }

    pub fn on_board_clicked(&self) -> impl FnMut(i32) + 'static {
        let board_weak = Rc::downgrade(&self.board);

        move |index: i32| {
            let strong_board = board_weak.upgrade().unwrap();
            let mut board = strong_board.as_ref().borrow_mut();
            let selected_piece = board.selected_square as usize;

            if board.piece_is_player(selected_piece) {
                let moves = board.get_legal_moves_piece(selected_piece);
                if let Some(moves) = moves {
                    for mov in moves.0 {
                        if mov.end == index as usize {
                            board.move_piece(mov);
                        }
                    }
                }
            }

            board.reset_squares();
            if let Some(moves) = board.get_legal_moves() {
                let mark_indicies: Vec<usize> = moves.iter().map(|mov| mov.end).collect();
                board.mark_squares(mark_indicies.as_slice());
            }

            board.selected_square = index;
        }
    }
}
