pub mod net;

use crate::net::{
    interface::{self, get_next_game_action},
    status::get_connection_status,
};
use std::{env, thread::sleep, time::Duration};

#[tokio::main]
async fn main() {
    let args = env::args().collect::<Vec<String>>();
    println!("WELCOME!");

    match args[1].to_lowercase().as_str() {
        "host" => {
            let join_code = interface::start_lan_host().await;
            println!("JOIN CODE:\n{}", join_code);
            while !get_connection_status().await.is_connected() {}

            sleep(Duration::from_secs(3));

            interface::send_game_action(interface::GameAction::Surrender, |_| {
                println!("Got resp!! (HELL YES!!)");
            });

            sleep(Duration::from_secs(3));

            interface::send_game_action(interface::GameAction::Surrender, |_| {
                println!("Got resp!! (HELL YES!!)");
            })
        }
        "join" => {
            interface::start_lan_client().await;

            interface::connect_to_host_loop(&args[2], "CLIENT").await;

            loop {
                if let Some(action) = get_next_game_action().await {
                    println!("Recieved action");
                    dbg!(&action);
                }
            }
        }
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
