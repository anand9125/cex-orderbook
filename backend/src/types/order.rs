pub use serde::{Serialize,Deserialize};
use uuid::Uuid;

use crate::Order;

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
pub enum OrderBookMessage{
    Order(Order)  //Enum Variants With Data = Structs Inside an Enum
}