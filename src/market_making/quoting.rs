//! Quoting engine for market making

use rust_decimal::Decimal;

use crate::execution::venue::Venue;
use crate::execution::order::OrderSide;

use super::orderbook::{OrderBook, OrderBookAnalytics};
use super::inventory::InventoryManager;

/// Quote configuration
#[derive(Debug, Clone)]
pub struct QuoteConfig {
    pub symbol: String,
    pub base_spread_bps: Decimal,      // Base spread in basis points
    pub min_spread_bps: Decimal,       // Minimum allowed spread
    pub max_spread_bps: Decimal,       // Maximum spread
    pub quote_size: Decimal,           // Size per quote
    pub max_orders_per_side: usize,    // Number of price levels
    pub price_precision: Decimal,      // Tick size
    pub size_precision: Decimal,       // Lot size
    pub update_interval_ms: u64,       // Quote refresh rate
}

impl Default for QuoteConfig {
    fn default() -> Self {
        Self {
            symbol: "BTC".to_string(),
            base_spread_bps: Decimal::from(10),  // 10 bps = 0.1%
            min_spread_bps: Decimal::from(5),    // 5 bps min
            max_spread_bps: Decimal::from(100),  // 100 bps max
            quote_size: Decimal::ONE,            // 1 unit
            max_orders_per_side: 3,
            price_precision: Decimal::try_from(0.01).unwrap(), // $0.01
            size_precision: Decimal::try_from(0.001).unwrap(), // 0.001 BTC
            update_interval_ms: 100,
        }
    }
}

/// Generated quote
#[derive(Debug, Clone)]
pub struct Quote {
    pub symbol: String,
    pub side: OrderSide,
    pub price: Decimal,
    pub quantity: Decimal,
    pub venue: Venue,
}

/// Two-sided quote
#[derive(Debug, Clone)]
pub struct TwoSidedQuote {
    pub symbol: String,
    pub bid: Option<Quote>,
    pub ask: Option<Quote>,
    pub spread_bps: Decimal,
    pub mid_price: Decimal,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Quoting engine
#[derive(Debug)]
pub struct QuotingEngine {
    config: QuoteConfig,
    order_book: OrderBook,
}

impl QuotingEngine {
    pub fn new(config: QuoteConfig) -> Self {
        let symbol = config.symbol.clone();
        Self {
            config,
            order_book: OrderBook::new(symbol),
        }
    }
    
    /// Update order book
    pub fn update_order_book(&mut self, book: OrderBook) {
        self.order_book = book;
    }
    
    /// Generate quotes based on market conditions and inventory
    pub fn generate_quotes(&self, inventory: &InventoryManager) -> TwoSidedQuote {
        let mid = self.order_book.mid_price().unwrap_or(Decimal::ZERO);
        
        if mid.is_zero() {
            return TwoSidedQuote {
                symbol: self.config.symbol.clone(),
                bid: None,
                ask: None,
                spread_bps: Decimal::ZERO,
                mid_price: Decimal::ZERO,
                timestamp: chrono::Utc::now(),
            };
        }
        
        // Calculate base spread
        let half_spread = (mid * self.config.base_spread_bps) / Decimal::from(20000);
        
        // Get inventory skew
        let (bid_skew, ask_skew) = inventory.calculate_skew(&self.config.symbol, mid);
        
        // Calculate raw prices
        let raw_bid = mid - half_spread + bid_skew;
        let raw_ask = mid + half_spread + ask_skew;
        
        // Round to precision
        let bid_price = self.round_price(raw_bid);
        let ask_price = self.round_price(raw_ask);
        
        // Ensure minimum spread
        let (bid_price, ask_price) = self.enforce_min_spread(bid_price, ask_price);
        
        // Check inventory limits
        let can_buy = inventory.can_add_position(&self.config.symbol, self.config.quote_size);
        let can_sell = inventory.can_add_position(&self.config.symbol, -self.config.quote_size);
        
        let bid = if can_buy {
            Some(Quote {
                symbol: self.config.symbol.clone(),
                side: OrderSide::Buy,
                price: bid_price,
                quantity: self.round_size(self.config.quote_size),
                venue: Venue::Internal, // Would be actual venue
            })
        } else {
            None
        };
        
        let ask = if can_sell {
            Some(Quote {
                symbol: self.config.symbol.clone(),
                side: OrderSide::Sell,
                price: ask_price,
                quantity: self.round_size(self.config.quote_size),
                venue: Venue::Internal,
            })
        } else {
            None
        };
        
        let spread_bps = if ask_price > Decimal::ZERO {
            ((ask_price - bid_price) / mid) * Decimal::from(10000)
        } else {
            Decimal::ZERO
        };
        
        TwoSidedQuote {
            symbol: self.config.symbol.clone(),
            bid,
            ask,
            spread_bps,
            mid_price: mid,
            timestamp: chrono::Utc::now(),
        }
    }
    
    /// Round price to tick size
    fn round_price(&self, price: Decimal) -> Decimal {
        let tick = self.config.price_precision;
        (price / tick).round() * tick
    }
    
    /// Round size to lot size
    fn round_size(&self, size: Decimal) -> Decimal {
        let lot = self.config.size_precision;
        (size / lot).floor() * lot
    }
    
    /// Ensure minimum spread is maintained
    fn enforce_min_spread(&self, bid: Decimal, ask: Decimal) -> (Decimal, Decimal) {
        let min_spread = (bid * self.config.min_spread_bps) / Decimal::from(10000);
        let current_spread = ask - bid;
        
        if current_spread < min_spread {
            // Widen spread symmetrically
            let adjustment = (min_spread - current_spread) / Decimal::from(2);
            (bid - adjustment, ask + adjustment)
        } else {
            (bid, ask)
        }
    }
    
    /// Generate multi-level quotes (ladder)
    pub fn generate_quote_ladder(
        &self,
        inventory: &InventoryManager,
    ) -> (Vec<Quote>, Vec<Quote>) {
        let base_quote = self.generate_quotes(inventory);
        let mut bids = Vec::new();
        let mut asks = Vec::new();
        
        if let Some(bid) = base_quote.bid {
            bids.push(bid.clone());
            
            // Add deeper bids
            for i in 1..self.config.max_orders_per_side {
                let price_adjustment = self.config.price_precision * Decimal::from(i as i64);
                bids.push(Quote {
                    symbol: bid.symbol.clone(),
                    side: OrderSide::Buy,
                    price: bid.price - price_adjustment,
                    quantity: self.round_size(self.config.quote_size * Decimal::from(i as i64 + 1)),
                    venue: bid.venue.clone(),
                });
            }
        }
        
        if let Some(ask) = base_quote.ask {
            asks.push(ask.clone());
            
            // Add deeper asks
            for i in 1..self.config.max_orders_per_side {
                let price_adjustment = self.config.price_precision * Decimal::from(i as i64);
                asks.push(Quote {
                    symbol: ask.symbol.clone(),
                    side: OrderSide::Sell,
                    price: ask.price + price_adjustment,
                    quantity: self.round_size(self.config.quote_size * Decimal::from(i as i64 + 1)),
                    venue: ask.venue.clone(),
                });
            }
        }
        
        (bids, asks)
    }
    
    /// Check if quote needs update
    pub fn should_update(&self, current: &TwoSidedQuote, new: &TwoSidedQuote) -> bool {
        let price_threshold = self.config.price_precision;
        
        match (&current.bid, &new.bid) {
            (Some(c), Some(n)) if (c.price - n.price).abs() > price_threshold => return true,
            (None, Some(_)) | (Some(_), None) => return true,
            _ => {}
        }
        
        match (&current.ask, &new.ask) {
            (Some(c), Some(n)) if (c.price - n.price).abs() > price_threshold => return true,
            (None, Some(_)) | (Some(_), None) => return true,
            _ => {}
        }
        
        false
    }
}

/// Spread adjustment based on market conditions
#[derive(Debug, Clone)]
pub struct SpreadAdjuster {
    pub volatility_multiplier: Decimal,
    pub inventory_multiplier: Decimal,
    pub time_of_day_multiplier: Decimal,
}

impl Default for SpreadAdjuster {
    fn default() -> Self {
        Self {
            volatility_multiplier: Decimal::ONE,
            inventory_multiplier: Decimal::ONE,
            time_of_day_multiplier: Decimal::ONE,
        }
    }
}

impl SpreadAdjuster {
    /// Calculate adjusted spread
    pub fn adjust_spread(&self, base_spread: Decimal, analytics: &OrderBookAnalytics) -> Decimal {
        // Widen spread in volatile conditions
        let vol_adjustment = if analytics.volatility_estimate > Decimal::from(20) {
            Decimal::try_from(1.5).unwrap()
        } else {
            Decimal::ONE
        };
        
        base_spread * vol_adjustment * self.volatility_multiplier 
            * self.inventory_multiplier * self.time_of_day_multiplier
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::inventory::{InventoryLimits, InventoryManager};
    use chrono::Utc;
    
    fn create_test_orderbook() -> OrderBook {
        let mut book = OrderBook::new("BTC");
        book.update_bid(Decimal::from(49950), Decimal::from(10), 5);
        book.update_ask(Decimal::from(50050), Decimal::from(10), 5);
        book
    }
    
    #[test]
    fn test_quote_generation() {
        let config = QuoteConfig {
            symbol: "BTC".to_string(),
            base_spread_bps: Decimal::from(20), // 20 bps = 0.2%
            ..Default::default()
        };
        
        let mut engine = QuotingEngine::new(config);
        engine.update_order_book(create_test_orderbook());
        
        let limits = InventoryLimits::default();
        let inventory = InventoryManager::new(limits);
        
        let quote = engine.generate_quotes(&inventory);
        
        assert!(quote.bid.is_some());
        assert!(quote.ask.is_some());
        
        let bid = quote.bid.unwrap();
        let ask = quote.ask.unwrap();
        
        // Mid is $50k, 10 bps half-spread = $5
        assert!(bid.price < Decimal::from(50000));
        assert!(ask.price > Decimal::from(50000));
        assert!(ask.price > bid.price);
    }
    
    #[test]
    fn test_spread_adjustment_for_inventory() {
        let config = QuoteConfig {
            symbol: "BTC".to_string(),
            base_spread_bps: Decimal::from(20),
            ..Default::default()
        };
        
        let mut engine = QuotingEngine::new(config);
        engine.update_order_book(create_test_orderbook());
        
        // Create long inventory
        let limits = InventoryLimits {
            max_position: Decimal::from(100),
            skew_factor: Decimal::ONE,
            ..Default::default()
        };
        let mut inventory = InventoryManager::new(limits);
        inventory.record_fill("BTC", Decimal::from(50), Decimal::from(50000), true);
        
        let quote = engine.generate_quotes(&inventory);
        
        // Long inventory should skew quotes down
        if let Some(bid) = quote.bid {
            // Bid should be lower than it would be without inventory
            assert!(bid.price < Decimal::from(49995));
        }
    }
    
    #[test]
    fn test_quote_disabled_when_limit_reached() {
        let config = QuoteConfig {
            symbol: "BTC".to_string(),
            base_spread_bps: Decimal::from(20),
            quote_size: Decimal::from(100),
            ..Default::default()
        };
        
        let mut engine = QuotingEngine::new(config);
        engine.update_order_book(create_test_orderbook());
        
        // At position limit
        let limits = InventoryLimits {
            max_position: Decimal::from(100),
            ..Default::default()
        };
        let mut inventory = InventoryManager::new(limits);
        inventory.record_fill("BTC", Decimal::from(95), Decimal::from(50000), true);
        
        let quote = engine.generate_quotes(&inventory);
        
        // Can't buy more (would exceed limit), but can sell
        assert!(quote.bid.is_none());
        assert!(quote.ask.is_some());
    }
    
    #[test]
    fn test_min_spread_enforcement() {
        let config = QuoteConfig {
            symbol: "BTC".to_string(),
            base_spread_bps: Decimal::from(5),  // Very tight
            min_spread_bps: Decimal::from(20),   // But min is 20 bps
            ..Default::default()
        };
        
        let mut engine = QuotingEngine::new(config);
        engine.update_order_book(create_test_orderbook());
        
        let limits = InventoryLimits::default();
        let inventory = InventoryManager::new(limits);
        
        let quote = engine.generate_quotes(&inventory);
        
        // The prices should respect min spread
        if let (Some(bid), Some(ask)) = (&quote.bid, &quote.ask) {
            let spread_value = ask.price - bid.price;
            let min_spread_value = bid.price * Decimal::try_from(0.002).unwrap();
            assert!(spread_value >= min_spread_value, 
                "Spread ${} is less than minimum ${}", spread_value, min_spread_value);
        }
    }
    
    #[test]
    fn test_quote_ladder() {
        let config = QuoteConfig {
            symbol: "BTC".to_string(),
            max_orders_per_side: 3,
            price_precision: Decimal::ONE, // $1 ticks for simplicity
            ..Default::default()
        };
        
        let mut engine = QuotingEngine::new(config);
        engine.update_order_book(create_test_orderbook());
        
        let limits = InventoryLimits::default();
        let inventory = InventoryManager::new(limits);
        
        let (bids, asks) = engine.generate_quote_ladder(&inventory);
        
        assert_eq!(bids.len(), 3);
        assert_eq!(asks.len(), 3);
        
        // Prices should be ascending
        for i in 1..bids.len() {
            assert!(bids[i].price < bids[i-1].price);
        }
        
        for i in 1..asks.len() {
            assert!(asks[i].price > asks[i-1].price);
        }
    }
    
    #[test]
    fn test_spread_adjuster() {
        let adjuster = SpreadAdjuster::default();
        let analytics = OrderBookAnalytics {
            volatility_estimate: Decimal::from(30), // High vol
            order_flow_imbalance: Decimal::ONE,
            price_momentum: Decimal::ZERO,
            support_level: None,
            resistance_level: None,
        };
        
        let base_spread = Decimal::from(10);
        let adjusted = adjuster.adjust_spread(base_spread, &analytics);
        
        // High volatility should widen spread
        assert!(adjusted > base_spread);
    }
}
