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
            let join_code = interface::start_lan_host();
            println!("JOIN CODE:\n{}", join_code);

            loop {
                if interface::get_connection_status_int().is_connected() {
                    println!("Connected, so I'm breaking!!");
                    break;
                }
                sleep(Duration::from_millis(50));
            }

            println!("Hello");

            sleep(Duration::from_secs(1));

            interface::send_game_action(interface::GameAction::Surrender, |_| {
                println!("Got resp!! (HELL YES!!)");
            });

            sleep(Duration::from_secs(1));

            interface::send_game_action(interface::GameAction::Surrender, |_| {
                println!("Got resp!! (HELL YES!!)");
            })
        }
        "join" => {
            interface::start_lan_client();

            interface::connect_to_host_loop(&args[2], "CLIENT");

            loop {
                if let Some(action) = get_next_game_action() {
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
