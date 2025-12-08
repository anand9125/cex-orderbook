use actix_web::{ Responder, http::StatusCode, web::{self, Json}};
use rust_decimal::{Decimal, prelude::{FromPrimitive}};
use rust_decimal_macros::dec;
use tokio::sync::oneshot;
use uuid::Uuid;

use crate::{state::{ AppState}, types::{ Order, OrderBookMessage, OrderRequest, OrderResponse, Response}};

pub async fn place_order(body: Json<OrderRequest>,state:web::Data<AppState>) -> impl Responder {
    let order = body.into_inner();
    let (tx, rx) = oneshot::channel::<OrderResponse>();

    let (price, amount) = match (
        Decimal::from_f64(order.price),
        Decimal::from_f64(order.amount),
    ){
        (Some(p), Some(a)) => (p, a),
        _ => {
            return (
                Json(Response {
                    message: String::new(),
                    error: format!(
                        "Invalid price or amount: {} / {}",
                        order.price, order.amount
                    ),
                }),
                StatusCode::BAD_REQUEST,
            );
        }
    };
    let leverage = Decimal::from_u32(order.leverage).unwrap_or(dec!(1));

    let order = Order{
        order_id : Uuid::new_v4(),
        user_id : order.user_id,
        price : price,
        amount:amount,
        leverage:leverage,
        side :order.side,
        responder : Some(tx)
    };

    if let Err(e) = state.book_tx.send(OrderBookMessage::Order(order)).await{
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

    match rx.await{
        Ok(response)=>{
            return (
                Json(Response{
                    message:format!(
                        "order processd : filled {},remaining:{},{}",
                        response.filled,response.remaining,response.status
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

