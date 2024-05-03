use std::{borrow::BorrowMut, process::exit};

use slint::ComponentHandle;

use std::{cell::RefCell, rc::Rc};

use the_checker_mater::{
    checkers_game::{Board, GameWindow, PieceColor},
    game_data::GameData,
};

fn main() -> Result<(), slint::PlatformError> {
    let mut gamedata: GameData = GameData::new()?;

    let window = gamedata.get_window();

    window.on_clicked(gamedata.on_board_clicked());

    window.on_join_game(gamedata.on_join_game());
    window.on_host_game(gamedata.on_host_game());

    window.on_exit(|| {
        exit(0);
    });

    gamedata.start_new_game();

    let window = gamedata.get_window();
    window.run()
}
