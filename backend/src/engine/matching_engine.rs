use std::sync::{Arc, mpsc}; 
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use tokio::sync::{oneshot};

use crate::{Order, OrderBook, OrderId, Price, Quantity, RingBuffer, UserId, now_nanos, types::{Event, OrderBookMessage, OrderResponse, OrderStatus, OrderType}};

pub struct MatchingEngine{
   event_buffer : Arc<RingBuffer<Event>>,
   order_book :OrderBook
}

impl MatchingEngine{
   pub fn new(
      event: Arc<RingBuffer<Event>>
   )->Self{
      Self {
         event_buffer: event ,
         order_book:OrderBook::new()
      }
   }

   pub fn run(
      &mut self,
      mut cmd_rx : mpsc::Receiver<OrderBookMessage>
   ){
      let mut batch:Vec<OrderBookMessage> = Vec::with_capacity(256);
      loop{
         //blocking revc wait for first command
            match cmd_rx.recv() {
                Ok(cmd) => batch.push(cmd),
                Err(_) => {
                    println!(" [ENGINE] Command channel closed");
                    break;
                }
            }
         //drain more cmd (non blocking)
         while batch.len()<256{
            match cmd_rx.try_recv(){ //try_recv return immediately if no messge
               Ok(cmd)=>batch.push(cmd),
               Err(_) =>break
            }
         }
         //sort by key => reorder the message inside the batch based on the value return by cmd.priorty message with lower key value come first
         //keys are 0,1,2,3 ordering rule sorts assending so sorting the batch order become critical high normal low
         batch.sort_by_key(|cmd| cmd.priority());

         let batch_size = batch.len();
         self.process_batch(&mut batch);
         
         //clear batch
         batch.clear();

      }
   }

   fn process_batch(&mut self, batch: &mut Vec<OrderBookMessage>) {
      for cmd in batch.drain(..) {
         match cmd {
               OrderBookMessage::PlaceOrder {
                  order,
                  mut priority,
                  mut responder,
               } => {
                  self.handle_place_order(order, &mut responder);
               }

               OrderBookMessage::CancelOrder {
                  order_id,
                  user_id,
                  responder,
               } => {
                  self.handle_cancel_order(order_id, user_id, responder);
               }

               OrderBookMessage::UpdateMarkPrice { price } => {
                  self.handle_update_mark_price(price);
               }
         }
      }
   }

   fn handle_place_order(
      &mut self,
      order:  Order,
      responder: &mut Option<oneshot::Sender<Result<OrderResponse, String>>,>
   ) {
      if let Err(e) = self.validate_order(&order) {
          if let Some(tx) = responder.take() {
            let _ = tx.send(Err(e));
         }
         self.emit_event(Event::OrderRejected { 
            order_id:order.order_id,
            user_id :order.user_id,
            reason : ("problem while validatin".to_string()),
            timestamp : now_nanos()
        });
        return;
      }
      let order_quantity = order.quantity;
      let order_id = order.order_id;
      let order_type = order.order_type;
      let (fills,remaining_order) = self.order_book.match_order(order);

      for fill in fills.iter() {
         self.emit_event(Event::Fill(fill.clone()));
      }

      if let Some(rem_order) = remaining_order {
         let order_id = rem_order.order_id;
         let user_id  = rem_order.user_id;
         let side     = rem_order.side;
         let price    = rem_order.price.unwrap();
         let quantity = rem_order.quantity;
         let filled = rem_order.filled;

         self.order_book.insert_order(rem_order);

         self.emit_event(Event::OrderPlaced {
            order_id,
            user_id,
            side,
            price,
            quantity:quantity.checked_sub(filled).unwrap(),
            timestamp: now_nanos(),
         });
      }
      //Prepare the send resposne for api layer
      let original_qty = order_quantity;
      let total_filled:Quantity = fills.iter().map(|f|f.quantity).sum();
      let remaining = original_qty.checked_sub(total_filled).ok_or("err").unwrap();

      let status = match order_type {
         OrderType::Market => {
            if total_filled == dec!(0) {
                  OrderStatus::Rejected   
            } else if remaining == dec!(0) {
                  OrderStatus::FullyFilled
            } else {
                  OrderStatus::PartiallyFilled
            }
         }
         OrderType::Limit => {
            if total_filled == dec!(0) {
                  OrderStatus::New
            } else if remaining == dec!(0) {
                  OrderStatus::FullyFilled
            } else {
                  OrderStatus::PartiallyFilled
            }
         }
      };
      
      if let Some(tx) = responder.take(){
         let _ = tx.send(Ok(OrderResponse{
            order_id,
            status,
            filled:total_filled,
            remaining
         }));
      }
   }

   fn handle_cancel_order(&mut self,order_id :  OrderId , user_id: UserId , responder:oneshot::Sender<Result<OrderResponse,String>>){

   }
 
   fn handle_update_mark_price(&mut self , price: Price){

   }
   fn emit_event(&self,event:Event){
      self.event_buffer.push(event);
   }
 
   
   fn validate_order(&self,order:&Order)->Result<(),String>{

      if order.order_type != OrderType::Limit && order.order_type != OrderType::Market{
         return Err("invalid order_type".to_string());
      }
      if order.quantity <= Decimal::ZERO {
         return Err("quantity should be greater then the zero".to_string());
      }
      if order.leverage < dec!(1)|| order.leverage > dec!(125) {
            return Err("Invalid leverage (1-125x)".to_string());
      }
      Ok(())
   }

}
