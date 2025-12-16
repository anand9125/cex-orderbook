use db::Db;
use std::sync::mpsc;

use crate::types::OrderBookMessage;

pub struct AppState{
    pub book_tx : mpsc::SyncSender<OrderBookMessage>,
    pub db: Db
}