
use actix_web::{App, HttpServer, web};
use db::Db;
pub mod auth;
pub use auth::*;
pub mod models;
pub use models::*;
pub mod types;
use tokio::sync::mpsc;
use types::*;
pub mod engine;
pub use engine::*;

use crate::state::AppState;
pub mod state;



#[actix_web::main]
async fn main(){
    let (book_tx, book_rx) = mpsc::channel::<OrderBookMessage>(1000);
    dotenvy::dotenv().ok();
    let db = Db::new().await.expect("fsafsafd");
    let _ = HttpServer::new(move ||{
        App::new()
            .app_data(web::Data::new(AppState{book_tx:book_tx.clone(),db:db.clone()}))
            .service(web::resource("/signin").route(web::post().to(create_user)))
            .service(web::resource("/signin").route(web::post().to(signin)))
         //   .service(web::resource("/order").route(web::post().to(place_order)))
    })
    .bind("0.0.0.0:3000")
    .unwrap()
    .run()
    .await;



    // std::thread::spawn(move || {
    //     let mini_runtime = tokio::runtime::Builder::new_current_thread()
    //         .thread_name("order book thread")
    //         .enable_all()
    //         .build()
    //         .expect("Failed to create tokio runtime on block thread");
    //     mini_runtime.block_on(async move {
    //         loop {
                
                
    //         }
    //     })

    // })
}