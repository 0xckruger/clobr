//! Types for the order book

use std::fmt::{Display, Formatter};
use std::num::NonZeroU64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Price(pub u64);

impl Price {
    pub(crate) fn is_zero(&self) -> bool {
        self.0 == 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Qty(pub u64);

impl Qty {
    pub(crate) fn is_zero(&self) -> bool {
        self.0 == 0
    }

    pub(crate) fn get(&self) -> u64 {
        self.0
    }

    pub(crate) fn subtract(&mut self, x: u64) {
        self.0 -= x
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OrderId(pub NonZeroU64);

impl Display for OrderId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl OrderId {
    pub(crate) fn get(&self) -> NonZeroU64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum OrderType {
    Limit { price: Price },
    Market,
}

// Submitted by a client
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NewOrder {
    pub qty: Qty,
    pub side: Side,
    pub order_type: OrderType,
}

impl NewOrder {
    pub fn new(qty: Qty, side: Side, order_type: OrderType) -> NewOrder {
        NewOrder {
            qty,
            side,
            order_type,
        }
    }
}

// Enriched form of NewOrder, submitted to order book
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RestingOrder {
    pub id: OrderId,
    pub qty: Qty,
    pub side: Side,
    pub price: Price,
}

impl RestingOrder {
    pub fn new(id: OrderId, qty: Qty, side: Side, price: Price) -> RestingOrder {
        RestingOrder {
            id,
            qty,
            side,
            price,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fill {
    pub new_order_id: OrderId,
    pub resting_order_id: OrderId,
    pub price: Price,
    pub qty: Qty,
}

impl Fill {
    pub fn new(new_order_id: OrderId, resting_order_id: OrderId, price: Price, qty: Qty) -> Fill {
        Fill {
            new_order_id,
            resting_order_id,
            price,
            qty,
        }
    }
}
