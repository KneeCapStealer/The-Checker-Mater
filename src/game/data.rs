use arboard::Clipboard;
use slint::ComponentHandle;

use crate::net::interface;

use super::{board::Board, GameWindow, PieceColor, WindowType};
use std::cell::{RefCell, RefMut};
use std::ops::{Deref, DerefMut};
use std::rc::Rc;
use std::thread::sleep;
use std::time::Duration;

pub struct Context {
    gamedata: Rc<RefCell<GameData>>,
}

impl Context {
    pub fn new() -> Result<Self, slint::PlatformError> {
        Ok(Self {
            gamedata: Rc::new(RefCell::new(GameData::new()?)),
        })
    }

    pub fn try_get_static_func(&self) -> impl FnMut() -> Option<Self> + 'static {
        let weak = Rc::downgrade(&self.gamedata);

        move || {
            if let Some(gamedata) = weak.upgrade() {
                return Some(Self { gamedata });
            }

            None
        }
    }
}

impl Deref for Context {
    type Target = GameData;

    fn deref(&self) -> &Self::Target {
        unsafe { self.gamedata.as_ptr().as_mut().unwrap_unchecked() }
    }
}

impl DerefMut for Context {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.gamedata.as_ptr().as_mut().unwrap_unchecked() }
    }
}

impl Context {
    pub fn on_join_game(&self) -> impl FnMut() + 'static {
        let mut try_get_static_self = self.try_get_static_func();

        move || {
            let gamedata = try_get_static_self().unwrap();

            gamedata.load_prompt_client_window();

            gamedata.window.on_join_prompt({
                let gamedata = try_get_static_self().unwrap();

                move || {
                    println!("Code was: \"{}\"", gamedata.window.get_lan_code());
                    let join_code: String = gamedata.window.get_lan_code().into();
                    gamedata.load_connecting_window(join_code.clone(), false);
                    interface::start_lan_client();
                    let handle_weak = gamedata.window.as_weak();
                    tokio::spawn(async move {
                        let _ = interface::connect_to_host_loop(&join_code, "CLIENT");
                        let handle_copy = handle_weak.clone();
                        slint::invoke_from_event_loop(move || {
                            handle_copy.unwrap().invoke_load_game_window();
                        })
                        .unwrap();
                    });
                }
            });
        }
    }

    pub fn on_host_game(&self) -> impl FnMut() + 'static {
        let mut try_get_static_self = self.try_get_static_func();

        move || {
            let gamedata = try_get_static_self().unwrap();

            let join_code = interface::start_lan_host();

            gamedata.load_connecting_window(join_code.clone(), true);

            let mut clipboard = Clipboard::new().unwrap();
            clipboard.set_text(join_code);

            let handle_weak = gamedata.window.as_weak();
            std::thread::spawn(move || {
                loop {
                    if interface::is_connected() {
                        break;
                    }
                    // Think this is important
                    sleep(Duration::from_millis(50));
                }
                let handle_copy = handle_weak.clone();
                slint::invoke_from_event_loop(move || {
                    handle_copy.unwrap().invoke_load_game_window();
                })
                .unwrap();
            });
        }
        // self.on_join_game()
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

pub struct GameData {
    pub window: GameWindow,
    pub board: Rc<RefCell<Board>>,
    pub is_host: Option<bool>,
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

    pub fn load_start_window(&self) {
        self.window.set_window_state(WindowType::Start);
    }

    pub fn load_game_window(&self) {
        self.window.set_window_state(WindowType::Game);
    }

    pub fn load_connecting_window(&self, join_code: String, is_host: bool) {
        self.window.set_join_code(join_code.into());
        self.window.set_is_host(is_host);
        self.window.set_window_state(WindowType::Connecting);
    }

    pub fn load_prompt_client_window(&self) {
        self.window.set_window_state(WindowType::LanPrompt);
    }
}
