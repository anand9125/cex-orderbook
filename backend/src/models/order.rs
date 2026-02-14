use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use std::sync::Arc;
use tokio::sync::oneshot;
use rust_decimal::Decimal;
use rust_decimal::prelude::FromPrimitive; 
use rust_decimal_macros::dec;

use crate::{AppState, CanceledOrderRequest, LimitOrder, MarketOrder, Order, OrderBookMessage, OrderRequest, OrderResponse, OrderType, Response};

pub async fn place_order(
    State(state): State<Arc<AppState>>,
    Json(req): Json<OrderRequest>,
) -> (StatusCode, Json<Response>) {

    let (tx, rx) = oneshot::channel::<Result<OrderResponse, String>>();

    let quantity = match Decimal::from_f64(req.quantity) {
        Some(q) if q > dec!(0) => q,
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(Response {
                    message: String::new(),
                    error: "Invalid quantity".to_string(),
                }),
            );
        }
    };

    let leverage = Decimal::from_u64(req.leverage).unwrap_or(dec!(1));

    let order = match req.type_ {
        OrderType::Limit => {
            let price_f64 = match req.price {
                Some(p) => p,
                None => {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(Response {
                            message: String::new(),
                            error: "Price is required for limit orders".to_string(),
                        }),
                    );
                }
            };

            let price = match Decimal::from_f64(price_f64) {
                Some(p) if p > dec!(0) => p,
                _ => {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(Response {
                            message: String::new(),
                            error: "Invalid price".to_string(),
                        }),
                    );
                }
            };

            Order::limit_order(LimitOrder {
                user_id: req.user_id,
                side: req.side,
                price,
                quantity,
                leverage,
            })
        }

        OrderType::Market => {
            if req.price.is_some() {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(Response {
                        message: String::new(),
                        error: "Market order must not include price".to_string(),
                    }),
                );
            }

            Order::market_order(MarketOrder {
                user_id: req.user_id,
                side: req.side,
                quantity,
                leverage,
            })
        }
    };

    if state.book_tx.send(OrderBookMessage::PlaceOrder {
        order,
        priority: crate::types::Priority::Normal,
        responder: Some(tx),
    }).is_err()
    {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(Response {
                message: String::new(),
                error: "Engine unavailable".to_string(),
            }),
        );
    }

    match rx.await {
        Ok(Ok(OrderResponse::PlacedOrder {
            order_id,
            status,
            filled,
            remaining,
        })) => (
            StatusCode::OK,
            Json(Response {
                message: format!(
                    "order processed: filled {}, status {}, remaining {}, {}",
                    filled, status, remaining, order_id
                ),
                error: String::new(),
            }),
        ),

        _ => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(Response {
                message: String::new(),
                error: "Engine response dropped".to_string(),
            }),
        ),
    }
}


pub async fn cancel_order(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CanceledOrderRequest>,
) -> (StatusCode, Json<Response>) {

    let (tx, rx) = oneshot::channel::<Result<OrderResponse, String>>();

    if state.book_tx
        .send(OrderBookMessage::CancelOrder {
            user_id: req.user_id,
            order_id: req.order_id,
            responder: Some(tx),
        })
        .is_err()
    {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(Response {
                message: String::new(),
                error: "Engine unavailable".to_string(),
            }),
        );
    }
    match rx.await {
        Ok(Ok(OrderResponse::CanceledOrder {
            order_id,
            user_id,
            status,
            message,
        })) => (
            StatusCode::OK,
            Json(Response {
                message: format!(
                    "Order cancelled successfully: order_id {}, user_id {}, status {}, message {}",
                    order_id, user_id, status, message
                ),
                error: String::new(),
            }),
        ),

        Ok(Err(err)) => (
            StatusCode::BAD_REQUEST,
            Json(Response {
                message: String::new(),
                error: err,
            }),
        ),
        _ => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(Response {
                message: String::new(),
                error: "Engine response dropped".to_string(),
            }),
        ),
    }
}