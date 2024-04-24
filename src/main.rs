pub mod net;

slint::include_modules!();

fn main() -> Result<(), slint::PlatformError> {
    let game = GameWindow::new()?;

    game.on_clicked(|index: i32| {
        std::println!("Number {}", index);
    });

    game.run()
}
