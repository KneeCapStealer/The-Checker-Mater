use slint::ComponentHandle;

use the_checker_mater::{checkers_game::{Board, GameWindow, PieceColor}, game_data::GameData};

fn main() -> Result<(), slint::PlatformError> {
    let mut gamedata = GameData::new()?;

    gamedata.new_game();
    gamedata.get_window().on_clicked(gamedata.on_board_clicked());

    gamedata.get_window().run()
}
