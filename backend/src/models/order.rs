use actix_web::{ Responder,  http::StatusCode, web::{self, Json}};
use rust_decimal::{Decimal, prelude::{FromPrimitive}};
use rust_decimal_macros::dec;
use tokio::sync::oneshot;

use crate::{LimitOrder, MarketOrder, Order, OrderResponse, state::AppState, types::{ OrderBookMessage, OrderRequest, OrderType, Response}};

pub async fn place_order(body: Json<OrderRequest>,state:web::Data<AppState>) -> impl Responder {
    let order = body.into_inner();
    let (tx, rx) = oneshot::channel::<Result<OrderResponse,String>>();
    let price_in_f64 = order.price.ok_or("err").unwrap();

    let (price, quantity) = match (
        Decimal::from_f64(price_in_f64),
        Decimal::from_f64(order.quantity),
    ){
        (Some(p), Some(a)) => (p, a),
        _ => {
            return (
                Json(Response {
                    message: String::new(),
                    error: format!(
                        "Invalid price or amount: {} / {}",
                        price_in_f64, order.quantity
                    ),
                }),
                StatusCode::BAD_REQUEST,
            );
        }
    };
    let leverage = Decimal::from_u32(order.leverage).unwrap_or(dec!(1));

    let order = match order.type_ {
        OrderType::Limit => {
            Order::limit_order(LimitOrder{
                user_id: order.user_id,
                side: order.side,
                price: price,
                quantity:quantity,
                leverage: leverage,  
            })
        }

        OrderType::Market => {
            Order::market_order(MarketOrder {
                user_id: order.user_id,
                side:order.side,
                quantity: quantity,
                leverage: leverage,
            })
        }
    };

    if let Err(e) = state.book_tx.send(OrderBookMessage::PlaceOrder{
        order,
        priority : crate::types::Priority::Normal,
        responder : Some(tx)
    }){
        return (
            Json(Response{
                message:String::new(),
                error:format!(
                    "error while sending"
                )
            }),
            StatusCode::BAD_REQUEST
        );
    }

    match rx.await.unwrap(){
        Ok(response)=>{
            return (
                Json(Response{
                    message:format!(
                        "order processd : filled {},remaining:{},{}",
                        response.filled,response.remaining,response.order_id
                    ),
                    error:String::new()
                }),
                StatusCode::OK
            );
        }
        Err(val) => {
            return (
                Json(
                    Response{
                        message:String::new(),
                        error:format!("error while recieving")
                    }
                ),
                StatusCode::BAD_REQUEST
            );

        }
    }
}



