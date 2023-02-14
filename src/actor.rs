use tokio::sync::{mpsc, oneshot};
use uuid::Uuid;

use crate::order_book::{Command, Event, OrderBook, OrderBookState};

use crate::{Error, Result};

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
        self.sender.send(Request::new(command, sender)).await?;
        Ok(receiver.await?)
    }

    pub async fn get_order_book(&self) -> Result<OrderBookState> {
        let mut events = self.call(Command::GetState).await?;
        match (events.len(), events.pop()) {
            (1, Some(Event::State { state })) => Ok(state),
            _ => Err(Error::new("Internal server error")),
        }
    }

    pub async fn buy(&self, quantity: u32, price: u32) -> Result<Vec<Event>> {
        self.call(Command::Buy { quantity, price }).await
    }

    pub async fn sell(&self, quantity: u32, price: u32) -> Result<Vec<Event>> {
        self.call(Command::Sell { quantity, price }).await
    }

    pub async fn cancel(&self, order: Uuid) -> Result<Vec<Event>> {
        self.call(Command::Cancel { id: order }).await
    }

    pub async fn update(&self, order: Uuid, quantity: u32, price: u32) -> Result<Vec<Event>> {
        self.call(Command::Update {
            id: order,
            new_quantity: quantity,
            new_price: price,
        })
        .await
    }
}

#[derive(Debug)]
struct Request {
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
}

impl Actor {
    fn new(receiver: mpsc::Receiver<Request>, ticker: &str) -> Self {
        Self {
            receiver,
            order_book: OrderBook::new(ticker),
        }
    }

    pub async fn run(mut self) -> Result<()> {
        tracing::info!("Waiting for commands");
        while let Some(request) = self.receiver.recv().await {
            let events = self.order_book.process(request.command);
            match request.callback.send(events) {
                Ok(_) => (),
                Err(events) => {
                    tracing::error!("Sender dropped the message, events dropped={:?}", events);
                }
            }
        }
        Ok(())
    }
}

pub fn build(ticker: &str, channel_buffer: usize) -> (Client, Actor) {
    let (sender, receiver) = mpsc::channel(channel_buffer);
    let client = Client::new(sender);
    let server = Actor::new(receiver, ticker);
    (client, server)
}
