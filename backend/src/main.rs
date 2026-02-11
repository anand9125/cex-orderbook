use std::sync::Arc;

use actix_web::{App, HttpServer, web};
use db::Db;
pub mod auth;
pub use auth::*;
pub mod models;
pub use models::*;
pub mod types;
use std::sync::mpsc;
use types::*;
pub mod engine;
pub use engine::*;

use crate::state::AppState;
pub mod state;


#[actix_web::main]
async fn main() {
    let (book_tx, book_rx) = mpsc::sync_channel::<OrderBookMessage>(1000);

    let ring_buffer = Arc::new(RingBuffer::<Event>::new(256));
    let engine_ring = Arc::clone(&ring_buffer);

    dotenvy::dotenv().ok();
    let db = Db::new().await.expect("db init failed");

    std::thread::Builder::new()
        .name("matching-engine".to_string())
        .spawn(move || {
            println!("[ENGINE] Matching engine thread started");

            let mut engine = MatchingEngine::new(engine_ring);
            engine.run(book_rx);

            println!("[ENGINE] Matching engine stopped");
        })
        .expect("failed to spawn matching engine");

    println!("[MAIN] Matching engine spawned");

    // THEN START HTTP SERVER
    let _ = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(AppState {
                book_tx: book_tx.clone(),
                db: db.clone(),
            }))
            .service(web::resource("/signup").route(web::post().to(create_user)))
            .service(web::resource("/signin").route(web::post().to(signin)))
            .service(web::resource("/place_order").route(web::post().to(place_order)))
            .service(web::resource("/cancel_order").route(web::post().to(cancel_order)))
    })
    .bind("0.0.0.0:3000")
    .unwrap()
    .run()
    .await;
}
