//! Order book implementation

use std::collections::{BTreeMap, HashMap, VecDeque};
use std::num::NonZeroU64;
use crate::types::*;

type Result<T> = std::result::Result<T, BookError>;
#[derive(Debug, Clone)]
pub enum BookError {
    ExecutionError(String),
}
impl std::fmt::Display for BookError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "An error occurred when adding an order to the book")
    }
}

pub struct OrderBook {
    bids: BTreeMap<Price, VecDeque<RestingOrder>>,
    asks: BTreeMap<Price, VecDeque<RestingOrder>>,
    orders: HashMap<OrderId, (Side, Price)>,
    next_id: OrderId,
}

impl OrderBook {
    pub fn new() -> Self {
        OrderBook {
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
            orders: HashMap::new(),
            next_id: OrderId(NonZeroU64::MIN),
        }
    }

    pub fn add(&mut self, new_order: NewOrder) -> Result<OrderId> {
        if new_order.qty.is_zero() {
            return Err(BookError::ExecutionError("Quantity must be positive".into()));
        }

        if let OrderType::Limit { price } = new_order.order_type {
            if price.is_zero() {
                return Err(BookError::ExecutionError("Price must be positive".into()));
            }
        }

        let id = self.next_id;
        let next_id = id.get().checked_add(1).ok_or_else(|| {
            BookError::ExecutionError("Order IDs exhausted".into())
        })?;
        self.next_id = OrderId(next_id);

        match new_order.side {
            Side::Buy => self.execute_buy(new_order, id),
            Side::Sell => self.execute_sell(new_order, id),
        }

        Ok(id)
    }

    fn execute_buy(&mut self, new_order: NewOrder, id: OrderId) {
        debug_assert_eq!(new_order.side, Side::Buy);

        let limit_price = match new_order.order_type {
            OrderType::Market => None,
            OrderType::Limit { price } => Some(price),
        };

        let mut remaining = new_order.qty.get();

        while remaining > 0 {
            let Some(mut entry) = self.asks.first_entry() else {
                break;
            };

            let ask_price = *entry.key();

            if limit_price.is_some_and(|limit| ask_price > limit) {
                break;
            }

            let level = entry.get_mut();

            while remaining > 0 {
                let Some(maker) = level.front_mut() else {
                    break;
                };

                let filled = remaining.min(maker.qty.get());
                remaining -= filled;
                maker.qty.subtract(filled);

                if maker.qty.is_zero() {
                    let maker_id = maker.id;
                    level.pop_front();
                    self.orders.remove(&maker_id);
                }
            }

            if level.is_empty() {
                entry.remove_entry();
            }
        }

        if remaining == 0 {
            return;
        }

        let Some(price) = limit_price else {
            return;
        };

        let order = RestingOrder::new(id, Qty(remaining), Side::Buy, price);

        self.bids.entry(price).or_default().push_back(order);
        self.orders.insert(id, (Side::Buy, price));
    }

    fn execute_sell(&mut self, new_order: NewOrder, id: OrderId) {
        debug_assert_eq!(new_order.side, Side::Sell);

        let limit_price = match new_order.order_type {
            OrderType::Market => None,
            OrderType::Limit { price } => Some(price),
        };

        let mut remaining = new_order.qty.get();

        while remaining > 0 {
            let Some(mut entry) = self.bids.last_entry() else {
                break;
            };

            let bid_price = *entry.key();

            if limit_price.is_some_and(|limit| bid_price < limit) {
                break;
            }

            let level = entry.get_mut();

            while remaining > 0 {
                let Some(maker) = level.front_mut() else {
                    break;
                };

                let filled = remaining.min(maker.qty.get());
                remaining -= filled;
                maker.qty.subtract(filled);

                if maker.qty.is_zero() {
                    let maker_id = maker.id;
                    level.pop_front();
                    self.orders.remove(&maker_id);
                }
            }

            if level.is_empty() {
                entry.remove_entry();
            }
        }

        if remaining == 0 {
            return;
        }

        let Some(price) = limit_price else {
            return;
        };

        let order = RestingOrder::new(id, Qty(remaining), Side::Sell, price);

        self.asks.entry(price).or_default().push_back(order);
        self.orders.insert(id, (Side::Sell, price));
    }

    pub fn cancel(&mut self, order_id: OrderId) -> Result<()> {
        let &(side, price) = self.orders.get(&order_id).ok_or_else(|| {
            BookError::ExecutionError("Order ID does not exist".into())
        })?;

        let book = match side {
            Side::Buy => &mut self.bids,
            Side::Sell => &mut self.asks,
        };

        let level = book.get_mut(&price).expect("Indexed price level does not exist");
        let index = level
            .iter()
            .position(|order| order.id == order_id)
            .expect("Indexed order does not exist");

        level.remove(index);
        if level.is_empty() {
            book.remove(&price);
        };

        self.orders.remove(&order_id);
        Ok(())
    }


    pub fn best_bid(&mut self) -> Option<Price> {
        self.bids.last_key_value().map(|(price, _)| *price)
    }

    pub fn best_ask(&mut self) -> Option<Price> {
        self.asks.first_key_value().map(|(price, _)| *price)
    }
}

#[cfg(test)]
mod tests {
    use crate::types::OrderType::Limit;
    use crate::types::Side::*;
    use super::*;

    #[test]
    fn new_orders_increment_order_ids() {
        let mut orderbook = OrderBook::new();
        let order_1 = NewOrder::new(Qty(1), Buy, Limit { price: Price(100)});
        let first_id = orderbook.next_id;
        let order_1_id = orderbook.add(order_1).unwrap();
        assert_eq!(first_id, order_1_id);
        let second_id = OrderId(first_id.get().checked_add(1).unwrap());
        let order_2 = NewOrder::new(Qty(2), Buy, Limit { price: Price(100)});
        let order_2_id = orderbook.add(order_2).unwrap();
        assert_eq!(second_id, order_2_id);
        assert_eq!(orderbook.orders.len(), 2);
        assert_eq!(orderbook.bids[&Price(100)][0].id, order_1_id);
        assert_eq!(orderbook.bids[&Price(100)][1].id, order_2_id);
    }

    #[test]
    fn non_resting_orders_receive_order_ids() {
        for (order_type, ask_qty) in [
            (OrderType::Market, 0),
            (OrderType::Market, 2),
            (OrderType::Market, 3),
            (OrderType::Market, 4),
            (Limit { price: Price(100) }, 3),
        ] {
            let mut book = OrderBook::new();
            if ask_qty > 0 {
                book.add(NewOrder::new(Qty(ask_qty), Sell, Limit { price: Price(100) })).unwrap();
            }

            let expected_id = book.next_id;
            let id = book.add(NewOrder::new(Qty(3), Buy, order_type)).unwrap();

            assert_eq!(id, expected_id);
            assert_eq!(book.next_id.get(), id.get().checked_add(1).unwrap());
            assert!(!book.orders.contains_key(&id));
            assert!(book.bids.is_empty());
            let remaining: u64 = book.asks.values().flatten().map(|o| o.qty.get()).sum();
            assert_eq!(remaining, ask_qty.saturating_sub(3));
        }
    }

    #[test]
    fn invalid_orders_do_not_consume_order_ids() {
        let mut book = OrderBook::new();
        let first_id = book.next_id;
        for side in [Buy, Sell] {
            assert!(book.add(NewOrder::new(Qty(0), side, OrderType::Market)).is_err());
            assert!(book.add(NewOrder::new(Qty(1), side, Limit { price: Price(0) })).is_err());
        }
        assert_eq!(book.next_id, first_id);
    }

    #[test]
    fn exhausted_order_ids_reject_before_matching() {
        let mut book = OrderBook::new();
        let maker_id = book.next_id;
        let maker = RestingOrder::new(maker_id, Qty(3), Sell, Price(100));
        book.asks.entry(Price(100)).or_default().push_back(maker);
        book.orders.insert(maker_id, (Sell, Price(100)));
        book.next_id = OrderId(NonZeroU64::MAX);

        for order_type in [OrderType::Market, Limit { price: Price(100) }] {
            assert!(book.add(NewOrder::new(Qty(3), Buy, order_type)).is_err());
        }
        assert_eq!(book.asks[&Price(100)][0], maker);
        assert_eq!(book.orders.len(), 1);
        assert!(book.bids.is_empty());
        assert_eq!(book.next_id, OrderId(NonZeroU64::MAX));
    }

    #[test]
    fn sell_matches_highest_bid_in_fifo_order() {
        let mut book = OrderBook::new();
        let lower = book.add(NewOrder::new(Qty(4), Buy, Limit { price: Price(100) })).unwrap();
        let first = book.add(NewOrder::new(Qty(2), Buy, Limit { price: Price(101) })).unwrap();
        let second = book.add(NewOrder::new(Qty(5), Buy, Limit { price: Price(101) })).unwrap();

        let sell_id = book.add(NewOrder::new(Qty(6), Sell, Limit { price: Price(100) })).unwrap();

        let best_level = &book.bids[&Price(101)];
        assert_eq!(best_level.len(), 1);
        assert_eq!(best_level[0].id, second);
        assert_eq!(best_level[0].qty, Qty(1));
        assert_eq!(book.bids[&Price(100)][0].qty, Qty(4));
        assert!(book.orders.contains_key(&lower));
        assert!(book.orders.contains_key(&second));
        assert!(!book.orders.contains_key(&first));
        assert!(!book.orders.contains_key(&sell_id));
        assert!(book.asks.is_empty());
    }

    #[test]
    fn sell_limit_sweeps_crossing_bids_and_rests_remainder() {
        let mut book = OrderBook::new();
        let lower = book.add(NewOrder::new(Qty(7), Buy, Limit { price: Price(99) })).unwrap();
        let equal = book.add(NewOrder::new(Qty(2), Buy, Limit { price: Price(100) })).unwrap();
        let higher = book.add(NewOrder::new(Qty(3), Buy, Limit { price: Price(101) })).unwrap();

        let sell_id = book.add(NewOrder::new(Qty(9), Sell, Limit { price: Price(100) })).unwrap();

        assert_eq!(book.bids.len(), 1);
        assert_eq!(book.bids[&Price(99)][0].qty, Qty(7));
        assert_eq!(book.asks[&Price(100)][0], RestingOrder::new(sell_id, Qty(4), Sell, Price(100)));
        assert_eq!(book.orders[&sell_id], (Sell, Price(100)));
        assert!(book.orders.contains_key(&lower));
        assert!(!book.orders.contains_key(&equal));
        assert!(!book.orders.contains_key(&higher));
        assert_eq!(book.orders.len(), 2);
    }

    #[test]
    fn sell_market_never_rests_and_always_receives_an_id() {
        for bid_qty in [0, 2, 3, 4] {
            let mut book = OrderBook::new();
            if bid_qty > 0 {
                book.add(NewOrder::new(Qty(bid_qty), Buy, Limit { price: Price(100) })).unwrap();
            }

            let expected_id = book.next_id;
            let sell_id = book.add(NewOrder::new(Qty(3), Sell, OrderType::Market)).unwrap();

            assert_eq!(sell_id, expected_id);
            assert_eq!(book.next_id.get(), sell_id.get().checked_add(1).unwrap());
            assert!(!book.orders.contains_key(&sell_id));
            assert!(book.asks.is_empty());
            if bid_qty <= 3 {
                assert!(book.bids.is_empty());
                assert!(book.orders.is_empty());
            } else {
                assert_eq!(book.bids[&Price(100)][0].qty, Qty(bid_qty - 3));
                assert_eq!(book.orders.len(), 1);
            }
        }
    }

    #[test]
    fn best_bid_highest_bid_price() {
        let mut book = OrderBook::new();
        for price in [100, 102, 101] {
            book.add(NewOrder::new(Qty(1), Buy, Limit { price: Price(price) })).unwrap();
        }
        assert_eq!(book.best_bid(), Some(Price(102)));
        assert_eq!(book.best_ask(), None);

        book.add(NewOrder::new(Qty(1), Sell, OrderType::Market)).unwrap();
        assert_eq!(book.best_bid(), Some(Price(101)));
    }

    #[test]
    fn best_ask_lowest_ask_price() {
        let mut book = OrderBook::new();
        for price in [102, 100, 101] {
            book.add(NewOrder::new(Qty(1), Sell, Limit { price: Price(price) })).unwrap();
        }
        assert_eq!(book.best_ask(), Some(Price(100)));
        assert_eq!(book.best_bid(), None);

        book.add(NewOrder::new(Qty(1), Buy, OrderType::Market)).unwrap();
        assert_eq!(book.best_ask(), Some(Price(101)));
    }

    #[test]
    fn adding_to_empty_book_sets_best_bid_ask() {
        for first_side in [Buy, Sell] {
            let mut book = OrderBook::new();
            assert_eq!(book.best_bid(), None);
            assert_eq!(book.best_ask(), None);

            let (first_price, second_side, second_price) = match first_side {
                Buy => (100, Sell, 101),
                Sell => (101, Buy, 100),
            };
            book.add(NewOrder::new(Qty(1), first_side, Limit { price: Price(first_price) })).unwrap();
            assert_eq!(book.best_bid(), if first_side == Buy { Some(Price(100)) } else { None });
            assert_eq!(book.best_ask(), if first_side == Sell { Some(Price(101)) } else { None });

            book.add(NewOrder::new(Qty(1), second_side, Limit { price: Price(second_price) })).unwrap();
            assert_eq!(book.best_bid(), Some(Price(100)));
            assert_eq!(book.best_ask(), Some(Price(101)));
        }
    }

    #[test]
    fn two_orders_at_same_price_keep_fifo_order() {
        for side in [Buy, Sell] {
            let mut book = OrderBook::new();
            let first = book.add(NewOrder::new(Qty(2), side, Limit { price: Price(100) })).unwrap();
            let second = book.add(NewOrder::new(Qty(5), side, Limit { price: Price(100) })).unwrap();
            let opposite = match side { Buy => Sell, Sell => Buy };

            book.add(NewOrder::new(Qty(3), opposite, OrderType::Market)).unwrap();

            let levels = match side { Buy => &book.bids, Sell => &book.asks };
            assert_eq!(levels[&Price(100)].len(), 1);
            assert_eq!(levels[&Price(100)][0], RestingOrder::new(second, Qty(4), side, Price(100)));
            assert!(!book.orders.contains_key(&first));
            assert_eq!(book.orders.len(), 1);
            assert_eq!(book.orders[&second], (side, Price(100)));
        }
    }

    #[test]
    fn cancel_removes_order_from_level_and_level_if_empty() {
        for side in [Buy, Sell] {
            let mut book = OrderBook::new();
            let first = book.add(NewOrder::new(Qty(2), side, Limit { price: Price(100) })).unwrap();
            let middle = book.add(NewOrder::new(Qty(3), side, Limit { price: Price(100) })).unwrap();
            let last = book.add(NewOrder::new(Qty(4), side, Limit { price: Price(100) })).unwrap();
            let (other_price, opposite, opposite_price) = match side {
                Buy => (99, Sell, 101),
                Sell => (101, Buy, 99),
            };
            let other = book.add(NewOrder::new(Qty(5), side, Limit { price: Price(other_price) })).unwrap();
            let opposing = book.add(NewOrder::new(Qty(6), opposite, Limit { price: Price(opposite_price) })).unwrap();

            book.cancel(middle).unwrap();
            let levels = match side { Buy => &book.bids, Sell => &book.asks };
            assert_eq!(levels[&Price(100)], VecDeque::from([
                RestingOrder::new(first, Qty(2), side, Price(100)),
                RestingOrder::new(last, Qty(4), side, Price(100)),
            ]));
            assert!(!book.orders.contains_key(&middle));

            book.cancel(first).unwrap();
            book.cancel(last).unwrap();
            let levels = match side { Buy => &book.bids, Sell => &book.asks };
            assert_eq!(levels.len(), 1);
            assert_eq!(levels[&Price(other_price)][0], RestingOrder::new(other, Qty(5), side, Price(other_price)));
            let opposing_levels = match opposite { Buy => &book.bids, Sell => &book.asks };
            assert_eq!(opposing_levels[&Price(opposite_price)][0], RestingOrder::new(opposing, Qty(6), opposite, Price(opposite_price)));
            assert_eq!(book.orders.len(), 2);
            assert_eq!(book.orders[&other], (side, Price(other_price)));
            assert_eq!(book.orders[&opposing], (opposite, Price(opposite_price)));
            assert_eq!(match side { Buy => book.best_bid(), Sell => book.best_ask() }, Some(Price(other_price)));

            book.cancel(other).unwrap();
            assert_eq!(match side { Buy => book.best_bid(), Sell => book.best_ask() }, None);
            book.cancel(opposing).unwrap();
            assert!(book.orders.is_empty());
            assert!(book.bids.is_empty());
            assert!(book.asks.is_empty());
        }
    }

    #[test]
    fn cancel_unknown_id_or_twice_errors() {
        for side in [Buy, Sell] {
            let mut book = OrderBook::new();
            let id = book.add(NewOrder::new(Qty(2), side, Limit { price: Price(100) })).unwrap();
            let unknown = book.next_id;
            let bids_before = book.bids.clone();
            let asks_before = book.asks.clone();
            let orders_before = book.orders.clone();

            assert!(book.cancel(unknown).is_err());
            assert_eq!(book.bids, bids_before);
            assert_eq!(book.asks, asks_before);
            assert_eq!(book.orders, orders_before);

            book.cancel(id).unwrap();
            assert!(book.cancel(id).is_err());
            assert!(book.bids.is_empty());
            assert!(book.asks.is_empty());
            assert!(book.orders.is_empty());
            assert_eq!(book.next_id, unknown);
        }
    }
}
