//! Order book model and analysis

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Price level in order book
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceLevel {
    pub price: Decimal,
    pub quantity: Decimal,
    pub order_count: usize,
}

/// Order book representation
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OrderBook {
    pub symbol: String,
    pub bids: BTreeMap<Decimal, PriceLevel>, // Price -> Level (sorted descending)
    pub asks: BTreeMap<Decimal, PriceLevel>, // Price -> Level (sorted ascending)
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub sequence: u64,
}

impl OrderBook {
    /// Create new empty order book
    pub fn new(symbol: impl Into<String>) -> Self {
        Self {
            symbol: symbol.into(),
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
            timestamp: chrono::Utc::now(),
            sequence: 0,
        }
    }
    
    /// Get best bid (highest)
    pub fn best_bid(&self) -> Option<&PriceLevel> {
        self.bids.values().last()
    }
    
    /// Get best ask (lowest)
    pub fn best_ask(&self) -> Option<&PriceLevel> {
        self.asks.values().next()
    }
    
    /// Get mid price
    pub fn mid_price(&self) -> Option<Decimal> {
        match (self.best_bid(), self.best_ask()) {
            (Some(bid), Some(ask)) => Some((bid.price + ask.price) / Decimal::from(2)),
            _ => None,
        }
    }
    
    /// Get spread
    pub fn spread(&self) -> Option<Decimal> {
        match (self.best_bid(), self.best_ask()) {
            (Some(bid), Some(ask)) => Some(ask.price - bid.price),
            _ => None,
        }
    }
    
    /// Get spread in basis points
    pub fn spread_bps(&self) -> Option<Decimal> {
        match (self.spread(), self.mid_price()) {
            (Some(spread), Some(mid)) if !mid.is_zero() => {
                Some((spread / mid) * Decimal::from(10000))
            }
            _ => None,
        }
    }
    
    /// Update bid level
    pub fn update_bid(&mut self, price: Decimal, quantity: Decimal, order_count: usize) {
        if quantity.is_zero() {
            self.bids.remove(&price);
        } else {
            self.bids.insert(price, PriceLevel { price, quantity, order_count });
        }
        self.sequence += 1;
        self.timestamp = chrono::Utc::now();
    }
    
    /// Update ask level
    pub fn update_ask(&mut self, price: Decimal, quantity: Decimal, order_count: usize) {
        if quantity.is_zero() {
            self.asks.remove(&price);
        } else {
            self.asks.insert(price, PriceLevel { price, quantity, order_count });
        }
        self.sequence += 1;
        self.timestamp = chrono::Utc::now();
    }
    
    /// Get liquidity at price level
    pub fn get_liquidity(&self, price: Decimal, side: crate::execution::order::OrderSide) -> Decimal {
        let levels = match side {
            crate::execution::order::OrderSide::Buy => &self.asks,
            crate::execution::order::OrderSide::Sell => &self.bids,
        };
        
        levels.get(&price).map(|l| l.quantity).unwrap_or(Decimal::ZERO)
    }
    
    /// Calculate weighted average price for a given quantity
    pub fn vwap(&self, quantity: Decimal, side: crate::execution::order::OrderSide) -> Option<Decimal> {
        let levels: Vec<_> = match side {
            crate::execution::order::OrderSide::Buy => {
                self.asks.values().cloned().collect()
            }
            crate::execution::order::OrderSide::Sell => {
                self.bids.values().rev().cloned().collect()
            }
        };
        
        let mut remaining = quantity;
        let mut total_cost = Decimal::ZERO;
        
        for level in levels {
            let take = remaining.min(level.quantity);
            total_cost += take * level.price;
            remaining -= take;
            
            if remaining.is_zero() {
                break;
            }
        }
        
        if remaining > Decimal::ZERO {
            return None; // Insufficient liquidity
        }
        
        Some(total_cost / quantity)
    }
    
    /// Calculate market depth (total quantity within price range)
    pub fn depth(&self, price_range_pct: Decimal) -> (Decimal, Decimal) {
        let mid = self.mid_price().unwrap_or(Decimal::ZERO);
        if mid.is_zero() {
            return (Decimal::ZERO, Decimal::ZERO);
        }
        
        let range = mid * price_range_pct;
        
        let bid_depth: Decimal = self.bids
            .iter()
            .filter(|(price, _)| **price >= mid - range)
            .map(|(_, level)| level.quantity)
            .sum();
        
        let ask_depth: Decimal = self.asks
            .iter()
            .filter(|(price, _)| **price <= mid + range)
            .map(|(_, level)| level.quantity)
            .sum();
        
        (bid_depth, ask_depth)
    }
    
    /// Check if book is balanced (similar depth on both sides)
    pub fn imbalance_ratio(&self) -> Option<Decimal> {
        let (bid_depth, ask_depth) = self.depth(Decimal::try_from(0.01).unwrap());
        
        if ask_depth.is_zero() {
            return None;
        }
        
        Some(bid_depth / ask_depth)
    }
}

/// Order book analytics
#[derive(Debug, Clone)]
pub struct OrderBookAnalytics {
    pub volatility_estimate: Decimal,
    pub order_flow_imbalance: Decimal,
    pub price_momentum: Decimal,
    pub support_level: Option<Decimal>,
    pub resistance_level: Option<Decimal>,
}

impl OrderBookAnalytics {
    /// Analyze order book for trading signals
    pub fn analyze(book: &OrderBook) -> Self {
        // Simplified analytics - in production would use more sophisticated models
        let imbalance = book.imbalance_ratio().unwrap_or(Decimal::ONE);
        
        Self {
            volatility_estimate: book.spread_bps().unwrap_or(Decimal::from(10)),
            order_flow_imbalance: imbalance,
            price_momentum: Decimal::ZERO, // Would need historical data
            support_level: book.bids.keys().next().copied(),
            resistance_level: book.asks.keys().next().copied(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution::order::OrderSide;
    
    #[test]
    fn test_order_book_creation() {
        let book = OrderBook::new("BTC");
        
        assert_eq!(book.symbol, "BTC");
        assert!(book.best_bid().is_none());
        assert!(book.best_ask().is_none());
    }
    
    #[test]
    fn test_update_levels() {
        let mut book = OrderBook::new("BTC");
        
        book.update_bid(Decimal::from(50000), Decimal::from(10), 5);
        book.update_ask(Decimal::from(50100), Decimal::from(5), 3);
        
        assert_eq!(book.best_bid().unwrap().price, Decimal::from(50000));
        assert_eq!(book.best_ask().unwrap().price, Decimal::from(50100));
        assert_eq!(book.spread(), Some(Decimal::from(100)));
    }
    
    #[test]
    fn test_mid_price() {
        let mut book = OrderBook::new("BTC");
        
        book.update_bid(Decimal::from(50000), Decimal::from(10), 1);
        book.update_ask(Decimal::from(50200), Decimal::from(5), 1);
        
        let mid = book.mid_price().unwrap();
        assert_eq!(mid, Decimal::from(50100));
    }
    
    #[test]
    fn test_vwap_calculation() {
        let mut book = OrderBook::new("BTC");
        
        // Asks: 100 @ $50k, 200 @ $50.1k
        book.update_ask(Decimal::from(50000), Decimal::from(100), 1);
        book.update_ask(Decimal::from(50100), Decimal::from(200), 1);
        
        // Buy 150 units
        let vwap = book.vwap(Decimal::from(150), OrderSide::Buy).unwrap();
        
        // (100 * 50000 + 50 * 50100) / 150 = 50033.33
        let expected = (Decimal::from(100) * Decimal::from(50000) 
            + Decimal::from(50) * Decimal::from(50100)) / Decimal::from(150);
        assert_eq!(vwap, expected);
    }
    
    #[test]
    fn test_depth_calculation() {
        let mut book = OrderBook::new("BTC");
        
        book.update_bid(Decimal::from(49900), Decimal::from(100), 1);
        book.update_bid(Decimal::from(49800), Decimal::from(200), 1);
        book.update_ask(Decimal::from(50100), Decimal::from(50), 1);
        book.update_ask(Decimal::from(50200), Decimal::from(150), 1);
        
        let (bid_depth, ask_depth) = book.depth(Decimal::try_from(0.01).unwrap());
        
        // With mid ~50000, 1% range = 500
        // Bids >= 49500: both levels = 300
        // Asks <= 50500: both levels = 200
        assert!(bid_depth > Decimal::ZERO);
        assert!(ask_depth > Decimal::ZERO);
    }
    
    #[test]
    fn test_imbalance_ratio() {
        let mut book = OrderBook::new("BTC");
        
        book.update_bid(Decimal::from(50000), Decimal::from(1000), 1);
        book.update_ask(Decimal::from(50100), Decimal::from(500), 1);
        
        let imbalance = book.imbalance_ratio().unwrap();
        // 1000 / 500 = 2.0
        assert_eq!(imbalance, Decimal::from(2));
    }
    
    #[test]
    fn test_remove_zero_quantity() {
        let mut book = OrderBook::new("BTC");
        
        book.update_bid(Decimal::from(50000), Decimal::from(10), 1);
        assert!(book.best_bid().is_some());
        
        // Remove by setting quantity to 0
        book.update_bid(Decimal::from(50000), Decimal::ZERO, 0);
        assert!(book.best_bid().is_none());
    }
}
