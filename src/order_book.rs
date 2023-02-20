use std::{
    cmp::Ordering,
    collections::{BTreeSet, HashMap},
    rc::Rc,
};

use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub enum Command {
    Buy {
        quantity: u32,
        price: BigDecimal,
    },
    Sell {
        quantity: u32,
        price: BigDecimal,
    },
    Cancel {
        id: Uuid,
    },
    Update {
        id: Uuid,
        new_quantity: u32,
        new_price: BigDecimal,
    },
    GetState,
}

#[derive(Debug, Serialize)]
pub enum Event {
    Filled {
        ts: DateTime<Utc>,
        order: Order,
        counterpart: Order,
    },
    Accepted {
        ts: DateTime<Utc>,
        order: Order,
    },
    Canceled {
        ts: DateTime<Utc>,
        order: Order,
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
    pub order_type: OrderType,
    pub id: Uuid,
    pub ts: DateTime<Utc>,
    pub quantity: u32,
    pub price: BigDecimal,
}

#[derive(Debug, PartialEq, PartialOrd, Eq, Clone, Serialize, Deserialize)]
pub struct OrderBookState {
    pub buy: Vec<Order>,
    pub sell: Vec<Order>,
}

impl OrderBookState {
    fn new(order_book: &OrderBook) -> Self {
        Self {
            buy: order_book
                .buy_book
                .clone()
                .into_iter()
                .map(|rc| (*rc).clone())
                .collect(),
            sell: order_book
                .sell_book
                .clone()
                .into_iter()
                .map(|rc| (*rc).clone())
                .collect(),
        }
    }
}

impl Order {
    pub fn sell(ts: DateTime<Utc>, quantity: u32, price: BigDecimal) -> Self {
        Self {
            id: Uuid::new_v4(),
            order_type: OrderType::Sell,
            ts,
            quantity,
            price,
        }
    }
    pub fn buy(ts: DateTime<Utc>, quantity: u32, price: BigDecimal) -> Self {
        Self {
            id: Uuid::new_v4(),
            order_type: OrderType::Buy,
            ts,
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
    ts: DateTime<Utc>,
    sell_book: BTreeSet<Rc<Order>>,
    sell_index: HashMap<Uuid, Rc<Order>>,
    buy_book: BTreeSet<Rc<Order>>,
    buy_index: HashMap<Uuid, Rc<Order>>,
}

impl OrderBook {
    pub fn new(ticker: &str) -> Self {
        OrderBook {
            ts: Utc::now(),
            ticker: ticker.to_owned(),
            sell_book: BTreeSet::new(),
            sell_index: HashMap::new(),
            buy_book: BTreeSet::new(),
            buy_index: HashMap::new(),
        }
    }

    pub fn process(&mut self, command: Command) -> Vec<Event> {
        let ts = Utc::now();
        match command {
            Command::Buy { quantity, price } => {
                let mut events = vec![];
                self.process_buy_order(ts, &mut events, Order::buy(ts, quantity, price));
                self.ts = ts;
                events
            }
            Command::Sell { quantity, price } => {
                let mut events = vec![];
                self.process_sell_order(ts, &mut events, Order::sell(ts, quantity, price));
                self.ts = ts;
                events
            }
            Command::Cancel { id } => {
                let events = self.process_cancel_order(ts, id);
                self.ts = ts;
                events
            }
            Command::Update {
                id,
                new_quantity,
                new_price,
            } => {
                let events = self.process_update_order(ts, id, new_quantity, new_price);
                self.ts = ts;
                events
            }
            Command::GetState => {
                vec![Event::State {
                    state: OrderBookState::new(self),
                }]
            }
        }
    }

    fn process_sell_order(&mut self, ts: DateTime<Utc>, events: &mut Vec<Event>, order: Order) {
        OrderBook::process_order(
            ts,
            events,
            order,
            &mut self.buy_book,
            &mut self.buy_index,
            &mut self.sell_book,
            &mut self.sell_index,
        )
    }

    fn process_buy_order(&mut self, ts: DateTime<Utc>, events: &mut Vec<Event>, order: Order) {
        OrderBook::process_order(
            ts,
            events,
            order,
            &mut self.sell_book,
            &mut self.sell_index,
            &mut self.buy_book,
            &mut self.buy_index,
        )
    }

    fn process_order(
        ts: DateTime<Utc>,
        events: &mut Vec<Event>,
        order: Order,
        counterpart_book: &mut BTreeSet<Rc<Order>>,
        counterpart_index: &mut HashMap<Uuid, Rc<Order>>,
        source_book: &mut BTreeSet<Rc<Order>>,
        source_index: &mut HashMap<Uuid, Rc<Order>>,
    ) {
        events.push(Event::Accepted {
            ts,
            order: order.clone(),
        });
        match counterpart_book.pop_first() {
            Some(counterpart) if order.price <= counterpart.price => {
                match order.quantity.cmp(&counterpart.quantity) {
                    Ordering::Less => {
                        let new_counterpart = Order {
                            order_type: counterpart.order_type,
                            id: counterpart.id,
                            ts: counterpart.ts,
                            price: counterpart.price.clone(),
                            quantity: counterpart.quantity - order.quantity,
                        };
                        let rc = Rc::new(new_counterpart);
                        counterpart_book.insert(rc.clone());
                        counterpart_index.insert(rc.id, rc);
                        events.push(Event::Filled {
                            ts,
                            order: order.clone(),
                            counterpart: counterpart.as_ref().clone(),
                        });
                    }
                    Ordering::Greater => {
                        counterpart_book.remove(&order);
                        counterpart_index.remove(&order.id);
                        events.push(Event::Filled {
                            ts,
                            order: order.clone(),
                            counterpart: counterpart.as_ref().clone(),
                        });
                        let new_source_order = Order {
                            order_type: order.order_type,
                            id: order.id,
                            ts: order.ts,
                            price: order.price,
                            quantity: order.quantity - counterpart.quantity,
                        };
                        OrderBook::process_order(
                            ts,
                            events,
                            new_source_order,
                            counterpart_book,
                            counterpart_index,
                            source_book,
                            source_index,
                        )
                    }
                    Ordering::Equal => events.push(Event::Filled {
                        ts,
                        order: order.clone(),
                        counterpart: counterpart.as_ref().clone(),
                    }),
                }
            }
            _ => {
                let rc = Rc::new(order);
                source_book.insert(rc.clone());
                source_index.insert(rc.id, rc);
            }
        }
    }

    fn process_cancel_order(&mut self, ts: DateTime<Utc>, id: Uuid) -> Vec<Event> {
        match (self.sell_index.get(&id), self.buy_index.get(&id)) {
            (Some(sell_order), None) => {
                let order: Order = sell_order.as_ref().clone();
                self.sell_book.remove(sell_order);
                self.sell_index.remove(&id);
                vec![Event::Canceled { ts, order }]
            }
            (None, Some(buy_order)) => {
                let order: Order = buy_order.as_ref().clone();
                self.buy_book.remove(buy_order);
                self.buy_index.remove(&id);
                vec![Event::Canceled { ts, order }]
            }
            (None, None) => vec![Event::Rejected {
                ts,
                reason: format!("Order {} not found in sell or buy side", id),
            }],
            (Some(_), Some(_)) => {
                panic!("Bug, order found in both sides");
            }
        }
    }

    fn process_update_order(
        &mut self,
        ts: DateTime<Utc>,
        id: Uuid,
        new_quantity: u32,
        new_price: BigDecimal,
    ) -> Vec<Event> {
        match (self.sell_index.get(&id), self.buy_index.get(&id)) {
            (Some(_), None) => {
                let mut events = self.process_cancel_order(ts, id);
                self.process_sell_order(ts, &mut events, Order::sell(ts, new_quantity, new_price));
                events
            }
            (None, Some(_)) => {
                let mut events = self.process_cancel_order(ts, id);
                self.process_buy_order(ts, &mut events, Order::buy(ts, new_quantity, new_price));
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

    use std::str::FromStr;

    use chrono::Duration;

    use super::*;

    // fn print_order_book(order_book: &OrderBook) {
    //     println!();
    //     for (i, buy) in order_book.buy_book.iter().enumerate() {
    //         println!("{i} - BUY) {:?}", buy);
    //     }
    //     for (i, sell) in order_book.sell_book.iter().enumerate() {
    //         println!("{i} - SELL) {:?}", sell);
    //     }
    //     println!();
    // }

    #[test]
    fn test_order_on_same_price_should_be_ordered_by_earliest() {
        let ts = Utc::now();
        {
            let order1 = Order::buy(ts, 10, 1.into());
            let order2 = Order::buy(ts + Duration::milliseconds(1), 10, 1.into());
            assert_eq!(order1.cmp(&order2), Ordering::Less);
        }
        {
            let order1 = Order::sell(ts, 10, 1.into());
            let order2 = Order::sell(ts + Duration::milliseconds(1), 10, 1.into());
            assert_eq!(order1.cmp(&order2), Ordering::Less);
        }
    }

    #[test]
    fn test_insert_buy() {
        let mut order_book = OrderBook::new("test");
        let events = order_book.process(Command::Buy {
            quantity: 5,
            price: 2.into(),
        });
        let [
            Event::Accepted {
                ts: _,
                order: Order {
                    order_type: OrderType::Buy,
                    id: _,
                    ts: _,
                    quantity: 5, price
                }
            }] = &events[..] else {
            panic!("Wrong event type, events={:?}", events);
        };
        assert_eq!(price, &2.into());
        assert_eq!(order_book.buy_book.len(), 1);
        assert!(order_book.sell_book.is_empty());
    }

    #[test]
    fn test_insert_sell() {
        let mut order_book = OrderBook::new("test");
        let events = order_book.process(Command::Sell {
            quantity: 5,
            price: 2.into(),
        });
        let [
            Event::Accepted {
                ts: _,
                order: Order {
                    order_type: OrderType::Sell,
                    id: _,
                    ts: _,
                    quantity: 5, price
                }
            }] = &events[..] else {
            panic!("Wrong event type, events={:?}", events);
        };
        assert_eq!(price, &2.into());
        assert_eq!(order_book.sell_book.len(), 1);
        assert!(order_book.buy_book.is_empty());
    }

    #[test]
    fn test_reject_cancel_of_non_existing_order() {
        let mut order_book = OrderBook::new("test");
        let events = order_book.process(Command::Cancel { id: Uuid::new_v4() });
        assert_eq!(events.len(), 1);
        assert!(matches!(events.first().unwrap(), Event::Rejected { .. }));
    }

    #[test]
    fn test_cancel_order() {
        let mut order_book = OrderBook::new("test");
        let events = order_book.process(Command::Buy {
            quantity: 5,
            price: 2.into(),
        });
        let [
            Event::Accepted {
                ts: _,
                order: Order {
                    order_type: OrderType::Buy,
                    id,
                    ts:_,
                    quantity:_,
                    price:_
                }
            }] = &events[..] else {
            panic!("Wrong event type, events={:?}", events);
        };
        let events = order_book.process(Command::Cancel { id: id.clone() });
        assert_eq!(events.len(), 1);
        assert!(matches!(events.first().unwrap(), Event::Canceled { .. }));
        assert!(order_book.buy_book.is_empty());
    }

    #[test]
    fn test_update_order() {
        let mut order_book = OrderBook::new("test");
        let events = order_book.process(Command::Buy {
            quantity: 5,
            price: 2.into(),
        });
        let [Event::Accepted { ts:_, order: first_order }] = &events[..] else {
            panic!("Wrong event type, events={:?}", events);
        };
        let events = order_book.process(Command::Update {
            id: first_order.id,
            new_quantity: 10,
            new_price: BigDecimal::from_str("5.5").unwrap(),
        });
        let [Event::Canceled { ts: _, order: first_order }, Event::Accepted { ts:_, order: updated_order }] = &events[..] else {
            panic!("Wrong events={:?}", events);
        };
        assert_ne!(first_order.id, updated_order.id);
        assert_eq!(updated_order.quantity, 10);
        assert_eq!(updated_order.price, BigDecimal::from_str("5.5").unwrap());
        assert_eq!(order_book.buy_book.len(), 1);
    }

    #[test]
    fn test_fill_buy_order_leaving_leftovers() {
        let mut order_book = OrderBook::new("test");
        let events = order_book.process(Command::Buy {
            quantity: 5,
            price: 2.into(),
        });
        let [Event::Accepted { ts: _, order: buy_order}] = &events[..] else {
            panic!("Wrong events={:?}", events);
        };
        assert_eq!(buy_order.order_type, OrderType::Buy);
        let events = order_book.process(Command::Sell {
            quantity: 10,
            price: 2.into(),
        });
        let [
            Event::Accepted {
                ts:_,
                order: sell_order
            },
            Event::Filled {
                ts:_,
                order: filled_order,
                counterpart
            },
            Event::Accepted {
                ts:_,
                order: updated_sell_order
            },
        ] = &events[..] else {
            panic!("Wrong events={:?}", events);
        };
        assert_eq!(filled_order.id, sell_order.id);
        assert_eq!(updated_sell_order.id, sell_order.id);
        assert_eq!(buy_order.id, counterpart.id);
        assert_eq!(order_book.sell_book.len(), 1);
    }

    #[test]
    fn test_fill_sell_order_leaving_leftovers() {
        let mut order_book = OrderBook::new("test");
        let events = order_book.process(Command::Sell {
            quantity: 5,
            price: 2.into(),
        });
        let [Event::Accepted { ts: _, order: buy_order}] = &events[..] else {
            panic!("Wrong events={:?}", events);
        };
        assert_eq!(buy_order.order_type, OrderType::Sell);
        let events = order_book.process(Command::Buy {
            quantity: 10,
            price: 2.into(),
        });
        let [
            Event::Accepted {
                ts:_,
                order: sell_order
            },
            Event::Filled {
                ts:_,
                order: filled_order,
                counterpart
            },
            Event::Accepted {
                ts:_,
                order: updated_sell_order
            },
        ] = &events[..] else {
            panic!("Wrong events={:?}", events);
        };
        assert_eq!(filled_order.id, sell_order.id);
        assert_eq!(updated_sell_order.id, sell_order.id);
        assert_eq!(buy_order.id, counterpart.id);
        assert_eq!(order_book.buy_book.len(), 1);
    }
}
