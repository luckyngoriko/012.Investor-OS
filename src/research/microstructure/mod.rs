//! Market Microstructure Analysis Module
//!
//! Analyzes order book dynamics, flow toxicity, and liquidity.
//! Implements VPIN (Volume-synchronized Probability of Informed Trading)
//! and other microstructure metrics.

use rust_decimal::Decimal;
use std::collections::VecDeque;
use chrono::{DateTime, Utc};
use thiserror::Error;

/// Microstructure analysis errors
#[derive(Error, Debug, Clone)]
pub enum MicrostructureError {
    #[error("Insufficient order book data")]
    InsufficientData,
    
    #[error("Invalid price level: {0}")]
    InvalidPriceLevel(String),
    
    #[error("Calculation error: {0}")]
    CalculationError(String),
}

/// Order book level
#[derive(Debug, Clone, Copy)]
pub struct OrderBookLevel {
    pub price: Decimal,
    pub quantity: Decimal,
    pub num_orders: u32,
}

impl OrderBookLevel {
    pub fn new(price: Decimal, quantity: Decimal) -> Self {
        Self {
            price,
            quantity,
            num_orders: 1,
        }
    }
    
    /// Get value at this level (price * quantity)
    pub fn value(&self) -> Decimal {
        self.price * self.quantity
    }
}

/// Full order book snapshot
#[derive(Debug, Clone)]
pub struct OrderBook {
    pub symbol: String,
    pub timestamp: DateTime<Utc>,
    pub bids: Vec<OrderBookLevel>, // Sorted descending
    pub asks: Vec<OrderBookLevel>, // Sorted ascending
    pub last_trade_price: Option<Decimal>,
    pub last_trade_size: Option<Decimal>,
}

impl OrderBook {
    /// Create empty order book
    pub fn new(symbol: impl Into<String>) -> Self {
        Self {
            symbol: symbol.into(),
            timestamp: Utc::now(),
            bids: Vec::new(),
            asks: Vec::new(),
            last_trade_price: None,
            last_trade_size: None,
        }
    }
    
    /// Get best bid
    pub fn best_bid(&self) -> Option<&OrderBookLevel> {
        self.bids.first()
    }
    
    /// Get best ask
    pub fn best_ask(&self) -> Option<&OrderBookLevel> {
        self.asks.first()
    }
    
    /// Get mid price
    pub fn mid_price(&self) -> Option<Decimal> {
        match (self.best_bid(), self.best_ask()) {
            (Some(bid), Some(ask)) => {
                Some((bid.price + ask.price) / Decimal::from(2))
            }
            _ => None,
        }
    }
    
    /// Get spread
    pub fn spread(&self) -> Option<Decimal> {
        match (self.best_ask(), self.best_bid()) {
            (Some(ask), Some(bid)) => {
                Some(ask.price - bid.price)
            }
            _ => None,
        }
    }
    
    /// Get spread in basis points
    pub fn spread_bps(&self) -> Option<f64> {
        match (self.spread(), self.mid_price()) {
            (Some(spread), Some(mid)) if mid > Decimal::ZERO => {
                Some((spread / mid * Decimal::from(10000)).try_into().unwrap_or(0.0))
            }
            _ => None,
        }
    }
    
    /// Get total bid volume
    pub fn total_bid_volume(&self) -> Decimal {
        self.bids.iter().map(|b| b.quantity).sum()
    }
    
    /// Get total ask volume
    pub fn total_ask_volume(&self) -> Decimal {
        self.asks.iter().map(|a| a.quantity).sum()
    }
    
    /// Get order book imbalance (-1 to 1, positive = more bids)
    pub fn imbalance(&self) -> f64 {
        let bid_vol = self.total_bid_volume();
        let ask_vol = self.total_ask_volume();
        let total = bid_vol + ask_vol;
        
        if total == Decimal::ZERO {
            0.0
        } else {
            ((bid_vol - ask_vol) / total).try_into().unwrap_or(0.0)
        }
    }
    
    /// Calculate weighted average price for given volume
    pub fn vwap(&self, side: Side, target_volume: Decimal) -> Option<Decimal> {
        let levels = match side {
            Side::Buy => &self.asks,
            Side::Sell => &self.bids,
        };
        
        let mut remaining = target_volume;
        let mut total_value = Decimal::ZERO;
        
        for level in levels {
            let take = remaining.min(level.quantity);
            total_value += take * level.price;
            remaining -= take;
            
            if remaining <= Decimal::ZERO {
                break;
            }
        }
        
        if remaining > Decimal::ZERO {
            // Not enough liquidity
            None
        } else {
            Some(total_value / target_volume)
        }
    }
    
    /// Calculate market depth at different levels
    pub fn depth(&self, levels: usize) -> (Decimal, Decimal) {
        let bid_depth: Decimal = self.bids.iter().take(levels).map(|b| b.quantity).sum();
        let ask_depth: Decimal = self.asks.iter().take(levels).map(|a| a.quantity).sum();
        (bid_depth, ask_depth)
    }
}

/// Side of order/trade
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Side {
    Buy,
    Sell,
}

/// Trade tick
#[derive(Debug, Clone)]
pub struct TradeTick {
    pub timestamp: DateTime<Utc>,
    pub price: Decimal,
    pub quantity: Decimal,
    pub side: Side,
    pub is_buyer_maker: bool, // True if buyer was maker (seller aggressive)
}

/// Volume bar (bucket)
#[derive(Debug, Clone)]
pub struct VolumeBar {
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub volume: Decimal,
    pub buy_volume: Decimal,
    pub sell_volume: Decimal,
    pub high: Decimal,
    pub low: Decimal,
    pub open: Decimal,
    pub close: Decimal,
    pub num_trades: u32,
}

/// VPIN Calculator
#[derive(Debug)]
pub struct VpinCalculator {
    /// Volume bucket size
    bucket_volume: Decimal,
    /// Number of buckets to use
    num_buckets: usize,
    /// Historical volume bars
    volume_bars: VecDeque<VolumeBar>,
}

impl VpinCalculator {
    /// Create new VPIN calculator
    pub fn new(bucket_volume: Decimal, num_buckets: usize) -> Self {
        Self {
            bucket_volume,
            num_buckets,
            volume_bars: VecDeque::with_capacity(num_buckets),
        }
    }
    
    /// Add trade to calculator
    pub fn add_trade(&mut self, _trade: &TradeTick) {
        // In production: aggregate trades into volume bars
        // For now, simplified
    }
    
    /// Calculate VPIN (Volume-synchronized Probability of Informed Trading)
    /// 
    /// VPIN measures the probability of informed trading based on order flow imbalance.
    /// High VPIN indicates toxic flow and potential adverse selection.
    pub fn calculate_vpin(&self) -> Result<f64, MicrostructureError> {
        if self.volume_bars.len() < self.num_buckets / 2 {
            return Err(MicrostructureError::InsufficientData);
        }
        
        // VPIN = average absolute order flow imbalance
        let mut total_imbalance = 0.0;
        let mut count = 0;
        
        for bar in &self.volume_bars {
            let imbalance = if bar.volume > Decimal::ZERO {
                ((bar.buy_volume - bar.sell_volume).abs() / bar.volume)
                    .try_into()
                    .unwrap_or(0.0)
            } else {
                0.0
            };
            
            total_imbalance += imbalance;
            count += 1;
        }
        
        if count == 0 {
            return Err(MicrostructureError::CalculationError(
                "No valid volume bars".to_string()
            ));
        }
        
        Ok(total_imbalance / count as f64)
    }
}

impl Default for VpinCalculator {
    fn default() -> Self {
        Self::new(Decimal::from(10_000), 50) // 10k volume buckets
    }
}

/// Liquidity metrics
#[derive(Debug, Clone)]
pub struct LiquidityMetrics {
    /// Bid-ask spread in basis points
    pub spread_bps: f64,
    /// Order book depth (total volume within 5 levels)
    pub depth_5_levels: Decimal,
    /// Kyle's lambda (price impact coefficient)
    pub kyle_lambda: f64,
    /// Amihud illiquidity ratio
    pub amihud_ratio: f64,
    /// Time to liquidate position (seconds)
    pub time_to_liquidate: f64,
}

/// Adverse selection estimator
#[derive(Debug)]
pub struct AdverseSelectionEstimator {
    /// Price changes after trades
    price_changes: VecDeque<(DateTime<Utc>, Decimal)>,
    /// Trade directions
    trade_directions: VecDeque<bool>,
    /// Lookback window
    window_size: usize,
}

impl AdverseSelectionEstimator {
    /// Create new estimator
    pub fn new(window_size: usize) -> Self {
        Self {
            price_changes: VecDeque::with_capacity(window_size),
            trade_directions: VecDeque::with_capacity(window_size),
            window_size,
        }
    }
    
    /// Add trade observation
    pub fn add_observation(&mut self, is_buy: bool, price_change: Decimal) {
        self.trade_directions.push_back(is_buy);
        self.price_changes.push_back((Utc::now(), price_change));
        
        while self.price_changes.len() > self.window_size {
            self.price_changes.pop_front();
            self.trade_directions.pop_front();
        }
    }
    
    /// Estimate adverse selection cost (in basis points)
    ///
    /// Measures how much prices move against the trader after execution.
    /// High values indicate informed trading.
    pub fn estimate_cost(&self) -> Result<f64, MicrostructureError> {
        if self.price_changes.len() < 10 {
            return Err(MicrostructureError::InsufficientData);
        }
        
        // Calculate average price change after buy vs sell trades
        let mut buy_changes = Vec::new();
        let mut sell_changes = Vec::new();
        
        for (i, (_, change)) in self.price_changes.iter().enumerate() {
            if let Some(&is_buy) = self.trade_directions.get(i) {
                if is_buy {
                    buy_changes.push((*change).try_into().unwrap_or(0.0));
                } else {
                    sell_changes.push((*change).try_into().unwrap_or(0.0));
                }
            }
        }
        
        let avg_buy: f64 = buy_changes.iter().sum::<f64>() / buy_changes.len().max(1) as f64;
        let avg_sell: f64 = sell_changes.iter().sum::<f64>() / sell_changes.len().max(1) as f64;
        
        // Adverse selection cost = half the spread + price impact
        let cost = ((avg_buy - avg_sell) / 2.0).abs();
        
        Ok(cost * 10_000.0) // Convert to basis points
    }
}

/// Market microstructure analyzer
#[derive(Debug)]
pub struct MicrostructureAnalyzer {
    order_books: std::collections::HashMap<String, OrderBook>,
    vpin_calculators: std::collections::HashMap<String, VpinCalculator>,
    adverse_sel_estimators: std::collections::HashMap<String, AdverseSelectionEstimator>,
}

impl MicrostructureAnalyzer {
    /// Create new analyzer
    pub fn new() -> Self {
        Self {
            order_books: std::collections::HashMap::new(),
            vpin_calculators: std::collections::HashMap::new(),
            adverse_sel_estimators: std::collections::HashMap::new(),
        }
    }
    
    /// Update order book for symbol
    pub fn update_order_book(&mut self, symbol: &str, book: OrderBook) {
        self.order_books.insert(symbol.to_string(), book);
    }
    
    /// Add trade
    pub fn add_trade(&mut self, symbol: &str, trade: TradeTick) {
        let calc = self.vpin_calculators
            .entry(symbol.to_string())
            .or_default();
        
        calc.add_trade(&trade);
        
        let estimator = self.adverse_sel_estimators
            .entry(symbol.to_string())
            .or_insert_with(|| AdverseSelectionEstimator::new(100));
        
        estimator.add_observation(
            trade.side == Side::Buy,
            Decimal::ZERO, // Would calculate actual price change
        );
    }
    
    /// Get liquidity metrics for symbol
    pub fn get_liquidity_metrics(&self, symbol: &str) -> Option<LiquidityMetrics> {
        let book = self.order_books.get(symbol)?;
        
        let spread_bps = book.spread_bps()?;
        let (bid_depth, ask_depth) = book.depth(5);
        let depth_5_levels = bid_depth + ask_depth;
        
        Some(LiquidityMetrics {
            spread_bps,
            depth_5_levels,
            kyle_lambda: 0.0, // Would calculate from trade data
            amihud_ratio: 0.0,
            time_to_liquidate: 0.0,
        })
    }
    
    /// Get VPIN for symbol
    pub fn get_vpin(&self, symbol: &str) -> Result<f64, MicrostructureError> {
        self.vpin_calculators
            .get(symbol)
            .ok_or(MicrostructureError::InsufficientData)?
            .calculate_vpin()
    }
    
    /// Get adverse selection estimate
    pub fn get_adverse_selection(&self, symbol: &str) -> Result<f64, MicrostructureError> {
        self.adverse_sel_estimators
            .get(symbol)
            .ok_or(MicrostructureError::InsufficientData)?
            .estimate_cost()
    }
    
    /// Detect anomalous flow (possible informed trading)
    pub fn detect_anomalous_flow(&self, symbol: &str) -> bool {
        // Check multiple indicators
        let vpin_high = self.get_vpin(symbol)
            .map(|v| v > 0.6)
            .unwrap_or(false);
        
        let adverse_sel_high = self.get_adverse_selection(symbol)
            .map(|c| c > 5.0) // > 5 bps
            .unwrap_or(false);
        
        let book_imbalance_extreme = self.order_books
            .get(symbol)
            .map(|b| b.imbalance().abs() > 0.8)
            .unwrap_or(false);
        
        vpin_high || adverse_sel_high || book_imbalance_extreme
    }
}

impl Default for MicrostructureAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_order_book_creation() {
        let book = OrderBook::new("AAPL");
        assert_eq!(book.symbol, "AAPL");
        assert!(book.best_bid().is_none());
    }

    #[test]
    fn test_order_book_mid_price() {
        let mut book = OrderBook::new("AAPL");
        
        book.bids.push(OrderBookLevel::new(Decimal::from(100), Decimal::from(500)));
        book.asks.push(OrderBookLevel::new(Decimal::from(101), Decimal::from(500)));
        
        let mid = book.mid_price().unwrap();
        assert_eq!(mid, Decimal::try_from(100.5).unwrap());
    }

    #[test]
    fn test_order_book_spread() {
        let mut book = OrderBook::new("AAPL");
        
        book.bids.push(OrderBookLevel::new(Decimal::from(100), Decimal::from(500)));
        book.asks.push(OrderBookLevel::new(Decimal::from(101), Decimal::from(500)));
        
        let spread = book.spread().unwrap();
        assert_eq!(spread, Decimal::ONE);
        
        let spread_bps = book.spread_bps().unwrap();
        assert!((spread_bps - 99.5).abs() < 1.0); // ~100 bps
    }

    #[test]
    fn test_order_book_imbalance() {
        let mut book = OrderBook::new("AAPL");
        
        book.bids.push(OrderBookLevel::new(Decimal::from(100), Decimal::from(1000)));
        book.asks.push(OrderBookLevel::new(Decimal::from(101), Decimal::from(500)));
        
        let imbalance = book.imbalance();
        assert!(imbalance > 0.0); // More bids
        assert!(imbalance < 1.0);
    }

    #[test]
    fn test_vpin_calculator() {
        let calculator = VpinCalculator::default();
        
        // Should fail with insufficient data
        assert!(calculator.calculate_vpin().is_err());
    }

    #[test]
    fn test_adverse_selection_estimator() {
        let mut estimator = AdverseSelectionEstimator::new(100);
        
        // Add some observations
        for i in 0..20 {
            estimator.add_observation(i % 2 == 0, Decimal::try_from(i as f64 * 0.01).unwrap());
        }
        
        // Should succeed with enough data
        assert!(estimator.estimate_cost().is_ok());
    }

    #[test]
    fn test_microstructure_analyzer() {
        let mut analyzer = MicrostructureAnalyzer::new();
        
        let mut book = OrderBook::new("AAPL");
        book.bids.push(OrderBookLevel::new(Decimal::from(100), Decimal::from(1000)));
        book.asks.push(OrderBookLevel::new(Decimal::from(101), Decimal::from(1000)));
        
        analyzer.update_order_book("AAPL", book);
        
        let metrics = analyzer.get_liquidity_metrics("AAPL").unwrap();
        assert!(metrics.spread_bps > 0.0);
    }
}
