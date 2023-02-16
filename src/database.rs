use anyhow::Result;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sqlx::postgres::{PgHasArrayType, PgPoolOptions};
use uuid::Uuid;

use crate::{order_book::Event, Config};

pub async fn connect(config: &Config) -> Result<sqlx::Pool<sqlx::Postgres>> {
    let db: sqlx::Pool<sqlx::Postgres> = PgPoolOptions::new()
        .max_connections(config.database_connection_pool_size as u32)
        .connect(&config.database_url)
        .await?;
    Ok(db)
}

pub async fn run_migrations(db: &sqlx::Pool<sqlx::Postgres>) -> Result<()> {
    sqlx::migrate!().run(db).await?;
    Ok(())
}

pub async fn run_health_check(db: &sqlx::Pool<sqlx::Postgres>) -> Result<()> {
    let _: i32 = sqlx::query_scalar("select 1").fetch_one(db).await?;
    Ok(())
}

#[derive(Debug, Clone, Copy, sqlx::Type)]
#[sqlx(type_name = "EVENT_TYPE_ENUM", rename_all = "lowercase")]
enum EventType {
    Buy,
    Sell,
    Fill,
    Cancel,
}

impl PgHasArrayType for EventType {
    fn array_type_info() -> sqlx::postgres::PgTypeInfo {
        sqlx::postgres::PgTypeInfo::with_name("_EVENT_TYPE_ENUM")
    }
}

#[derive(Debug)]
struct EventRow {
    ts: DateTime<Utc>,
    event_type: EventType,
    order_id: Uuid,
    order_quantity: Option<i32>,
    order_price: Option<Decimal>,
    counterpart_id: Option<Uuid>,
    counterpart_quantity: Option<i32>,
    counterpart_price: Option<Decimal>,
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
                event_type: EventType::Fill,
                order_id: order.id.clone(),
                order_quantity: Some(order.quantity as i32),
                order_price: Some(order.price),
                counterpart_id: Some(counterpart.id),
                counterpart_quantity: Some(counterpart.quantity as i32),
                counterpart_price: Some(counterpart.price),
            }),
            Event::Accepted { ts, order } => {
                let event_type = match order.order_type {
                    crate::order_book::OrderType::Sell => EventType::Sell,
                    crate::order_book::OrderType::Buy => EventType::Buy,
                };
                Ok(EventRow {
                    ts: ts.clone(),
                    event_type,
                    order_id: order.id.clone(),
                    order_quantity: Some(order.quantity as i32),
                    order_price: Some(order.price),
                    counterpart_id: None,
                    counterpart_quantity: None,
                    counterpart_price: None,
                })
            }
            Event::Canceled { ts, order } => Ok(EventRow {
                ts: ts.clone(),
                event_type: EventType::Cancel,
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

pub async fn save_events(db: &sqlx::Pool<sqlx::Postgres>, events: &Vec<Event>) -> Result<()> {
    let sql = r#"
    INSERT INTO orderbook_event
    (ts, event_type, order_id, order_quantity, order_price, counterpart_id, counterpart_quantity, counterpart_price)
    SELECT * FROM UNNEST(
    $1::TIMESTAMPTZ[], $2::EVENT_TYPE_ENUM[], $3::UUID[], $4::INT[], $5::NUMERIC[], $6::UUID[], $7::INT[], $8::NUMERIC[]
    );"#;
    let rows: Vec<EventRow> = events.iter().filter_map(|e| e.try_into().ok()).collect();
    if rows.is_empty() {
        return Ok(());
    }
    let ts: Vec<DateTime<Utc>> = rows.iter().map(|row| row.ts).collect();
    let event_type: Vec<EventType> = rows.iter().map(|row| row.event_type).collect();
    let order_id: Vec<Uuid> = rows.iter().map(|row| row.order_id).collect();
    let order_quantity: Vec<Option<i32>> = rows.iter().map(|row| row.order_quantity).collect();
    let order_price: Vec<Option<Decimal>> = rows.iter().map(|row| row.order_price).collect();
    let counterpart_id: Vec<Option<Uuid>> = rows.iter().map(|row| row.counterpart_id).collect();
    let counterpart_quantity: Vec<Option<i32>> =
        rows.iter().map(|row| row.counterpart_quantity).collect();
    let counterpart_price: Vec<Option<Decimal>> =
        rows.iter().map(|row| row.counterpart_price).collect();
    let _ = sqlx::query(sql)
        .bind(&ts[..])
        .bind(&event_type[..])
        .bind(&order_id[..])
        .bind(&order_quantity[..])
        .bind(&order_price[..])
        .bind(&counterpart_id[..])
        .bind(&counterpart_quantity[..])
        .bind(&counterpart_price[..])
        .execute(db)
        .await?;
    Ok(())
}
