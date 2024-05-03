use std::collections::HashMap;

use lazy_static::lazy_static;
use tokio::sync::Mutex;

use super::P2pPacket;

lazy_static! {
    static ref TRANSACTION_TABLE: Mutex<HashMap<u16, Option<P2pPacket>>> =
        Mutex::new(HashMap::new());
}

/// Queue for outgoing packets. Follows First in First out principle.
/// Each item in the queue is a tuple of two items: The outgoing packet, and a closure that runs when
/// the outgoing packet has gotten a response.
/// If the packet isn't meant to get a response, set the closure to `None`.
static mut OUTGOING_QUEUE: Vec<(P2pPacket, Option<fn(anyhow::Result<()>)>, u16)> = vec![];

pub async fn push_outgoing_queue(data: P2pPacket, closure: Option<fn(anyhow::Result<()>)>) -> u16 {
    let transaction_id = match &data {
        P2pPacket::Request(req) => req.transaction_id,
        P2pPacket::Response(resp) => resp.transaction_id,
    };
    unsafe {
        OUTGOING_QUEUE.push((data, closure, transaction_id));

        TRANSACTION_TABLE.lock().await.insert(transaction_id, None);
    }
    transaction_id
}

/// Pops and returns the next item in the outgoing network queue.
pub fn pop_outgoing_queue() -> Option<(P2pPacket, Option<fn(anyhow::Result<()>)>, u16)> {
    unsafe { OUTGOING_QUEUE.pop() }
}

pub fn get_outgoing_queue_len() -> usize {
    unsafe { OUTGOING_QUEUE.len() }
}

pub async fn set_response(transaction_id: u16, response: Option<P2pPacket>) {
    TRANSACTION_TABLE
        .lock()
        .await
        .insert(transaction_id, response);
}

pub async fn new_transaction_id() -> u16 {
    let mut transaction_id;
    loop {
        transaction_id = rand::random::<u16>();
        if TRANSACTION_TABLE
            .lock()
            .await
            .get(&transaction_id)
            .is_none()
        {
            break;
        }
    }
    transaction_id
}

pub async fn check_transaction_id(transaction_id: u16) -> bool {
    TRANSACTION_TABLE
        .lock()
        .await
        .get(&transaction_id)
        .is_some()
}

/// Wait for the transaction ID to get a response
pub async fn wait_for_response(transaction_id: u16) -> P2pPacket {
    loop {
        let response = TRANSACTION_TABLE
            .lock()
            .await
            .clone()
            .get(&transaction_id)
            .unwrap()
            .clone();

        if let Some(resp) = response {
            TRANSACTION_TABLE.lock().await.remove(&transaction_id);
            return resp.clone();
        }
    }
}
