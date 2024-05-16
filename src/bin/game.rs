use std::process::exit;

use slint::ComponentHandle;

use the_checker_mater::game::data::Context;

#[tokio::main]
async fn main() -> Result<(), slint::PlatformError> {
    let mut gamedata = Context::new()?;

    let window = gamedata.get_window();

    window.on_clicked(gamedata.on_board_clicked());

    window.on_join_game(gamedata.on_join_game());
    window.on_host_game(gamedata.on_host_game());

    window.on_exit(|| {
        exit(0);
    });

    let window = gamedata.get_window();
    window.run()
}
