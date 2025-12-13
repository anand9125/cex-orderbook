pub use serde::{Serialize,Deserialize};
use tokio::sync::oneshot;
use uuid::Uuid;

use crate::{Order, OrderId, Quantity, UserId};

#[derive(Deserialize, Serialize)]
pub struct OrderRequest {
    #[serde(rename = "type")]
    pub type_: OrderType,
    pub user_id : Uuid,
    pub side: Side,
    pub quantity: f64,
    pub price: Option<f64>,
    pub leverage: u32,
}

#[derive(Deserialize, Serialize,PartialEq)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]

pub enum Priority {
    Critical = 0,
    High = 1,
    Normal = 2, 
    Low = 3
}
pub enum OrderStatus {
    Accepted,
    FullyFilled,
    PartiallyFilled,
    Rejected,
    Cancelled,
}
pub struct OrderResponse{
    pub order_id : OrderId,
    pub status : OrderStatus,
    pub filled : Quantity,
    pub remaining : Quantity
}
pub enum OrderBookMessage{
    PlaceOrder{
        order: Order,
        priority : Priority,
        responder : oneshot::Sender<Result<OrderResponse,String>>
    },
    CancelOrder{
        order_id : OrderId,
        user_id : UserId,
        priority : Priority,
        responder : oneshot::Sender<Result<OrderResponse,String>>
    }
}
