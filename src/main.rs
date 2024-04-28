use std::{
    env,
    net::{IpAddr, SocketAddr},
    thread::sleep,
    time::Duration,
};

use net::{
    net_utils::{get_available_port, get_local_ip, hex_decode_ip, hex_encode_ip},
    p2p::P2pRequestPacket,
};

use crate::net::p2p::{
    communicate::{recieve_p2p_packet, send_p2p_packet},
    P2pRequest, P2pResponse, P2pResponsePacket, PieceColor,
};

pub mod net;

#[tokio::main]
async fn main() {
    let args = env::args().collect::<Vec<String>>();
    if args[1] == "host" {
        let local_ip = get_local_ip().unwrap();

        let port = get_available_port().await.unwrap();
        let socket = tokio::net::UdpSocket::bind(("0.0.0.0", port))
            .await
            .unwrap();

        let encoded_ip = hex_encode_ip(SocketAddr::new(IpAddr::V4(local_ip), port)).unwrap();

        println!("Encoded SocketAddr:\n{}", encoded_ip);

        // tokio::spawn(async move {
        //     loop {
        //         let (incoming_packet, addr) =
        //             recieve_p2p_packet::<P2pRequest>(&socket).await.unwrap();
        //
        //         let packet = match incoming_packet.packet {
        //             P2pRequestPacket::Ping => P2pResponsePacket::Pong,
        //             P2pRequestPacket::Connect {
        //                 join_code: _,
        //                 username: _,
        //             } => P2pResponsePacket::Connect {
        //                 client_color: PieceColor::White,
        //                 host_username: "Atle".to_string(),
        //             },
        //             _ => P2pResponsePacket::Resync { board: vec![] },
        //         };
        //
        //         let response = P2pResponse::new(incoming_packet.session_id, packet);
        //         send_p2p_packet(&socket, response, addr).await.unwrap();
        //
        //         if let Some(packet) = get_next_network_packet::<P2pRequest>() {
        //             println!("Sending: {:?}", packet);
        //             send_p2p_packet(&socket, packet.clone(), addr)
        //                 .await
        //                 .unwrap();
        //
        //             let (response, _) = recieve_p2p_packet::<P2pResponse>(&socket).await.unwrap();
        //             set_response_packet(response.clone()).await;
        //             println!("Got: {:?}", response);
        //         }
        //         if let Some(packet) = get_next_network_packet::<P2pResponse>() {
        //             send_p2p_packet(&socket, packet.clone(), addr)
        //                 .await
        //                 .unwrap();
        //         }
        //     }
        // });

        // let response = P2pResponse::new(
        //     420,
        //     P2pResponsePacket::Connect {
        //         client_color: PieceColor::White,
        //         host_username: "Atle".to_string(),
        //     },
        // );
    }
    if args[1] == "join" {
        let host_addr = hex_decode_ip(&args[2]).unwrap();

        let port = get_available_port().await.unwrap();

        let socket = tokio::net::UdpSocket::bind(("0.0.0.0", port))
            .await
            .unwrap();

        let join_request = P2pRequest::new(
            69,
            P2pRequestPacket::Connect {
                join_code: args[2].as_bytes().try_into().unwrap(),
                username: "AAAAAAAAAAAAAAAAAAA".to_string(),
            },
        );

        send_p2p_packet(&socket, join_request, host_addr)
            .await
            .unwrap();

        let (connect_resp, _) = recieve_p2p_packet::<P2pResponse>(&socket).await.unwrap();
        dbg!(&connect_resp);

        // tokio::spawn(async move {
        //     loop {
        //         let ping = P2pRequest::new(69, P2pRequestPacket::Ping);
        //         send_p2p_packet(&socket, ping, host_addr).await.unwrap();
        //         //num_pings += 1;
        //         //println!("\r{}", num_pings);
        //
        //         let (incoming_packet, addr) =
        //             recieve_p2p_packet::<P2pRequest>(&socket).await.unwrap();
        //         let packet = match incoming_packet.packet {
        //             P2pRequestPacket::Ping => P2pResponsePacket::Pong,
        //             _ => P2pResponsePacket::Resync { board: vec![] },
        //         };
        //
        //         let response = P2pResponse::new(incoming_packet.session_id, packet);
        //         send_p2p_packet(&socket, response, addr).await.unwrap();
        //
        //         if let Some(packet) = get_next_network_packet::<P2pRequest>() {
        //             println!("Sending: {:?}", packet);
        //             send_p2p_packet(&socket, packet.clone(), host_addr)
        //                 .await
        //                 .unwrap();
        //
        //             let (response, _) = recieve_p2p_packet::<P2pResponse>(&socket).await.unwrap();
        //             set_response_packet(response.clone()).await;
        //             println!("Got: {:?}", response);
        //         }
        //         if let Some(packet) = get_next_network_packet::<P2pResponse>() {
        //             send_p2p_packet(&socket, packet.clone(), host_addr)
        //                 .await
        //                 .unwrap();
        //         }
        //     }
        // });
        //
        // let resync_req = P2pRequest {
        //     session_id: 69,
        //     packet: P2pRequestPacket::Resync,
        // };
        //
        // set_next_network_packet(resync_req).await;
        // dbg!(&get_next_network_packet::<P2pRequest>());
    }
    sleep(Duration::new(60, 0));
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
