use rust_decimal::Decimal;
use tokio::sync::{mpsc, oneshot};
use uuid::Uuid;

use crate::order_book::{Command, Event, OrderBook, OrderBookState};

use crate::{database, Error, Result};

#[derive(Clone)]
pub struct Client {
    sender: mpsc::Sender<Request>,
}

impl Client {
    fn new(tx: mpsc::Sender<Request>) -> Self {
        Self { sender: tx }
    }

    async fn call(&self, command: Command) -> Result<Vec<Event>> {
        let (sender, receiver) = oneshot::channel();
        self.sender
            .send(Request::new(command, sender))
            .await
            .map_err(|error| {
                tracing::error!("Fail to send command to actor, error={}", error);
                Error::application_error("Internal server error")
            })?;
        let events = receiver.await.map_err(|error| {
            tracing::warn!(
                "Fail to send back the response for the caller, dropping response, error={}",
                error
            );
            Error::application_error("Internal server error")
        })?;
        Ok(events)
    }

    pub async fn get_order_book(&self) -> Result<OrderBookState> {
        let mut events = self.call(Command::GetState).await?;
        match (events.len(), events.pop()) {
            (1, Some(Event::State { state })) => Ok(state),
            _ => Err(Error::application_error("Internal server error")),
        }
    }

    pub async fn buy(&self, quantity: u32, price: Decimal) -> Result<Vec<Event>> {
        self.call(Command::Buy { quantity, price }).await
    }

    pub async fn sell(&self, quantity: u32, price: Decimal) -> Result<Vec<Event>> {
        self.call(Command::Sell { quantity, price }).await
    }

    pub async fn cancel(&self, order: Uuid) -> Result<Vec<Event>> {
        self.call(Command::Cancel { id: order }).await
    }

    pub async fn update(&self, order: Uuid, quantity: u32, price: Decimal) -> Result<Vec<Event>> {
        self.call(Command::Update {
            id: order,
            new_quantity: quantity,
            new_price: price,
        })
        .await
    }
}

#[derive(Debug)]
pub struct Request {
    command: Command,
    callback: oneshot::Sender<Vec<Event>>,
}

impl Request {
    fn new(command: Command, callback: oneshot::Sender<Vec<Event>>) -> Self {
        Self { command, callback }
    }
}

pub struct Actor {
    receiver: mpsc::Receiver<Request>,
    order_book: OrderBook,
    db: sqlx::Pool<sqlx::Sqlite>,
}

impl Actor {
    fn new(db: sqlx::Pool<sqlx::Sqlite>, receiver: mpsc::Receiver<Request>, ticker: &str) -> Self {
        Self {
            db,
            receiver,
            order_book: OrderBook::new(ticker),
        }
    }

    pub async fn run(mut self) -> Result<()> {
        tracing::info!("Waiting for commands");
        while let Some(request) = self.receiver.recv().await {
            let events = self.order_book.process(request.command);
            match database::save_events(&self.db, &events).await {
                Ok(_) => match request.callback.send(events) {
                    Ok(_) => (),
                    Err(events) => {
                        tracing::error!("Sender dropped the message, events dropped={:?}", events);
                    }
                },
                Err(error) => {
                    tracing::error!("Fail to persist events={:?}, error={}", events, error);
                    panic!("Fail to persist events! Error={}", error)
                }
            }
        }
        Ok(())
    }
}

pub fn build(db: sqlx::Pool<sqlx::Sqlite>, ticker: &str, channel_buffer: usize) -> (Client, Actor) {
    let (sender, receiver) = mpsc::channel(channel_buffer);
    let client = Client::new(sender);
    let server = Actor::new(db, receiver, ticker);
    (client, server)
}
