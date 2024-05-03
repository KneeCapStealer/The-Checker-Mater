use std::{env, thread::sleep, time::Duration};

use crate::net::interface;

pub mod net;

#[tokio::main]
async fn main() {
    let args = env::args().collect::<Vec<String>>();
    println!("WELCOME!");

    match args[1].to_lowercase().as_str() {
        "host" => {
            let join_code = interface::start_lan_host().await;
            println!("JOIN CODE:\n{}", join_code);
        }
        "join" => interface::start_lan_client(&args[2]).await,
        _ => {}
    }
    sleep(Duration::from_secs(60));
}

// slint::include_modules!();
//
// fn main() -> Result<(), slint::PlatformError> {
//     let game = GameWindow::new()?;
//
//     game.on_clicked(|index: i32| {
//         std::println!("Number {}", index);
//     });
//
//     game.run()
// }
