//! Market Making Module - Provide Liquidity & Capture Spread
//!
//! Sprint 20: Market Making Engine
//! - Order book management
//! - Dynamic quote generation
//! - Inventory management
//! - Risk controls
//! - Spread capture analytics

pub mod error;
pub mod inventory;
pub mod orderbook;
pub mod quoting;
pub mod risk;

pub use error::{MarketMakingError, Result};
pub use inventory::{InventoryManager, InventoryPosition, InventoryLimits};
pub use orderbook::{OrderBook, OrderBookAnalytics, PriceLevel};
pub use quoting::{QuotingEngine, Quote, TwoSidedQuote, QuoteConfig, SpreadAdjuster};
pub use risk::{MarketMakingRiskManager, RiskLimits, TradingPause};

use rust_decimal::Decimal;
use std::collections::HashMap;
use tokio::time::{interval, Duration};
use tracing::{info, warn, debug};

/// Market making strategy configuration
#[derive(Debug, Clone)]
pub struct MarketMakingConfig {
    pub symbols: Vec<String>,
    pub quote_config: QuoteConfig,
    pub inventory_limits: InventoryLimits,
    pub risk_limits: RiskLimits,
    pub update_interval_ms: u64,
}

impl Default for MarketMakingConfig {
    fn default() -> Self {
        Self {
            symbols: vec!["BTC".to_string()],
            quote_config: QuoteConfig::default(),
            inventory_limits: InventoryLimits::default(),
            risk_limits: RiskLimits::default(),
            update_interval_ms: 100,
        }
    }
}

/// Market making engine - main entry point
#[derive(Debug)]
pub struct MarketMakingEngine {
    config: MarketMakingConfig,
    quoting_engines: HashMap<String, QuotingEngine>,
    inventory: InventoryManager,
    risk_manager: MarketMakingRiskManager,
    order_books: HashMap<String, OrderBook>,
    running: bool,
    
    // Performance tracking
    quotes_generated: u64,
    quotes_executed: u64,
    total_spread_captured: Decimal,
}

impl MarketMakingEngine {
    /// Create new market making engine
    pub fn new(config: MarketMakingConfig) -> Self {
        let mut quoting_engines = HashMap::new();
        
        for symbol in &config.symbols {
            let mut quote_config = config.quote_config.clone();
            quote_config.symbol = symbol.clone();
            quoting_engines.insert(symbol.clone(), QuotingEngine::new(quote_config));
        }
        
        let inventory = InventoryManager::new(config.inventory_limits.clone());
        let risk_manager = MarketMakingRiskManager::new(config.risk_limits.clone());
        
        Self {
            config,
            quoting_engines,
            inventory,
            risk_manager,
            order_books: HashMap::new(),
            running: false,
            quotes_generated: 0,
            quotes_executed: 0,
            total_spread_captured: Decimal::ZERO,
        }
    }
    
    /// Update order book for symbol
    pub fn update_order_book(&mut self, book: OrderBook) {
        let symbol = book.symbol.clone();
        
        // Update order book
        self.order_books.insert(symbol.clone(), book.clone());
        
        // Update quoting engine
        if let Some(engine) = self.quoting_engines.get_mut(&symbol) {
            engine.update_order_book(book);
        }
    }
    
    /// Generate quotes for all symbols
    pub fn generate_all_quotes(&mut self) -> Vec<TwoSidedQuote> {
        let mut quotes = Vec::new();
        
        // Check if trading is allowed
        if !self.risk_manager.can_trade() {
            warn!("Trading paused due to risk limits");
            return quotes;
        }
        
        let symbols: Vec<_> = self.quoting_engines.keys().cloned().collect();
        for symbol in symbols {
            if let Some(quote) = self.generate_symbol_quotes(&symbol) {
                quotes.push(quote);
            }
        }
        
        self.quotes_generated += quotes.len() as u64;
        quotes
    }
    
    /// Generate quotes for specific symbol
    pub fn generate_symbol_quotes(&mut self, symbol: &str) -> Option<TwoSidedQuote> {
        let engine = self.quoting_engines.get(symbol)?;
        let quote = engine.generate_quotes(&self.inventory);
        
        // Validate with risk manager
        if let Err(e) = self.risk_manager.validate_quote(&quote) {
            warn!("Quote rejected for {}: {}", symbol, e);
            return None;
        }
        
        Some(quote)
    }
    
    /// Record fill from market
    pub fn record_fill(&mut self, symbol: &str, qty: Decimal, price: Decimal, is_buy: bool) {
        // Update inventory
        self.inventory.record_fill(symbol, qty, price, is_buy);
        
        // Update risk manager
        self.risk_manager.record_trade(qty * price);
        
        // Track spread capture
        if let Some(book) = self.order_books.get(symbol) {
            let spread = book.spread().unwrap_or(Decimal::ZERO);
            self.total_spread_captured += spread * qty;
        }
        
        self.quotes_executed += 1;
        
        debug!(
            "Fill recorded: {} {} @ ${} ({})",
            qty, symbol, price, if is_buy { "buy" } else { "sell" }
        );
    }
    
    /// Update mark prices for P&L calculation
    pub fn update_marks(&mut self, marks: &HashMap<String, Decimal>) {
        self.inventory.update_marks(marks);
    }
    
    /// Run market making loop
    pub async fn run(&mut self) -> Result<()> {
        self.running = true;
        let mut interval = interval(Duration::from_millis(self.config.update_interval_ms));
        
        info!("🚀 Market Making Engine started for: {:?}", self.config.symbols);
        
        while self.running {
            interval.tick().await;
            
            // Generate and publish quotes
            let quotes = self.generate_all_quotes();
            
            for quote in quotes {
                if let (Some(bid), Some(ask)) = (&quote.bid, &quote.ask) {
                    info!(
                        "📊 Quote: {} | Bid: {} @ ${} | Ask: {} @ ${} | Spread: {} bps",
                        quote.symbol,
                        bid.quantity, bid.price,
                        ask.quantity, ask.price,
                        quote.spread_bps
                    );
                }
            }
        }
        
        Ok(())
    }
    
    /// Stop the engine
    pub fn stop(&mut self) {
        self.running = false;
        info!("🛑 Market Making Engine stopped");
    }
    
    /// Get current position
    pub fn get_position(&self, symbol: &str) -> Option<&InventoryPosition> {
        self.inventory.get_position(symbol)
    }
    
    /// Get all positions
    pub fn positions(&self) -> &HashMap<String, InventoryPosition> {
        self.inventory.positions()
    }
    
    /// Get P&L summary
    pub fn pnl_summary(&self) -> PnlSummary {
        PnlSummary {
            realized: self.inventory.total_realized_pnl(),
            unrealized: self.inventory.total_unrealized_pnl(),
            total_spread_captured: self.total_spread_captured,
        }
    }
    
    /// Get engine statistics
    pub fn stats(&self) -> MarketMakingStats {
        MarketMakingStats {
            symbols: self.config.symbols.clone(),
            quotes_generated: self.quotes_generated,
            quotes_executed: self.quotes_executed,
            fill_rate: if self.quotes_generated > 0 {
                Decimal::from(self.quotes_executed as i64) / Decimal::from(self.quotes_generated as i64)
            } else {
                Decimal::ZERO
            },
            total_pnl: self.inventory.total_realized_pnl() + self.inventory.total_unrealized_pnl(),
            active: self.running,
        }
    }
}

/// P&L summary
#[derive(Debug, Clone)]
pub struct PnlSummary {
    pub realized: Decimal,
    pub unrealized: Decimal,
    pub total_spread_captured: Decimal,
}

/// Market making statistics
#[derive(Debug, Clone)]
pub struct MarketMakingStats {
    pub symbols: Vec<String>,
    pub quotes_generated: u64,
    pub quotes_executed: u64,
    pub fill_rate: Decimal,
    pub total_pnl: Decimal,
    pub active: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_market_making_engine_creation() {
        let config = MarketMakingConfig::default();
        let engine = MarketMakingEngine::new(config);
        
        assert!(!engine.running);
        assert_eq!(engine.quotes_generated, 0);
    }
    
    #[test]
    fn test_generate_quotes() {
        let config = MarketMakingConfig::default();
        let mut engine = MarketMakingEngine::new(config);
        
        // Setup order book
        let mut book = OrderBook::new("BTC");
        book.update_bid(Decimal::from(49950), Decimal::from(10), 5);
        book.update_ask(Decimal::from(50050), Decimal::from(10), 5);
        engine.update_order_book(book);
        
        // Generate quotes
        let quotes = engine.generate_all_quotes();
        
        assert!(!quotes.is_empty());
        let quote = &quotes[0];
        assert!(quote.bid.is_some());
        assert!(quote.ask.is_some());
    }
    
    #[test]
    fn test_record_fill() {
        let config = MarketMakingConfig::default();
        let mut engine = MarketMakingEngine::new(config);
        
        // Setup order book
        let mut book = OrderBook::new("BTC");
        book.update_bid(Decimal::from(49950), Decimal::from(10), 5);
        book.update_ask(Decimal::from(50050), Decimal::from(10), 5);
        engine.update_order_book(book);
        
        // Record buy fill
        engine.record_fill("BTC", Decimal::ONE, Decimal::from(50000), true);
        
        let pos = engine.get_position("BTC").unwrap();
        assert_eq!(pos.quantity, Decimal::ONE);
        
        // Record sell fill
        engine.record_fill("BTC", Decimal::ONE, Decimal::from(50100), false);
        
        let pos = engine.get_position("BTC").unwrap();
        assert!(pos.quantity.is_zero());
        assert_eq!(pos.realized_pnl, Decimal::from(100));
    }
    
    #[test]
    fn test_pnl_summary() {
        let config = MarketMakingConfig::default();
        let mut engine = MarketMakingEngine::new(config);
        
        // Setup
        let mut book = OrderBook::new("BTC");
        book.update_bid(Decimal::from(49950), Decimal::from(10), 5);
        book.update_ask(Decimal::from(50050), Decimal::from(10), 5);
        engine.update_order_book(book);
        
        // Buy at 50k
        engine.record_fill("BTC", Decimal::ONE, Decimal::from(50000), true);
        
        // Mark at 50.1k
        let mut marks = HashMap::new();
        marks.insert("BTC".to_string(), Decimal::from(50100));
        engine.update_marks(&marks);
        
        // Sell at 50.2k
        engine.record_fill("BTC", Decimal::ONE, Decimal::from(50200), false);
        
        let summary = engine.pnl_summary();
        
        // Realized = (50200 - 50000) = 200
        assert_eq!(summary.realized, Decimal::from(200));
        // Unrealized = 0 (position closed)
        assert!(summary.unrealized.is_zero());
    }
    
    #[tokio::test]
    async fn test_full_market_making_lifecycle() {
        println!("\n📈 Testing Full Market Making Lifecycle");
        
        let config = MarketMakingConfig {
            symbols: vec!["BTC".to_string()],
            quote_config: QuoteConfig {
                symbol: "BTC".to_string(),
                base_spread_bps: Decimal::from(20),
                quote_size: Decimal::from(1),
                ..Default::default()
            },
            ..Default::default()
        };
        
        let mut engine = MarketMakingEngine::new(config);
        
        // 1. Setup market
        let mut book = OrderBook::new("BTC");
        book.update_bid(Decimal::from(49900), Decimal::from(100), 10);
        book.update_ask(Decimal::from(50100), Decimal::from(100), 10);
        engine.update_order_book(book);
        
        println!("✅ Order book initialized");
        
        // 2. Generate initial quotes
        let quotes = engine.generate_all_quotes();
        assert!(!quotes.is_empty());
        
        let quote = &quotes[0];
        println!("📊 Initial quote:");
        if let (Some(bid), Some(ask)) = (&quote.bid, &quote.ask) {
            println!("   Bid: {} @ ${}", bid.quantity, bid.price);
            println!("   Ask: {} @ ${}", ask.quantity, ask.price);
            println!("   Spread: {} bps", quote.spread_bps);
        }
        
        // 3. Simulate fills (someone hits our bid)
        engine.record_fill("BTC", Decimal::from(1), Decimal::from(49980), true);
        println!("💰 Bought 1 BTC @ $49,980");
        
        // 4. Check position
        let pos = engine.get_position("BTC").unwrap();
        println!("📋 Position: {} BTC @ avg ${}", pos.quantity, pos.avg_entry_price);
        
        // 5. Update market (price moves up)
        let mut book = OrderBook::new("BTC");
        book.update_bid(Decimal::from(50000), Decimal::from(100), 10);
        book.update_ask(Decimal::from(50200), Decimal::from(100), 10);
        engine.update_order_book(book);
        
        // 6. Mark to market
        let mut marks = HashMap::new();
        marks.insert("BTC".to_string(), Decimal::from(50200));
        engine.update_marks(&marks);
        
        // 7. Sell at profit
        engine.record_fill("BTC", Decimal::from(1), Decimal::from(50190), false);
        println!("💰 Sold 1 BTC @ $50,190");
        
        // 8. Check P&L
        let summary = engine.pnl_summary();
        let stats = engine.stats();
        
        println!("\n📈 Performance:");
        println!("   Realized P&L: ${}", summary.realized);
        println!("   Quotes generated: {}", stats.quotes_generated);
        println!("   Quotes executed: {}", stats.quotes_executed);
        
        // Profit = 50190 - 49980 = 210
        assert_eq!(summary.realized, Decimal::from(210));
        
        println!("\n✅ Full market making lifecycle completed!");
    }
}
