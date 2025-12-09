use std::{collections::{BTreeMap, HashMap, VecDeque}, fmt::format};

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use tokio::sync::oneshot;
use uuid::Uuid;

use crate::types::{ OrderType, Side};
pub type Price = Decimal;
pub type OrderId = Uuid;
pub type UserId = Uuid;
pub type Quantity = Decimal;

pub struct LimitOrder{
    pub user_id :Uuid,
    pub side : Side,
    pub price : Price,
    pub quantity : Quantity,
    pub leverage : Decimal,
    pub responder : Option<oneshot::Sender<OrderResponse>>
}
pub struct MarketOrder{
    pub user_id : Uuid,
    pub side : Side,
    pub quantity : Quantity,
    pub leverage : Decimal,
    pub responder : Option<oneshot::Sender<OrderResponse>>
}

pub struct PriceLevel{
    pub price : Price,
    pub orders : VecDeque<OrderId>,
    pub total_qty : Quantity
}
pub struct Order {
    pub order_id : Uuid,
    pub user_id : Uuid,
    pub price : Option<Price>,
    pub leverage : Decimal,
    pub side : Side,
    pub order_type : OrderType,
    pub quantity : Quantity,
    pub filled : Quantity,
    pub responder : Option<oneshot::Sender<OrderResponse>>  
}

#[derive(Clone)]
pub struct OrderResponse{
    pub status : String,
    pub filled : Decimal,
    pub remaining : Decimal
}


impl Order {
    pub fn limit_order(limit_order:LimitOrder)->Self{
        Self{
            order_id : Uuid::new_v4(),
            user_id : limit_order.user_id,
            side : limit_order.side,
            price : Some(limit_order.price),
            quantity : limit_order.quantity,
            leverage : limit_order.leverage,
            order_type : OrderType::Limit,
            filled : dec!(0),
            responder : limit_order.responder
        }
    }
    pub fn market_order(market_order : MarketOrder)->Self{
        Self{
            order_id : Uuid::new_v4(),
            user_id : market_order.user_id,
            price : None,
            leverage : market_order.leverage,
            side : market_order.side,
            order_type : OrderType::Market,
            quantity : market_order.quantity,
            filled : dec!(0),
            responder : market_order.responder
        }
    } 
    pub fn remaining(&self)->Quantity{
        self.quantity-self.filled
    }
    pub fn validate(&self)->Result<(),String>{
        if self.quantity <= dec!(0){
            return Err(format!("quantity must be greater then 0 got {}",self.quantity));
        }
        if self.leverage <= dec!(0){
            return Err(format!("leverage must be greater then 0 got {}",self.leverage));
        }
        match self.order_type {
            OrderType::Limit =>{
                    if self.price.is_none() || self.price.unwrap() <= dec!(0) {
                    return Err("limit order must have valid price".into());
                }
            }
            OrderType::Market =>{
                 if self.price.is_some() {
                    return Err("market order should not have price".into());
                }
            } 
        }
        Ok(())
    }
}

pub struct OrderBook {
   pub bids : BTreeMap<Price,PriceLevel>,
   pub asks : BTreeMap<Price,PriceLevel>,
   pub orders : HashMap<OrderId,Order>,
   pub user_orders : HashMap<UserId,Vec<OrderId>>,
   pub best_bid : Option<Price>,
   pub best_ask :Option<Price>,
   pub fill_seq:u64  //sequence numners for fills
}

pub struct Fill{


}
impl OrderBook{
    pub fn new()->Self{
        Self{
            bids : BTreeMap::new(),
            asks : BTreeMap::new(),
            orders : HashMap::new(),
            user_orders : HashMap::new(),
            best_bid : None,
            best_ask : None,
            fill_seq : 0
        }
    }
    pub fn get_orderbook_side(&mut self,side:Side)->&mut  BTreeMap<Price,PriceLevel>{
        match side {
            Side::Buy => &mut self.bids,
            Side::Sell => &mut self.asks
        }
    }
    pub fn get_opposite_side(&mut self,side:Side)->&mut BTreeMap<Price,PriceLevel>{
        match side {
            Side::Buy => &mut self.asks,
            Side::Sell => &mut self.bids
        }
    }
    pub fn update_best_prices(&mut self){
        self.best_ask = self.asks.keys().next().cloned();
        self.best_bid = self.bids.keys().next_back().cloned();
    }
    pub fn insert_order (&mut self,order: Order){
        if order.order_type != OrderType::Limit{
            return;
        }
        let order_id = order.order_id;
        let user_id = order.user_id;
        let price = order.price.unwrap();
        let amount = order.quantity;
        let side = order.side;


        let book = self.get_orderbook_side(side);
        
        let level = book.entry(price).or_insert_with(|| PriceLevel{
            price,
            orders : VecDeque::new(),
            total_qty : dec!(0)
        });
        level.orders.push_back(order_id);
        level.total_qty += amount;

        self.user_orders
            .entry(user_id)
            .or_insert_with(Vec::new)
            .push(order_id);

        self.orders.insert(order_id,order);
        self.update_best_prices();

    }

    pub fn cancel_order(&mut self, order_id : &OrderId, user_id :&UserId)->Result<Order,String>{
        let order = self.orders.get(order_id).ok_or("order is not found").unwrap();

        if &order.user_id != user_id{
            return Err("unauthorized : not owner order".into());
        }

        let order = self.orders.remove(order_id).unwrap();
        let price = order.price.unwrap();
        let side = order.side;

        let book = self.get_orderbook_side(side);

        if let Some(level) =  book.get_mut(&price){
            level.orders.retain(|id|id!=order_id); //retain keep the element where clouser return true
            level.total_qty -= order.quantity;

            if level.orders.is_empty() {
                book.remove(&price);
            }
        }

        if let Some(user_orders) = self.user_orders.get_mut(user_id){
            user_orders.retain(|id|id!=order_id);
        }
        self.update_best_prices();

        Ok(order) 
    }

    pub fn match_order(&mut self,mut taker : Order) ->(Vec<Fill>,Option<Order>){
        let mut fills: Vec<Fill> = Vec::new();
        let opposite_side = self.get_opposite_side(taker.side);

        loop{

        }


    }

    
}

