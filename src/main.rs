use std::{
    env,
    net::{IpAddr, SocketAddr},
    sync::Arc,
    thread::sleep,
    time::Duration,
};

use net::{
    net_loop::push_outgoing_queue,
    net_utils::{get_available_port, get_local_ip, hex_decode_ip, hex_encode_ip},
    p2p::P2pRequestPacket,
    set_other_addr, CONNECT_SESSION_ID,
};

use crate::net::{
    get_session_id,
    net_loop::{client_network_loop, host_network_loop},
    p2p::{P2pRequest, P2pResponse},
    set_connection_status, set_join_code, set_session_id, ConnectionStatus,
};

pub mod net;

#[tokio::main]
async fn main() {
    let port = get_available_port().await.unwrap();
    let socket = tokio::net::UdpSocket::bind(("0.0.0.0", port))
        .await
        .unwrap();
    let args = env::args().collect::<Vec<String>>();
    println!("WELCOME!");

    match args[1].to_lowercase().as_str() {
        "host" => {
            let local_ip = get_local_ip().unwrap();

            let encoded_ip = hex_encode_ip(SocketAddr::new(IpAddr::V4(local_ip), port)).unwrap();
            set_join_code(&encoded_ip);

            println!("Encoded SocketAddr:\n{}", encoded_ip);

            tokio::spawn(host_network_loop(socket));
        }
        "join" => {
            set_join_code(&args[2]);
            let host_addr = hex_decode_ip(&args[2]).unwrap();
            set_other_addr(Some(host_addr));

            tokio::spawn(client_network_loop(socket, 10));

            let join_request = P2pRequest::new(
                CONNECT_SESSION_ID,
                P2pRequestPacket::Connect {
                    join_code: args[2].clone(),
                    username: "Client".to_string(),
                },
            );

            push_outgoing_queue(
                join_request,
                Some(Arc::new(move |resp: anyhow::Result<P2pResponse>| {
                    if let Ok(resp) = &resp {
                        println!("Joined a game!");
                        set_connection_status(ConnectionStatus::Connected { ping: 0 });
                        set_session_id(resp.session_id);
                    }
                    dbg!(&resp);
                })),
            );
            sleep(Duration::from_secs(10));

            if let Some(session_id) = get_session_id() {
                let resync_request = P2pRequest::new(session_id, P2pRequestPacket::Resync);

                push_outgoing_queue(
                    resync_request,
                    Some(Arc::new(move |resp: anyhow::Result<P2pResponse>| {
                        dbg!(&resp);
                    })),
                );
            } else {
                println!("No session ID")
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
