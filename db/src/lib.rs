use sqlx::{PgPool, postgres::PgPoolOptions};
use anyhow::{Ok, Result};

pub mod model;
pub use model::*;

#[derive(Clone)]
pub struct Db{
    pub pool : PgPool
}

impl Db {
    pub async fn new()->Result<Self>{
        dotenvy::dotenv().ok();
        let db = "postgresql://postgres:postgres@localhost:5432/perp";
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&db)
            .await?;

        Ok(Self{
            pool
         })
    }
    
}