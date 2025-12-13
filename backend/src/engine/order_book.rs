use std::{collections::{BTreeMap, HashMap, VecDeque}, time::{SystemTime, UNIX_EPOCH}};

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
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
}
pub struct MarketOrder{
    pub user_id : Uuid,
    pub side : Side,
    pub quantity : Quantity,
    pub leverage : Decimal,
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
        }
    } 
    pub fn remaining(&self)->Quantity{
        self.quantity-self.filled
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
#[derive(Clone)]
pub struct Fill{
    pub seq_no : u64,
    pub maker_order_id:OrderId,
    pub taker_order_id:OrderId,
    pub maker_user_id:OrderId,
    pub taker_user_id :OrderId,
    pub price : Price,
    pub quantity : Quantity,
    pub taker_leverage : Decimal,
    pub maker_leverage : Decimal,
    pub maker_side : Side,
    pub taker_side : Side,
    pub timestamp_: u128

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

         //if always expects a boolean expression but if let is not a boolean expression
         //if let is syntactic sugar for a match, not a normal if.
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
   

    pub fn match_order(&mut self, mut taker:  Order) -> (Vec<Fill>, Option<Order>) {
        let mut fills: Vec<Fill> = Vec::new();

        loop {
            if taker.remaining() <= dec!(0) {
                break;
            }

            let best_price = {
                let side = self.get_opposite_side(taker.side);
                match taker.side {
                    Side::Buy => side.keys().next().cloned(),
                    Side::Sell => side.keys().next_back().cloned(),
                }
            };

            let best_price = match best_price {
                Some(p) => p,
                None => break,
            };

            if let OrderType::Limit = taker.order_type {
                let taker_price = taker.price.unwrap();
                let crosses = match taker.side {
                    Side::Buy => taker_price >= best_price,
                    Side::Sell => taker_price <= best_price,
                };
                if !crosses {
                    break;
                }
            }

            let maker_ids: Vec<Uuid> = {
                let side = self.get_opposite_side(taker.side);
                if let Some(level) = side.get(&best_price) {
                    level.orders.iter().cloned().collect()
                } else {
                    break;
                }
            };

            let mut orders_to_remove = Vec::new();
            let mut total_qty_decrease = dec!(0);

            for maker_id in maker_ids {
                if taker.remaining() <= dec!(0) {
                    break;
                }

                let maker = match self.orders.get_mut(&maker_id) {
                    Some(m) => m,
                    None => continue,
                };

                let qty = maker.remaining().min(taker.remaining());

                self.fill_seq += 1;
                fills.push(Fill {
                    seq_no: self.fill_seq,
                    maker_order_id: maker.order_id,
                    taker_order_id: taker.order_id,
                    maker_user_id: maker.user_id,
                    taker_user_id: taker.user_id,
                    price: best_price,
                    quantity: qty,
                    maker_leverage: maker.leverage,
                    taker_leverage: taker.leverage,
                    maker_side: maker.side,
                    taker_side: taker.side,
                    timestamp_: now_nanos(),
                });

                maker.filled += qty;
                taker.filled += qty;
                total_qty_decrease += qty;

                if maker.remaining() <= dec!(0) {
                    orders_to_remove.push(maker_id);
                }
            }

            {
                let side = self.get_opposite_side(taker.side);
                if let Some(level) = side.get_mut(&best_price) {
                    level.total_qty -= total_qty_decrease;
                    level.orders.retain(|id| !orders_to_remove.contains(id));
                }
            }

            for id in orders_to_remove {
                if let Some(order) = self.orders.remove(&id) {
                    if let Some(list) = self.user_orders.get_mut(&order.user_id) {
                        list.retain(|x| x != &id);
                    }
                }
            }
        }

        self.update_best_prices();

        let remaining = match taker.order_type {
            OrderType::Limit => {
                if taker.remaining() > dec!(0) {
                    Some(taker)
                } else {
                    None
                }
            }
            OrderType::Market => None, // Market orders never sit in book
        };

        (fills, remaining)
    }
    
    
}
pub fn now_nanos() -> u128 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
}


