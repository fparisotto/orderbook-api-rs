use crate::{order_book::Event, Config};
use anyhow::Result;
use chrono::{DateTime, Utc};
use rust_decimal::prelude::*;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use uuid::Uuid;

type SqlxPool = sqlx::Pool<sqlx::Sqlite>;

pub async fn connect(config: &Config) -> Result<SqlxPool> {
    let options = SqliteConnectOptions::new()
        .create_if_missing(true)
        .filename(&config.database_file)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        .synchronous(sqlx::sqlite::SqliteSynchronous::Normal)
        .locking_mode(sqlx::sqlite::SqliteLockingMode::Exclusive);

    let db = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await?;

    Ok(db)
}

pub async fn run_migrations(db: &SqlxPool) -> Result<()> {
    sqlx::migrate!().run(db).await?;
    Ok(())
}

pub async fn run_health_check(db: &SqlxPool) -> Result<()> {
    let _: i32 = sqlx::query_scalar("select 1").fetch_one(db).await?;
    Ok(())
}

#[derive(Debug, Clone, Copy, sqlx::Type)]
enum EventType {
    Buy,
    Sell,
    Fill,
    Cancel,
}

impl EventType {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Buy => "buy",
            Self::Sell => "sell",
            Self::Fill => "fill",
            Self::Cancel => "cancel",
        }
    }
}

#[derive(Debug)]
struct EventRow {
    ts: DateTime<Utc>,
    event_type: &'static str,
    order_id: Uuid,
    order_quantity: Option<i32>,
    order_price: Option<f64>,
    counterpart_id: Option<Uuid>,
    counterpart_quantity: Option<i32>,
    counterpart_price: Option<f64>,
}

impl TryFrom<&Event> for EventRow {
    type Error = ();

    fn try_from(value: &Event) -> Result<Self, Self::Error> {
        match value {
            Event::Filled {
                ts,
                order,
                counterpart,
            } => Ok(EventRow {
                ts: ts.clone(),
                event_type: EventType::Fill.as_str(),
                order_id: order.id.clone(),
                order_quantity: Some(order.quantity as i32),
                order_price: Some(order.price.to_f64().unwrap()),
                counterpart_id: Some(counterpart.id),
                counterpart_quantity: Some(counterpart.quantity as i32),
                counterpart_price: Some(counterpart.price.to_f64().unwrap()),
            }),
            Event::Accepted { ts, order } => {
                let event_type = match order.order_type {
                    crate::order_book::OrderType::Sell => EventType::Sell,
                    crate::order_book::OrderType::Buy => EventType::Buy,
                };
                Ok(EventRow {
                    ts: ts.clone(),
                    event_type: event_type.as_str(),
                    order_id: order.id.clone(),
                    order_quantity: Some(order.quantity as i32),
                    order_price: Some(order.price.to_f64().unwrap()),
                    counterpart_id: None,
                    counterpart_quantity: None,
                    counterpart_price: None,
                })
            }
            Event::Canceled { ts, order } => Ok(EventRow {
                ts: ts.clone(),
                event_type: EventType::Cancel.as_str(),
                order_id: order.id.clone(),
                order_quantity: None,
                order_price: None,
                counterpart_id: None,
                counterpart_quantity: None,
                counterpart_price: None,
            }),
            Event::Rejected { .. } => Err(()),
            Event::State { .. } => Err(()),
        }
    }
}

pub async fn save_events(db: &SqlxPool, events: &Vec<Event>) -> Result<()> {
    let sql = r#"INSERT INTO orderbook_event
    (ts, event_type, order_id, order_quantity, order_price, counterpart_id, counterpart_quantity, counterpart_price)
    VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"#;

    let rows: Vec<EventRow> = events.iter().filter_map(|e| e.try_into().ok()).collect();
    if rows.is_empty() {
        return Ok(());
    }

    let mut tx = db.begin().await?;
    for row in rows {
        let _ = sqlx::query(sql)
            .bind(row.ts)
            .bind(row.event_type)
            .bind(row.order_id)
            .bind(row.order_quantity)
            .bind(row.order_price)
            .bind(row.counterpart_id)
            .bind(row.counterpart_quantity)
            .bind(row.counterpart_price)
            .execute(&mut tx)
            .await?;
    }
    tx.commit().await?;
    Ok(())
}
