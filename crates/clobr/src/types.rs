//! Types for the order book

pub type Price = u64;

pub type Qty = u32;

pub type OrderId = u64;

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

// Enriched form of NewOrder, submitted to order book
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RestingOrder {
    pub id: OrderId,
    pub qty: Qty,
    pub side: Side,
    pub price: Price,
}
