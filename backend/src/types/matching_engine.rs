use uuid::Uuid;

use crate::{Fill, OrderId, Price, Quantity, UserId, types::Side};

#[derive(Clone)]
pub enum Event {
    OrderPlaced {
        order_id : OrderId,
        user_id : UserId,
        side : Side,
        price : Price,
        quantity : Quantity,
        timestamp : u128 
    },
    Fill(Fill),
    OrderCancelled {
        order_id : OrderId,
        user_id : UserId,
        timestamp : u128
    },
    OrderRejected {
        order_id : OrderId,
        user_id : UserId,
        reason : String,
        timestamp : u128
    }
}