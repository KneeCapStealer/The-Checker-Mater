use slint::ComponentHandle;

use the_checker_mater::checkers_game::{Board, GameWindow, PieceColor};

static mut SELECTED_PIECE: usize = 0;

fn main() -> Result<(), slint::PlatformError> {
    let game = GameWindow::new()?;
    let mut board = Board::new(&game);

    board.start_new_game(PieceColor::White);

    game.on_clicked({
        let mut board = board.clone();

        move |index: i32| {
            let selected_piece = unsafe { SELECTED_PIECE };

            if board.piece_is_player(selected_piece) {
                let moves = board.get_legal_moves_piece(selected_piece);
                if let Some(moves) = moves {
                    for mov in moves {
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

            unsafe {
                SELECTED_PIECE = index as usize;
            }
        }
    });

    game.run()
}
