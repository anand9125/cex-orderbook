pub mod models;
pub use models::*;
pub mod state;
pub use state::*;
pub mod types;
use tokio::net::TcpListener;
pub use types::*;
pub mod auth;
pub use auth::*;
pub mod engine;
pub use engine::*;
use std::sync::mpsc;

use axum::{Router, routing::post};
use db::Db;
use std::sync::Arc;

#[tokio::main]
async fn main(){
    dotenvy::dotenv().ok();
    let db = Db::new().await.expect("db init needed");
    let (book_tx, book_rx) = mpsc::sync_channel::<OrderBookMessage>(1000);


    let app_state = Arc::new(AppState {
        book_tx,
        db,
    });
    let app = Router::new()
        .route("/signup", post(create_user))
        .route("/signin", post(signin))  
        .route("/place_order", post(place_order))
        .route("/cancel", post(cancel_order))
        .with_state(app_state);  

    let listener = TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("failed to bind");

    axum::serve(listener, app)
        .await
        .expect("server failed");
}