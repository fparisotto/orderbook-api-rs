pub mod actor;
pub mod database;
pub mod endpoints;
pub mod order_book;

use std::sync::Arc;

use actor::Client;
// use sqlx::{Pool, Postgres};

// use anyhow::Context;

#[derive(Clone)]
pub struct AppContext {
    // pub db: Pool<Postgres>,
    pub config: Arc<Config>,
    pub actor_client: Client,
}

pub struct Config {
    // pub database_url: String,
    // pub database_connection_pool_size: u8,
}

impl Config {
    pub fn parse() -> anyhow::Result<Self> {
        // let database_url = std::env::var("DATABASE_URL").context("env DATABASE_URL is required")?;
        // let database_connection_pool_size: u8 =
        //     std::env::var("DATABASE_CONNECTION_POOL_SIZE")?.parse()?;
        // Ok(Config {
        //     database_url,
        //     database_connection_pool_size,
        // })
        Ok(Config {})
    }
}

pub struct Error(anyhow::Error);

impl Error {
    pub fn new(message: &str) -> Self {
        Error(anyhow::anyhow!(message.to_owned()))
    }
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

impl<E> From<E> for Error
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}
