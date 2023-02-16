pub mod actor;
pub mod database;
pub mod endpoints;
pub mod order_book;

use std::sync::Arc;

use actor::Client;
use chrono::{DateTime, Utc};
use sqlx::{Pool, Postgres};

#[derive(Clone)]
pub struct AppContext {
    pub db: Pool<Postgres>,
    pub config: Arc<Config>,
    pub actor_client: Client,
}

pub struct Config {
    pub database_url: String,
    pub database_connection_pool_size: u8,
}

impl Config {
    pub fn parse() -> anyhow::Result<Self> {
        let database_url = std::env::var("DATABASE_URL")?;
        let pool_size: u8 = std::env::var("DATABASE_CONNECTION_POOL_SIZE")?.parse()?;
        Ok(Config {
            database_url,
            database_connection_pool_size: pool_size,
        })
    }
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("rejection")]
    EventRejection { ts: DateTime<Utc>, reason: String },

    #[error("database_error")]
    Database(#[from] sqlx::Error),

    #[error("migration_error")]
    Migration(#[from] sqlx::migrate::MigrateError),

    #[error("internal_server_error")]
    Anyhow(#[from] anyhow::Error),

    #[error("internal_server_error")]
    ApplicationError { reason: String },
}

pub(crate) type Result<T, E = Error> = std::result::Result<T, E>;

impl Error {
    pub fn event_rejection(ts: DateTime<Utc>, reason: impl Into<String>) -> Self {
        Self::EventRejection {
            ts,
            reason: reason.into(),
        }
    }

    pub fn application_error(reason: impl Into<String>) -> Self {
        Self::ApplicationError {
            reason: reason.into(),
        }
    }
}
