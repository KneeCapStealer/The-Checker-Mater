use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
};

use lazy_static::lazy_static;
use tokio::sync::Mutex;

use crate::game::GameAction;

use super::{P2pPacket, P2pResponse};

lazy_static! {
    static ref TRANSACTION_TABLE: Mutex<
        HashMap<
            u16,
            (
                Option<P2pPacket>,
                Option<Arc<Mutex<dyn FnMut(P2pResponse) + Send + Sync>>>
            ),
        >,
    > = Mutex::const_new(HashMap::new());
}

lazy_static! {
    /// Queue for outgoing packets. Follows First in First out principle.
    /// Each item in the queue is a tuple of two items: The outgoing packet, and a closure that runs when
    /// the outgoing packet has gotten a response.
    /// If the packet isn't meant to get a response, set the closure to `None`.
    static ref OUTGOING_QUEUE: Mutex<VecDeque<(P2pPacket, u16)>> =
        Mutex::const_new(VecDeque::new());
}

lazy_static! {
    /// A list which holds all `GameActions` send from the other user.
    static ref INCOMING_ACTIONS: Mutex<VecDeque<GameAction>> =
        Mutex::const_new(VecDeque::new());
}

pub async fn push_outgoing_queue(
    data: P2pPacket,
    closure: Option<Arc<Mutex<(dyn FnMut(P2pResponse) + Send + Sync + 'static)>>>,
) -> u16 {
    let transaction_id = match &data {
        P2pPacket::Request(req) => req.transaction_id,
        P2pPacket::Response(resp) => resp.transaction_id,
    };
    OUTGOING_QUEUE
        .lock()
        .await
        .push_back((data, transaction_id));

    TRANSACTION_TABLE
        .lock()
        .await
        .insert(transaction_id, (None, closure));
    transaction_id
}

/// Pops and returns the next item in the outgoing network queue.
pub async fn pop_outgoing_queue() -> Option<(P2pPacket, u16)> {
    OUTGOING_QUEUE.lock().await.pop_front()
}

pub async fn get_outgoing_queue_len() -> usize {
    OUTGOING_QUEUE.lock().await.len()
}

/// Sets the response to a request inside the transaction table.
/// If the transaction has a closure, this will run that closure, and then remove the request and
/// its response.
pub async fn set_response(transaction_id: u16, response: Option<P2pPacket>) {
    let table = &mut TRANSACTION_TABLE.lock().await;
    if let Some((_, closure)) = table.get(&transaction_id) {
        if let Some(closure) = closure {
            if let Some(P2pPacket::Response(resp)) = response.clone() {
                closure.lock().await(resp);
            }
            table.remove(&transaction_id);
        } else {
            table.insert(transaction_id, (response, None));
        };
    }
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

pub async fn check_for_response(transaction_id: u16) -> Option<P2pPacket> {
    let response = TRANSACTION_TABLE
        .lock()
        .await
        .clone()
        .get(&transaction_id)
        .unwrap_or(&(None, None))
        .clone();

    if response.0.is_some() {
        TRANSACTION_TABLE.lock().await.remove(&transaction_id);
    }
    response.0
}

/// Wait for the transaction ID to get a response
pub async fn wait_for_response(transaction_id: u16) -> P2pPacket {
    loop {
        let response = TRANSACTION_TABLE
            .lock()
            .await
            .clone()
            .get(&transaction_id)
            .unwrap_or(&(None, None))
            .clone();

        if let Some(resp) = response.0 {
            TRANSACTION_TABLE.lock().await.remove(&transaction_id);
            return resp.clone();
        }
    }
}

pub async fn get_transaction_table() -> HashMap<
    u16,
    (
        Option<P2pPacket>,
        Option<Arc<Mutex<dyn FnMut(P2pResponse) + Send + Sync>>>,
    ),
> {
    TRANSACTION_TABLE.lock().await.clone()
}

pub async fn push_incoming_gameaction(action: GameAction) {
    INCOMING_ACTIONS.lock().await.push_back(action);
}
pub async fn pop_incoming_gameaction() -> Option<GameAction> {
    INCOMING_ACTIONS.lock().await.pop_front()
}
pub async fn get_incoming_gameaction_len() -> usize {
    INCOMING_ACTIONS.lock().await.len()
}
