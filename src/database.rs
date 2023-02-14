use anyhow::Result;
// use sqlx::postgres::PgPoolOptions;

use crate::{order_book::Event, Config};

pub async fn connect(_config: &Config) -> Result<sqlx::Pool<sqlx::Postgres>> {
    // let db: sqlx::Pool<sqlx::Postgres> = PgPoolOptions::new()
    //     .max_connections(config.database_connection_pool_size as u32)
    //     .connect(&config.database_url)
    //     .await?;
    // Ok(db)
    todo!()
}

pub async fn run_migrations(db: &sqlx::Pool<sqlx::Postgres>) -> Result<()> {
    sqlx::migrate!().run(db).await?;
    Ok(())
}

pub async fn save_event(_db: &sqlx::Pool<sqlx::Postgres>, _event: Event) -> Result<()> {
    todo!()
}
