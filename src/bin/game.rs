use slint::Model;

slint::include_modules!();

fn main() -> Result<(), slint::PlatformError> {
    let game = GameWindow::new()?;
    let pieces = game.get_pieces();

    game.set_pieces(pieces.clone());

    game.on_clicked({
        let game_weak = game.as_weak();
        let pieces = pieces.clone();

        move |index: i32, last_pressed: i32| {
            let game = game_weak.unwrap();

            let new_data = PieceData {
                index,
                ..pieces.row_data(0).unwrap()
            };

            pieces.set_row_data(0, new_data);
        }
    });

    game.run()
}
