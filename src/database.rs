use anyhow::Result;
use bigdecimal::BigDecimal;
use chrono::NaiveDate;
use scylla::{Session, SessionBuilder, ValueList};
use strum_macros::{AsRefStr, EnumString};
use uuid::Uuid;

use crate::{order_book::Event, Config};

pub async fn connect(config: &Config) -> Result<Session> {
    let hosts: Vec<_> = config.database_url.split(",").collect();
    SessionBuilder::new()
        .known_nodes(&hosts)
        .default_consistency(scylla::statement::Consistency::One)
        .build()
        .await
        .map_err(From::from)
}

pub async fn run_migrations(db: &Session) -> Result<()> {
    let keyspace = r#"
    CREATE KEYSPACE IF NOT EXISTS orderbook WITH REPLICATION = {
        'class': 'SimpleStrategy',
        'replication_factor': 1
    };
    "#;
    let table = r#"
    CREATE TABLE IF NOT EXISTS orderbook.events (
        ticker Text,
        event_date Date,
        event_id Uuid,
        ts Timestamp,
        event_type Text,
        order_id Uuid,
        order_quantity Int,
        order_price Decimal,
        counterpart_id Uuid,
        counterpart_quantity Int,
        counterpart_price Decimal,
        PRIMARY KEY (ticker, event_date, ts, event_id)
    ) WITH CLUSTERING ORDER BY (event_date ASC, ts ASC, event_id ASC);
    "#;
    db.query(keyspace, ()).await?;
    db.query(table, ()).await?;
    Ok(())
}

#[derive(Debug, Clone, Copy, EnumString, AsRefStr)]
#[strum(serialize_all = "snake_case")]
enum EventType {
    Buy,
    Sell,
    Fill,
    Cancel,
}

#[derive(ValueList)]
struct Row {
    ticker: String,
    event_date: NaiveDate,
    event_id: Uuid,
    ts: i64,
    event_type: String,
    order_id: Uuid,
    order_quantity: Option<i32>,
    order_price: Option<BigDecimal>,
    counterpart_id: Option<Uuid>,
    counterpart_quantity: Option<i32>,
    counterpart_price: Option<BigDecimal>,
}

fn to_row(ticker: &str, value: &Event) -> Option<Row> {
    match value {
        Event::Filled {
            ts,
            order,
            counterpart,
        } => Some(Row {
            event_id: Uuid::new_v4(),
            event_date: ts.date_naive(),
            ticker: ticker.to_owned(),
            ts: ts.timestamp_millis(),
            event_type: EventType::Fill.as_ref().to_owned(),
            order_id: order.id,
            order_quantity: Some(order.quantity as i32),
            order_price: Some(order.price.clone()),
            counterpart_id: Some(counterpart.id),
            counterpart_quantity: Some(counterpart.quantity as i32),
            counterpart_price: Some(counterpart.price.clone()),
        }),
        Event::Accepted { ts, order } => {
            let event_type = match order.order_type {
                crate::order_book::OrderType::Sell => EventType::Sell,
                crate::order_book::OrderType::Buy => EventType::Buy,
            };
            Some(Row {
                event_id: Uuid::new_v4(),
                ticker: ticker.to_owned(),
                event_date: ts.date_naive(),
                ts: ts.timestamp_millis(),
                event_type: event_type.as_ref().to_owned(),
                order_id: order.id,
                order_quantity: Some(order.quantity as i32),
                order_price: Some(order.price.clone()),
                counterpart_id: None,
                counterpart_quantity: None,
                counterpart_price: None,
            })
        }
        Event::Canceled { ts, order } => Some(Row {
            event_id: Uuid::new_v4(),
            ticker: ticker.to_owned(),
            event_date: ts.date_naive(),
            ts: ts.timestamp_millis(),
            event_type: EventType::Cancel.as_ref().to_owned(),
            order_id: order.id,
            order_quantity: None,
            order_price: None,
            counterpart_id: None,
            counterpart_quantity: None,
            counterpart_price: None,
        }),
        Event::Rejected { .. } => None,
        Event::State { .. } => None,
    }
}

pub async fn save_events(ticker: &str, db: &Session, events: &[Event]) -> Result<()> {
    let insert = r#"
    INSERT INTO orderbook.events
    (ticker, event_date, event_id, ts, event_type, order_id, order_quantity, order_price, counterpart_id, counterpart_quantity, counterpart_price)
    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
    "#;
    let prepared = db.prepare(insert).await?;
    let rows: Vec<_> = events.iter().filter_map(|e| to_row(ticker, e)).collect();
    for row in rows.iter() {
        let _ = db.execute(&prepared, row).await?;
    }
    Ok(())
}
