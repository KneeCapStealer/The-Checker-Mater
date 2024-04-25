use slint::ComponentHandle;

use the_checker_mater::checkers_game::Board;
use the_checker_mater::checkers_game::GameWindow;
use the_checker_mater::checkers_game::PieceColor;

fn main() -> Result<(), slint::PlatformError> {
    let game = GameWindow::new()?;
    let mut board = Board::new(&game);

    board.start_new_game(PieceColor::White);

    game.run()
}
