use std::{collections::{BTreeMap, HashMap, VecDeque}};

use rust_decimal::Decimal;
use uuid::Uuid;

use crate::{types::Order};
pub type Price = Decimal;
pub type OrderId = Uuid;


pub struct OrderBook {
    pub bids : BTreeMap<Price,VecDeque<Order>>,
    pub asks : BTreeMap<Price,VecDeque<Order>>,
    pub orders : HashMap<OrderId,Order>,
    pub user_order : HashMap<OrderId,Order> , //cancel order

    
}