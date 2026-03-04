//! Order Book Reconstruction

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use std::collections::BTreeMap;
use tracing::{debug, trace};

/// Order book side
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Side {
    Bid,
    Ask,
}

impl Side {
    pub fn opposite(&self) -> Self {
        match self {
            Side::Bid => Side::Ask,
            Side::Ask => Side::Bid,
        }
    }
}

/// Price level in order book
#[derive(Debug, Clone, Copy, Default)]
pub struct PriceLevel {
    pub price: Decimal,
    pub quantity: Decimal,
    pub order_count: u32,
}

impl PriceLevel {
    pub fn new(price: Decimal, quantity: Decimal) -> Self {
        Self {
            price,
            quantity,
            order_count: 1,
        }
    }
}

/// Book update type
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UpdateType {
    Add,
    Remove,
    Modify,
}

/// Order book update
#[derive(Debug, Clone)]
pub struct BookUpdate {
    pub side: Side,
    pub price: Decimal,
    pub quantity: Decimal,
    pub update_type: UpdateType,
    pub timestamp: DateTime<Utc>,
}

/// Real-time order book
#[derive(Debug, Clone)]
pub struct OrderBook {
    symbol: String,
    exchange: String,
    bids: BTreeMap<Decimal, PriceLevel>, // Sorted descending (we'll iterate in reverse)
    asks: BTreeMap<Decimal, PriceLevel>, // Sorted ascending
    last_update: DateTime<Utc>,
    sequence: u64,
}

impl OrderBook {
    /// Create a new order book
    pub fn new(symbol: String, exchange: String) -> Self {
        Self {
            symbol,
            exchange,
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
            last_update: Utc::now(),
            sequence: 0,
        }
    }

    /// Apply book update
    pub fn apply_update(&mut self, update: BookUpdate) {
        self.sequence += 1;
        self.last_update = update.timestamp;

        let book = match update.side {
            Side::Bid => &mut self.bids,
            Side::Ask => &mut self.asks,
        };

        match update.update_type {
            UpdateType::Add | UpdateType::Modify => {
                if update.quantity.is_zero() {
                    book.remove(&update.price);
                } else {
                    book.insert(update.price, PriceLevel::new(update.price, update.quantity));
                }
            }
            UpdateType::Remove => {
                book.remove(&update.price);
            }
        }

        trace!(
            "Applied {} update: {} @ {} {:?}",
            match update.update_type {
                UpdateType::Add => "Add",
                UpdateType::Remove => "Remove",
                UpdateType::Modify => "Modify",
            },
            update.quantity,
            update.price,
            update.side
        );
    }

    /// Apply snapshot
    pub fn apply_snapshot(&mut self, bids: Vec<PriceLevel>, asks: Vec<PriceLevel>) {
        self.bids.clear();
        self.asks.clear();

        for level in bids {
            if !level.quantity.is_zero() {
                self.bids.insert(level.price, level);
            }
        }

        for level in asks {
            if !level.quantity.is_zero() {
                self.asks.insert(level.price, level);
            }
        }

        self.sequence += 1;
        self.last_update = Utc::now();

        debug!(
            "Applied snapshot: {} bids, {} asks",
            self.bids.len(),
            self.asks.len()
        );
    }

    /// Get best bid
    pub fn best_bid(&self) -> Option<&PriceLevel> {
        // BTreeMap iterates in ascending order, so best bid is last
        self.bids.values().next_back()
    }

    /// Get best ask
    pub fn best_ask(&self) -> Option<&PriceLevel> {
        // Best ask is first in ascending order
        self.asks.values().next()
    }

    /// Get mid price
    pub fn mid_price(&self) -> Option<Decimal> {
        let bid = self.best_bid()?.price;
        let ask = self.best_ask()?.price;
        Some((bid + ask) / Decimal::from(2))
    }

    /// Get spread
    pub fn spread(&self) -> Option<Decimal> {
        let bid = self.best_bid()?.price;
        let ask = self.best_ask()?.price;
        Some(ask - bid)
    }

    /// Get spread as percentage
    pub fn spread_pct(&self) -> Option<Decimal> {
        let spread = self.spread()?;
        let mid = self.mid_price()?;
        if !mid.is_zero() {
            Some(spread / mid)
        } else {
            None
        }
    }

    /// Get bid/ask imbalance
    /// Returns (bid_volume, ask_volume, imbalance_ratio)
    /// imbalance_ratio > 0 means more bids (bullish)
    /// imbalance_ratio < 0 means more asks (bearish)
    pub fn get_imbalance(&self, depth: usize) -> (Decimal, Decimal, Decimal) {
        let bid_volume: Decimal = self
            .bids
            .values()
            .rev()
            .take(depth)
            .map(|l| l.quantity)
            .sum();
        let ask_volume: Decimal = self.asks.values().take(depth).map(|l| l.quantity).sum();

        let total = bid_volume + ask_volume;
        let imbalance = if !total.is_zero() {
            (bid_volume - ask_volume) / total
        } else {
            Decimal::ZERO
        };

        (bid_volume, ask_volume, imbalance)
    }

    /// Get volume at price
    pub fn volume_at_price(&self, price: Decimal) -> Decimal {
        if let Some(level) = self.bids.get(&price) {
            level.quantity
        } else if let Some(level) = self.asks.get(&price) {
            level.quantity
        } else {
            Decimal::ZERO
        }
    }

    /// Get bids (sorted descending by price)
    pub fn get_bids(&self, limit: usize) -> Vec<&PriceLevel> {
        self.bids.values().rev().take(limit).collect()
    }

    /// Get asks (sorted ascending by price)
    pub fn get_asks(&self, limit: usize) -> Vec<&PriceLevel> {
        self.asks.values().take(limit).collect()
    }

    /// Calculate weighted average price for a given quantity
    /// side: Side::Bid = buying (hit the asks), Side::Ask = selling (hit the bids)
    pub fn vwap(&self, side: Side, quantity: Decimal) -> Option<Decimal> {
        // When buying, we hit the asks (lowest prices first)
        // When selling, we hit the bids (highest prices first)
        let book = match side {
            Side::Bid => &self.asks,
            Side::Ask => &self.bids,
        };

        let mut remaining = quantity;
        let mut total_value = Decimal::ZERO;
        let mut total_qty = Decimal::ZERO;

        // Iterate in proper order
        // For asks (buying), we want lowest prices first (ascending - natural BTreeMap order)
        // For bids (selling), we want highest bids first (descending, so reverse)
        let levels: Vec<_> = match side {
            Side::Bid => book.values().collect(), // Asks in ascending order - lowest first for buying
            Side::Ask => book.values().rev().collect(), // Bids in ascending order - reverse for highest first
        };

        for level in levels {
            let take_qty = remaining.min(level.quantity);
            total_value += take_qty * level.price;
            total_qty += take_qty;
            remaining -= take_qty;

            if remaining.is_zero() {
                break;
            }
        }

        if !total_qty.is_zero() {
            Some(total_value / total_qty)
        } else {
            None
        }
    }

    /// Calculate market impact for an order
    pub fn market_impact(&self, side: Side, quantity: Decimal) -> Option<Decimal> {
        let entry_price = match side {
            Side::Bid => self.best_ask()?.price, // Buying at ask
            Side::Ask => self.best_bid()?.price, // Selling at bid
        };

        let exec_price = self.vwap(side, quantity)?;

        let impact = (exec_price - entry_price).abs() / entry_price;
        Some(impact)
    }

    /// Get book depth
    pub fn depth(&self) -> (usize, usize) {
        (self.bids.len(), self.asks.len())
    }

    /// Get total volume at each level of depth
    pub fn cumulative_volume(&self, levels: usize) -> (Decimal, Decimal) {
        let bid_vol: Decimal = self
            .bids
            .values()
            .rev()
            .take(levels)
            .map(|l| l.quantity)
            .sum();
        let ask_vol: Decimal = self.asks.values().take(levels).map(|l| l.quantity).sum();
        (bid_vol, ask_vol)
    }

    /// Check if book is crossed (best bid >= best ask)
    pub fn is_crossed(&self) -> bool {
        if let (Some(bid), Some(ask)) = (self.best_bid(), self.best_ask()) {
            bid.price >= ask.price
        } else {
            false
        }
    }

    /// Get last update time
    pub fn last_update(&self) -> DateTime<Utc> {
        self.last_update
    }

    /// Get sequence number
    pub fn sequence(&self) -> u64 {
        self.sequence
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_book() -> OrderBook {
        let mut book = OrderBook::new("BTCUSDT".to_string(), "binance".to_string());

        // Add bids (higher prices are better)
        for i in (1..=5).rev() {
            book.apply_update(BookUpdate {
                side: Side::Bid,
                price: Decimal::from(100 + i * 10),
                quantity: Decimal::from(i),
                update_type: UpdateType::Add,
                timestamp: Utc::now(),
            });
        }

        // Add asks
        for i in 1..=5 {
            book.apply_update(BookUpdate {
                side: Side::Ask,
                price: Decimal::from(160 + i * 10),
                quantity: Decimal::from(i),
                update_type: UpdateType::Add,
                timestamp: Utc::now(),
            });
        }

        book
    }

    #[test]
    fn test_best_bid_ask() {
        let book = create_test_book();

        assert_eq!(book.best_bid().unwrap().price, Decimal::from(150));
        assert_eq!(book.best_ask().unwrap().price, Decimal::from(170));
    }

    #[test]
    fn test_mid_price() {
        let book = create_test_book();

        let mid = book.mid_price().unwrap();
        assert_eq!(mid, Decimal::from(160));
    }

    #[test]
    fn test_spread() {
        let book = create_test_book();

        let spread = book.spread().unwrap();
        assert_eq!(spread, Decimal::from(20));

        let spread_pct = book.spread_pct().unwrap();
        assert_eq!(spread_pct, Decimal::try_from(0.125).unwrap());
    }

    #[test]
    fn test_imbalance() {
        let book = create_test_book();

        let (bid_vol, ask_vol, imbalance) = book.get_imbalance(5);

        // Bid volumes: 5,4,3,2,1 = 15
        // Ask volumes: 1,2,3,4,5 = 15
        assert_eq!(bid_vol, Decimal::from(15));
        assert_eq!(ask_vol, Decimal::from(15));
        assert_eq!(imbalance, Decimal::ZERO);
    }

    #[test]
    fn test_vwap() {
        let book = create_test_book();

        // VWAP for buying 2 units
        // Takes 1 @ 170 + 1 @ 180 = 350 / 2 = 175
        let vwap = book.vwap(Side::Bid, Decimal::from(2)).unwrap();
        assert_eq!(vwap, Decimal::from(175));
    }

    #[test]
    fn test_apply_update() {
        let mut book = create_test_book();

        // Modify a level
        book.apply_update(BookUpdate {
            side: Side::Bid,
            price: Decimal::from(150),
            quantity: Decimal::from(100),
            update_type: UpdateType::Modify,
            timestamp: Utc::now(),
        });

        assert_eq!(book.best_bid().unwrap().quantity, Decimal::from(100));

        // Remove a level
        book.apply_update(BookUpdate {
            side: Side::Bid,
            price: Decimal::from(150),
            quantity: Decimal::ZERO,
            update_type: UpdateType::Remove,
            timestamp: Utc::now(),
        });

        assert_eq!(book.best_bid().unwrap().price, Decimal::from(140));
    }

    #[test]
    fn test_is_crossed() {
        let mut book = create_test_book();
        assert!(!book.is_crossed());

        // Cross the book
        book.apply_update(BookUpdate {
            side: Side::Bid,
            price: Decimal::from(180),
            quantity: Decimal::from(1),
            update_type: UpdateType::Add,
            timestamp: Utc::now(),
        });

        assert!(book.is_crossed());
    }

    #[test]
    fn test_snapshot() {
        let mut book = OrderBook::new("BTCUSDT".to_string(), "binance".to_string());

        let bids = vec![
            PriceLevel::new(Decimal::from(100), Decimal::from(10)),
            PriceLevel::new(Decimal::from(99), Decimal::from(20)),
        ];
        let asks = vec![
            PriceLevel::new(Decimal::from(101), Decimal::from(15)),
            PriceLevel::new(Decimal::from(102), Decimal::from(25)),
        ];

        book.apply_snapshot(bids, asks);

        assert_eq!(book.best_bid().unwrap().price, Decimal::from(100));
        assert_eq!(book.best_ask().unwrap().price, Decimal::from(101));
    }
}
