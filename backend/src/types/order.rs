pub use serde::{Serialize,Deserialize};
use tokio::sync::oneshot;
use uuid::Uuid;
use std::fmt;

use crate::{Order, OrderId, Price, Quantity, UserId};



#[derive(Deserialize, Serialize)]
pub struct OrderRequest {
    #[serde(rename = "type")]
    pub type_: OrderType,
    pub user_id : Uuid,
    pub side: Side,
    pub quantity: f64,
    pub price: Option<f64>,
    pub leverage: u64,
}
#[derive(Deserialize,Serialize)]
pub struct CanceledOrderRequest{
    pub user_id : UserId,
    pub order_id : OrderId
}

#[derive(Deserialize, Serialize,PartialEq,Clone,Copy)]
#[serde(rename_all = "lowercase")]
pub enum OrderType {
    Market,
    Limit,
}

#[derive(Deserialize, Serialize,Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    Buy,
    Sell,
}

#[derive(Serialize, Deserialize)]
pub struct Response {
    pub message: String,
    pub error: String,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Critical = 0,
    High     = 1,
    Normal   = 2,
    Low      = 3,
}

pub enum OrderStatus {
    Accepted,
    FullyFilled,
    PartiallyFilled,
    Rejected,
    Cancelled,
    New
}
impl fmt::Display for OrderStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            OrderStatus::Accepted        => "Accepted",
            OrderStatus::FullyFilled     => "FullyFilled",
            OrderStatus::PartiallyFilled => "PartiallyFilled",
            OrderStatus::Rejected        => "Rejected",
            OrderStatus::Cancelled       => "Cancelled",
            OrderStatus::New             => "New",
        };
        write!(f, "{s}")
    }
}

pub enum OrderResponse{
    PlacedOrder{
       order_id : OrderId,
       status : OrderStatus,
       filled : Quantity,
       remaining : Quantity
    },
    CanceledOrder{
        order_id : OrderId,
        user_id : UserId,
        status : OrderStatus,
        message : String
    },
    Message{
        message : String
    }
}
pub enum OrderBookMessage {
    PlaceOrder {
        order: Order,
        priority: Priority,  //configurable
        responder: Option<oneshot::Sender<Result<OrderResponse, String>>>,
    },
    //prioruty for all message is fixed
    CancelOrder {
        order_id: OrderId,
        user_id: UserId,
        responder: Option<oneshot::Sender<Result<OrderResponse, String>>>,
    },
    UpdateMarkPrice {
        price: Price,
    },
}

impl OrderBookMessage {
    pub fn priority(&self) -> Priority {
        match self {
            OrderBookMessage::PlaceOrder { priority, .. } => *priority,
            OrderBookMessage::CancelOrder { .. } => Priority::Critical,
            OrderBookMessage::UpdateMarkPrice { .. } => Priority::Critical,
        }
    }
}
