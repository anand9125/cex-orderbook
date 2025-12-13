use std::{sync::Arc, time::{SystemTime, UNIX_EPOCH}};

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use tokio::sync::{mpsc, oneshot};

use crate::{Order, OrderBook, OrderId, Price, RingBuffer, UserId, now_nanos, types::{Event, OrderBookMessage, OrderResponse, OrderType, Priority}};

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

   pub async fn run(
      &mut self,
      mut cmd_rx : mpsc::Receiver<OrderBookMessage>
   ){
      let mut batch:Vec<OrderBookMessage> = Vec::with_capacity(256);
      loop{
         //blocking revc wait for first command
         match cmd_rx.recv().await{
            Some(cmd)=>batch.push(cmd),
            None=>{
               println!("command channle closed ");
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

   fn process_batch(&mut self, batch:&mut Vec<OrderBookMessage>){
      for cmd in batch {
         match cmd {
            OrderBookMessage::PlaceOrder { order, priority, responder }=>{
               self.handle_place_order(order,priority,responder);

            }
            OrderBookMessage::CancelOrder { order_id, user_id, responder }=>{
               self.handle_cancel_order(order_id,user_id,responder)

            }
            OrderBookMessage::UpdateMarkPrice { price }=>{
               self.handle_update_mark_price(price)

            }  
         }
      }
   }

   fn handle_place_order(
      &mut self,
      order:  Order,
      priority: &mut Priority,
      responder: &mut Option<oneshot::Sender<Result<OrderResponse, String>>,>
   ) {
      if let Err(e) = self.validate_order(order) {
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
      let (fills,remaining_order) = self.order_book.match_order();


      // success path must also consume responder
   }

   fn handle_cancel_order(&mut self,order_id : &mut OrderId , user_id:&mut UserId , responder:&mut oneshot::Sender<Result<OrderResponse,String>>){

   }
   fn handle_update_mark_price(&mut self , price:&mut Price){

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
