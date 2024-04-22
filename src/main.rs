slint::include_modules!();

fn main() -> Result<(), slint::PlatformError> {
    let game = GameWindow::new()?;

    game.run()
}
