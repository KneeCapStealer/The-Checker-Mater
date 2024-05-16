use arboard::Clipboard;
use slint::ComponentHandle;

use crate::net::interface;

use super::{
    board::{set_board_move, Board},
    GameAction, GameWindow, PieceColor, WindowType,
};
use std::cell::RefCell;
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
        unsafe { self.gamedata.as_ptr().as_ref().unwrap_unchecked() }
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
            let mut gamedata = try_get_static_self().unwrap();
            gamedata.start_new_game(PieceColor::Black);

            gamedata.load_prompt_client_window();

            gamedata.window.on_join_prompt({
                let mut gamedata = try_get_static_self().unwrap();

                move || {
                    let mut join_code: String = gamedata.window.get_lan_code().into();
                    join_code = join_code.trim().to_owned();

                    println!("Code was: \"{}\"", &join_code);

                    gamedata.load_connecting_window(join_code.clone(), false);

                    interface::start_lan_client();

                    let username: String = gamedata.window.get_username().into();

                    let handle_weak = gamedata.window.as_weak();
                    tokio::spawn(async move {
                        let (color, host_username) =
                            interface::connect_to_host_loop(&join_code, &username).unwrap();

                        println!("Joined {}'s game. You are {:?}", host_username, color);

                        let handle_copy = handle_weak.clone();
                        slint::invoke_from_event_loop(move || {
                            handle_copy
                                .unwrap()
                                .invoke_set_usernames(username.into(), host_username.into());
                        })
                        .unwrap();

                        let handle_copy = handle_weak.clone();
                        slint::invoke_from_event_loop(move || {
                            handle_copy.unwrap().invoke_load_game_window();
                        })
                        .unwrap();
                    });

                    gamedata.get_board_mut().start_new_game(PieceColor::Black);
                    gamedata.wait_for_opponent();
                }
            });
        }
    }

    pub fn on_host_game(&self) -> impl FnMut() + 'static {
        let mut try_get_static_self = self.try_get_static_func();

        move || {
            let mut gamedata = try_get_static_self().unwrap();
            let join_code = interface::start_lan_host();

            gamedata.load_connecting_window(join_code.clone(), true);

            let mut clipboard = Clipboard::new().unwrap();
            clipboard.set_text(join_code).unwrap();

            let username: String = gamedata.window.get_username().into();
            interface::set_my_username(&username);

            let handle_weak = gamedata.window.as_weak();
            std::thread::spawn(move || {
                loop {
                    if interface::is_connected() {
                        break;
                    }
                    // Think this is important
                    sleep(Duration::from_millis(50));
                }

                let client_username =
                    interface::get_other_username().unwrap_or("NO USERNAME".to_owned());

                let handle_copy = handle_weak.clone();
                slint::invoke_from_event_loop(move || {
                    handle_copy
                        .unwrap()
                        .invoke_set_usernames(username.into(), client_username.into());
                })
                .unwrap();

                let handle_copy = handle_weak.clone();
                slint::invoke_from_event_loop(move || {
                    handle_copy.unwrap().invoke_load_game_window();
                })
                .unwrap();
            });
            gamedata.start_new_game(PieceColor::White);
            gamedata.is_player_turn = true;
        }
        // self.on_join_game()
    }

    pub fn on_board_clicked(&self) -> impl FnMut(i32) + 'static {
        let mut try_get_static_self = self.try_get_static_func();

        move |index: i32| {
            let mut gamedata = try_get_static_self().unwrap();
            let board = gamedata.get_board_mut();

            let mut gamedata = try_get_static_self().unwrap();

            let selected_piece = board.selected_square as usize;

            if !gamedata.is_player_turn {
                return;
            }

            if board.piece_is_player(selected_piece) {
                let legal_moves = board.get_legal_moves();
                if let Some(moves) = legal_moves {
                    for mov in &moves {
                        let input_matches_move =
                            mov.end == index as usize && mov.index == selected_piece;

                        board.selected_square = index;

                        if input_matches_move {
                            set_board_move(mov);
                            gamedata.window.invoke_move_piece();
                            interface::send_game_action(GameAction::MovePiece(mov.clone()), |_| ());
                            gamedata.wait_for_opponent();
                            break;
                        }
                    }
                }
            }
            // If there was no move with the input
            board.reset_squares();
            if let Some(moves) = board.get_legal_moves_piece(index as usize) {
                let mark_indicies: Vec<usize> = moves.0.iter().map(|mov| mov.end).collect();
                board.mark_squares(mark_indicies.as_slice());
            }
            board.selected_square = index;
        }
    }

    pub fn on_move_piece(&self) -> impl FnMut() + 'static {
        let mut try_get_static_self = self.try_get_static_func();

        move || {
            let mut gamedata = try_get_static_self().unwrap();
            gamedata.get_board_mut().move_piece();

            gamedata.is_player_turn = true;
        }
    }

    pub fn wait_for_opponent(&mut self) {
        self.is_player_turn = false;
        let weak_window = self.window.as_weak();
        tokio::spawn(async move {
            let mut action;
            loop {
                action = interface::get_next_game_action();
                if action.is_none() {
                    tokio::time::sleep(Duration::from_millis(50)).await;
                    continue;
                }
                break;
            }

            let action = unsafe { action.unwrap_unchecked() };
            match action {
                GameAction::MovePiece(mov) => {
                    println!("Recieved move: {:#?}", mov);
                    set_board_move(&mov.reverse());
                    slint::invoke_from_event_loop(move || {
                        weak_window.unwrap().invoke_move_piece();
                    })
                    .unwrap();
                }
                _ => {
                    println!(
                        "Got GameAction {:?} while waiting for opponent,
                                     this is not implemented yet",
                        action
                    );
                }
            }
        });
    }
}

pub struct GameData {
    window: GameWindow,
    board: Board,
    is_host: Option<bool>,
    is_player_turn: bool,
}

impl GameData {
    pub fn new() -> Result<Self, slint::PlatformError> {
        let window = GameWindow::new()?;
        let board = Board::new(&window);

        Ok(GameData {
            window,
            board,
            is_host: None,
            is_player_turn: false,
        })
    }

    #[inline]
    pub fn get_window(&self) -> &GameWindow {
        &self.window
    }

    fn get_board_mut(&mut self) -> &mut Board {
        &mut self.board
    }

    pub fn start_new_game(&mut self, your_color: PieceColor) {
        self.get_board().start_new_game(your_color);
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
