use std::{
    cmp::Ordering,
    collections::{BTreeSet, HashMap},
    rc::Rc,
};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub enum Command {
    Buy {
        quantity: u32,
        price: u32, // FIXME
    },
    Sell {
        quantity: u32,
        price: u32, // FIXME
    },
    Cancel {
        id: Uuid,
    },
    Update {
        id: Uuid,
        new_quantity: u32,
        new_price: u32, // FIXME
    },
    GetState,
}

#[derive(Debug, Serialize)]
pub enum Event {
    Filled {
        ts: DateTime<Utc>,
        price: u32,
        quantity: u32,
        orders: Vec<Uuid>,
    },
    Accepted {
        order: Order,
    },
    Canceled {
        ts: DateTime<Utc>,
        order: Uuid,
    },
    Rejected {
        ts: DateTime<Utc>,
        reason: String,
    },
    State {
        state: OrderBookState,
    },
}

#[derive(Debug, PartialEq, PartialOrd, Eq, Clone, Copy, Serialize, Deserialize)]
pub enum OrderType {
    Sell,
    Buy,
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct Order {
    order_type: OrderType,
    id: Uuid,
    ts: DateTime<Utc>,
    quantity: u32,
    price: u32, // FIXME
}

#[derive(Debug, PartialEq, PartialOrd, Eq, Clone, Serialize, Deserialize)]
pub struct OrderBookState {
    pub buy: Vec<Order>,
    pub sell: Vec<Order>,
}

impl Order {
    pub fn sell(quantity: u32, price: u32) -> Self {
        Self {
            order_type: OrderType::Sell,
            id: Uuid::new_v4(),
            ts: Utc::now(),
            quantity,
            price,
        }
    }
    pub fn buy(quantity: u32, price: u32) -> Self {
        Self {
            order_type: OrderType::Buy,
            id: Uuid::new_v4(),
            ts: Utc::now(),
            quantity,
            price,
        }
    }
}

impl Ord for Order {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.order_type {
            OrderType::Sell => match self.price.cmp(&other.price) {
                Ordering::Equal => self.ts.cmp(&other.ts),
                ord => ord,
            },
            OrderType::Buy => match self.price.cmp(&other.price).reverse() {
                Ordering::Equal => self.ts.cmp(&other.ts),
                ord => ord,
            },
        }
    }
}

impl PartialOrd for Order {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match self.order_type.partial_cmp(&other.order_type) {
            Some(core::cmp::Ordering::Equal) => {}
            ord => return ord,
        }
        match self.id.partial_cmp(&other.id) {
            Some(core::cmp::Ordering::Equal) => {}
            ord => return ord,
        }
        match self.ts.partial_cmp(&other.ts) {
            Some(core::cmp::Ordering::Equal) => {}
            ord => return ord,
        }
        match self.quantity.partial_cmp(&other.quantity) {
            Some(core::cmp::Ordering::Equal) => {}
            ord => return ord,
        }
        self.price.partial_cmp(&other.price)
    }
}

#[derive(Debug)]
pub struct OrderBook {
    pub ticker: String,
    version: u128,
    sell_book: BTreeSet<Rc<Order>>,
    sell_index: HashMap<Uuid, Rc<Order>>,
    buy_book: BTreeSet<Rc<Order>>,
    buy_index: HashMap<Uuid, Rc<Order>>,
}

impl OrderBook {
    pub fn new(ticker: &str) -> Self {
        OrderBook {
            version: 0,
            ticker: ticker.to_owned(),
            sell_book: BTreeSet::new(),
            sell_index: HashMap::new(),
            buy_book: BTreeSet::new(),
            buy_index: HashMap::new(),
        }
    }

    pub fn process(&mut self, command: Command) -> Vec<Event> {
        self.version += 1;
        match command {
            Command::Buy { quantity, price } => {
                let mut events = vec![];
                self.process_buy_order(&mut events, Order::buy(quantity, price));
                events
            }
            Command::Sell { quantity, price } => {
                let mut events = vec![];
                self.process_sell_order(&mut events, Order::sell(quantity, price));
                events
            }
            Command::Cancel { id } => self.process_cancel_order(&id),
            Command::Update {
                id,
                new_quantity,
                new_price,
            } => self.process_update_order(&id, new_quantity, new_price),
            Command::GetState => {
                let buy = self
                    .buy_book
                    .clone()
                    .into_iter()
                    .map(|rc| (*rc).clone())
                    .collect();
                let sell = self
                    .sell_book
                    .clone()
                    .into_iter()
                    .map(|rc| (*rc).clone())
                    .collect();
                vec![Event::State {
                    state: OrderBookState { buy, sell },
                }]
            }
        }
    }

    fn process_sell_order(&mut self, events: &mut Vec<Event>, order: Order) {
        OrderBook::process_order(
            events,
            order,
            &mut self.buy_book,
            &mut self.buy_index,
            &mut self.sell_book,
            &mut self.sell_index,
        )
    }

    fn process_buy_order(&mut self, events: &mut Vec<Event>, order: Order) {
        OrderBook::process_order(
            events,
            order,
            &mut self.sell_book,
            &mut self.sell_index,
            &mut self.buy_book,
            &mut self.buy_index,
        )
    }

    fn process_order(
        events: &mut Vec<Event>,
        order: Order,
        counterpart_book: &mut BTreeSet<Rc<Order>>,
        counterpart_index: &mut HashMap<Uuid, Rc<Order>>,
        source_book: &mut BTreeSet<Rc<Order>>,
        source_index: &mut HashMap<Uuid, Rc<Order>>,
    ) {
        match counterpart_book.pop_first() {
            Some(counterpart_order) if order.price <= counterpart_order.price => {
                match order.quantity.cmp(&counterpart_order.quantity) {
                    Ordering::Less => {
                        let new_counterpart_order = Order {
                            order_type: counterpart_order.order_type,
                            id: counterpart_order.id,
                            ts: counterpart_order.ts,
                            price: counterpart_order.price,
                            quantity: counterpart_order.quantity - order.quantity,
                        };
                        let rc = Rc::new(new_counterpart_order);
                        counterpart_book.insert(rc.clone());
                        counterpart_index.insert(rc.id, rc);
                        events.push(Event::Filled {
                            ts: Utc::now(),
                            price: order.price,
                            quantity: order.quantity,
                            orders: vec![order.id, counterpart_order.id],
                        });
                    }
                    Ordering::Greater => {
                        counterpart_book.remove(&order);
                        counterpart_index.remove(&order.id);
                        let new_source_order = Order {
                            order_type: order.order_type,
                            id: order.id,
                            ts: order.ts,
                            price: order.price,
                            quantity: order.quantity - counterpart_order.quantity,
                        };
                        OrderBook::process_order(
                            events,
                            new_source_order,
                            counterpart_book,
                            counterpart_index,
                            source_book,
                            source_index,
                        )
                    }
                    Ordering::Equal => events.push(Event::Filled {
                        ts: Utc::now(),
                        price: order.price,
                        quantity: order.quantity,
                        orders: vec![order.id, counterpart_order.id],
                    }),
                }
            }
            _ => {
                events.push(Event::Accepted {
                    order: order.clone(),
                });
                let rc = Rc::new(order);
                source_book.insert(rc.clone());
                source_index.insert(rc.id, rc);
            }
        }
    }

    fn process_cancel_order(&mut self, id: &Uuid) -> Vec<Event> {
        match (self.sell_index.get(id), self.buy_index.get(id)) {
            (Some(sell_order), None) => {
                self.sell_book.remove(sell_order);
                self.sell_index.remove(id);
                vec![Event::Canceled {
                    ts: Utc::now(),
                    order: id.to_owned(),
                }]
            }
            (None, Some(buy_order)) => {
                self.buy_book.remove(buy_order);
                self.buy_index.remove(id);
                vec![Event::Canceled {
                    ts: Utc::now(),
                    order: id.to_owned(),
                }]
            }
            (None, None) => vec![Event::Rejected {
                ts: Utc::now(),
                reason: format!("Order {} not found in sell or buy side", id),
            }],
            (Some(_), Some(_)) => {
                panic!("Bug, order found in both sides");
            }
        }
    }

    fn process_update_order(&mut self, id: &Uuid, new_quantity: u32, new_price: u32) -> Vec<Event> {
        match (self.sell_index.get(id), self.buy_index.get(id)) {
            (Some(_), None) => {
                let mut events = self.process_cancel_order(id);
                self.process_sell_order(&mut events, Order::sell(new_quantity, new_price));
                events
            }
            (None, Some(_)) => {
                let mut events = self.process_cancel_order(id);
                self.process_buy_order(&mut events, Order::buy(new_quantity, new_price));
                events
            }
            (None, None) => vec![Event::Rejected {
                ts: Utc::now(),
                reason: format!("Order {} not found in sell or buy side", id),
            }],
            (Some(_), Some(_)) => {
                panic!("Bug, order found in both sides");
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_order_compare_on_same_price() {
        {
            let order1 = Order::buy(10, 1);
            let order2 = Order::buy(10, 1);
            assert_eq!(order1.cmp(&order2), Ordering::Less);
        }
        {
            let order1 = Order::sell(10, 1);
            let order2 = Order::sell(10, 1);
            assert_eq!(order1.cmp(&order2), Ordering::Less);
        }
    }

    #[test]
    fn test_order_book_basic() {
        let mut order_book = OrderBook::new("test");
        let events = order_book.process(Command::Buy {
            quantity: 10,
            price: 3,
        });
        assert_eq!(events.len(), 1);
        assert!(matches!(
            events.first().unwrap(),
            Event::Accepted { order: _ }
        ));

        let events = order_book.process(Command::Buy {
            quantity: 10,
            price: 3,
        });
        assert_eq!(events.len(), 1);
        assert!(matches!(
            events.first().unwrap(),
            Event::Accepted { order: _ }
        ));

        let events = order_book.process(Command::Sell {
            quantity: 5,
            price: 3,
        });
        assert_eq!(events.len(), 1);
        assert!(matches!(
            events.first().unwrap(),
            Event::Filled {
                ts: _,
                price: _,
                quantity: _,
                orders: _
            }
        ));

        let events = order_book.process(Command::Sell {
            quantity: 5,
            price: 3,
        });
        assert_eq!(events.len(), 1);
        assert!(matches!(
            events.first().unwrap(),
            Event::Filled {
                ts: _,
                price: _,
                quantity: _,
                orders: _
            }
        ));

        assert_eq!(order_book.sell_book.len(), 0);
        assert_eq!(order_book.buy_book.len(), 1);
    }
}
