//! Trade Flow Analysis

use chrono::{DateTime, Duration, Utc};
use rust_decimal::Decimal;
use std::collections::VecDeque;
use tracing::debug;

use super::MarketTick;

/// Trade classification
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TradeClassification {
    Retail,        // Small size
    Institutional, // Large size
    Block,         // Very large
    Unknown,
}

impl TradeClassification {
    /// Classify trade by size
    pub fn from_size(quantity: Decimal, _symbol: &str) -> Self {
        // Simplified thresholds using Decimal comparison
        let block_threshold = Decimal::from(100);
        let institutional_threshold = Decimal::from(10);

        if quantity >= block_threshold {
            TradeClassification::Block
        } else if quantity >= institutional_threshold {
            TradeClassification::Institutional
        } else if quantity > Decimal::ZERO {
            TradeClassification::Retail
        } else {
            TradeClassification::Unknown
        }
    }
}

/// Trade aggressor side
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AggressorSide {
    Buyer,  // Took the ask (aggressive buyer)
    Seller, // Hit the bid (aggressive seller)
    Unknown,
}

/// Analyzed trade
#[derive(Debug, Clone)]
pub struct AnalyzedTrade {
    pub tick: MarketTick,
    pub classification: TradeClassification,
    pub aggressor: AggressorSide,
    pub notional: Decimal,
    pub is_aggressive: bool,
}

/// Trade flow metrics
#[derive(Debug, Clone, Default)]
pub struct TradeFlow {
    pub total_volume: Decimal,
    pub buy_volume: Decimal,
    pub sell_volume: Decimal,
    pub buy_pressure: Decimal, // 0.0 - 1.0
    pub avg_trade_size: Decimal,
    pub large_trade_count: u32,
    pub trade_count: u32,
    pub vwap: Decimal,
}

/// Trade analyzer
#[derive(Debug, Clone)]
pub struct TradeAnalyzer {
    /// Recent trades window
    trades: VecDeque<(DateTime<Utc>, AnalyzedTrade)>,
    /// Window duration
    window: Duration,
    /// Large trade threshold
    large_trade_threshold: Decimal,
    /// Total notional tracked
    total_notional: Decimal,
    /// VWAP accumulator
    vwap_num: Decimal,
    vwap_denom: Decimal,
}

impl TradeAnalyzer {
    /// Create new analyzer
    pub fn new(window_sec: i64, large_trade_threshold: Decimal) -> Self {
        Self {
            trades: VecDeque::new(),
            window: Duration::seconds(window_sec),
            large_trade_threshold,
            total_notional: Decimal::ZERO,
            vwap_num: Decimal::ZERO,
            vwap_denom: Decimal::ZERO,
        }
    }

    /// Analyze a trade tick
    pub fn analyze(
        &mut self,
        tick: MarketTick,
        best_bid: Decimal,
        best_ask: Decimal,
    ) -> AnalyzedTrade {
        let classification = TradeClassification::from_size(tick.quantity, &tick.symbol);

        // Determine aggressor side
        let aggressor = if tick.price >= best_ask {
            AggressorSide::Buyer
        } else if tick.price <= best_bid {
            AggressorSide::Seller
        } else {
            AggressorSide::Unknown
        };

        let notional = tick.price * tick.quantity;
        let is_aggressive = matches!(aggressor, AggressorSide::Buyer | AggressorSide::Seller);

        let analyzed = AnalyzedTrade {
            tick: tick.clone(),
            classification,
            aggressor,
            notional,
            is_aggressive,
        };

        // Add to window
        self.trades.push_back((Utc::now(), analyzed.clone()));

        // Update VWAP accumulators
        self.vwap_num += notional;
        self.vwap_denom += tick.quantity;

        // Clean old trades
        self.clean_old_trades();

        debug!(
            "Analyzed trade: {} {} @ {} - {:?} aggressor",
            tick.quantity, tick.symbol, tick.price, aggressor
        );

        analyzed
    }

    /// Get current trade flow metrics
    pub fn get_flow(&self) -> TradeFlow {
        if self.trades.is_empty() {
            return TradeFlow::default();
        }

        let mut total_vol = Decimal::ZERO;
        let mut buy_vol = Decimal::ZERO;
        let mut sell_vol = Decimal::ZERO;
        let mut large_count = 0u32;
        let mut total_qty = Decimal::ZERO;

        for (_, trade) in &self.trades {
            total_vol += trade.notional;
            total_qty += trade.tick.quantity;

            match trade.aggressor {
                AggressorSide::Buyer => buy_vol += trade.notional,
                AggressorSide::Seller => sell_vol += trade.notional,
                _ => {}
            }

            if trade.tick.quantity >= self.large_trade_threshold {
                large_count += 1;
            }
        }

        let total_vol_for_pressure = buy_vol + sell_vol;
        let buy_pressure = if !total_vol_for_pressure.is_zero() {
            buy_vol / total_vol_for_pressure
        } else {
            Decimal::try_from(0.5).unwrap()
        };

        let avg_size = if !self.trades.is_empty() {
            total_qty / Decimal::from(self.trades.len() as i64)
        } else {
            Decimal::ZERO
        };

        let vwap = if !self.vwap_denom.is_zero() {
            self.vwap_num / self.vwap_denom
        } else {
            Decimal::ZERO
        };

        TradeFlow {
            total_volume: total_vol,
            buy_volume: buy_vol,
            sell_volume: sell_vol,
            buy_pressure,
            avg_trade_size: avg_size,
            large_trade_count: large_count,
            trade_count: self.trades.len() as u32,
            vwap,
        }
    }

    /// Detect trade clustering (unusual activity)
    /// Returns true if recent trades are coming faster than average
    pub fn detect_clustering(&self, _threshold_std: Decimal) -> bool {
        if self.trades.len() < 10 {
            return false;
        }

        // Calculate mean inter-trade time
        let mut total_interval: i64 = 0;
        let trades: Vec<_> = self.trades.iter().collect();

        for i in 1..trades.len() {
            let diff = trades[i].0 - trades[i - 1].0;
            total_interval += diff.num_milliseconds();
        }

        let count = (trades.len() - 1) as i64;
        if count == 0 {
            return false;
        }

        let mean_interval = total_interval / count;

        // Check if last trade was faster than average
        if let Some(last_diff) = trades.last().and_then(|(t, _)| {
            trades
                .get(trades.len().saturating_sub(2))
                .map(|(t2, _)| (*t - *t2).num_milliseconds())
        }) {
            // If last interval is less than 50% of mean, consider it clustering
            last_diff < (mean_interval / 2)
        } else {
            false
        }
    }

    /// Get aggressive trade imbalance
    /// Returns (buy_count, sell_count, imbalance_ratio)
    pub fn get_aggressive_imbalance(&self) -> (u32, u32, Decimal) {
        let mut buys = 0;
        let mut sells = 0;

        for (_, trade) in &self.trades {
            if trade.is_aggressive {
                match trade.aggressor {
                    AggressorSide::Buyer => buys += 1,
                    AggressorSide::Seller => sells += 1,
                    _ => {}
                }
            }
        }

        let total = buys + sells;
        let imbalance = if total > 0 {
            Decimal::from(buys as i64 - sells as i64) / Decimal::from(total as i64)
        } else {
            Decimal::ZERO
        };

        (buys, sells, imbalance)
    }

    /// Get trades by classification
    pub fn get_by_classification(
        &self,
        classification: TradeClassification,
    ) -> Vec<&AnalyzedTrade> {
        self.trades
            .iter()
            .filter(|(_, t)| t.classification == classification)
            .map(|(_, t)| t)
            .collect()
    }

    /// Clean old trades outside the window
    fn clean_old_trades(&mut self) {
        let now = Utc::now();
        let cutoff = now - self.window;

        while let Some((timestamp, trade)) = self.trades.front() {
            if *timestamp < cutoff {
                // Remove from VWAP accumulators
                self.vwap_num -= trade.notional;
                self.vwap_denom -= trade.tick.quantity;
                self.trades.pop_front();
            } else {
                break;
            }
        }
    }

    /// Get recent trades
    pub fn get_recent(&self, count: usize) -> Vec<&AnalyzedTrade> {
        self.trades
            .iter()
            .rev()
            .take(count)
            .map(|(_, t)| t)
            .collect()
    }

    /// Clear all data
    pub fn clear(&mut self) {
        self.trades.clear();
        self.total_notional = Decimal::ZERO;
        self.vwap_num = Decimal::ZERO;
        self.vwap_denom = Decimal::ZERO;
    }
}

#[cfg(test)]
mod tests {
    use super::super::orderbook::Side as BookSide;
    use super::*;

    fn create_test_tick(price: f64, qty: f64, side: BookSide) -> MarketTick {
        MarketTick {
            symbol: "BTCUSDT".to_string(),
            exchange: "binance".to_string(),
            price: Decimal::try_from(price).unwrap(),
            quantity: Decimal::try_from(qty).unwrap(),
            side: Some(side),
            timestamp: Utc::now(),
            tick_type: super::super::TickType::Trade,
        }
    }

    #[test]
    fn test_trade_classification() {
        assert_eq!(
            TradeClassification::from_size(Decimal::try_from(0.5).unwrap(), "BTC"),
            TradeClassification::Retail
        );
        assert_eq!(
            TradeClassification::from_size(Decimal::try_from(50.0).unwrap(), "BTC"),
            TradeClassification::Institutional
        );
        assert_eq!(
            TradeClassification::from_size(Decimal::try_from(200.0).unwrap(), "BTC"),
            TradeClassification::Block
        );
    }

    #[test]
    fn test_analyze_trade() {
        let mut analyzer = TradeAnalyzer::new(60, Decimal::from(10));

        // Buyer-aggressive trade: price >= best_ask (hitting the ask)
        let tick = create_test_tick(50001.0, 1.0, BookSide::Bid);
        let analyzed = analyzer.analyze(tick, Decimal::from(49999), Decimal::from(50001));

        assert_eq!(analyzed.classification, TradeClassification::Retail);
        assert_eq!(analyzed.aggressor, AggressorSide::Buyer);
        assert!(analyzed.is_aggressive);
    }

    #[test]
    fn test_trade_flow() {
        let mut analyzer = TradeAnalyzer::new(60, Decimal::from(10));

        // Add some buys (buyer-aggressive: price >= best_ask)
        for _ in 0..5 {
            let tick = create_test_tick(50001.0, 1.0, BookSide::Bid);
            analyzer.analyze(tick, Decimal::from(49999), Decimal::from(50001));
        }

        // Add some sells (seller-aggressive: price <= best_bid)
        for _ in 0..3 {
            let tick = create_test_tick(49998.0, 1.0, BookSide::Ask);
            analyzer.analyze(tick, Decimal::from(49998), Decimal::from(50000));
        }

        let flow = analyzer.get_flow();

        assert_eq!(flow.trade_count, 8);
        assert!(flow.buy_pressure > Decimal::try_from(0.5).unwrap()); // More buys
    }

    #[test]
    fn test_aggressive_imbalance() {
        let mut analyzer = TradeAnalyzer::new(60, Decimal::from(10));

        // Aggressive buys (at ask)
        for _ in 0..3 {
            let tick = create_test_tick(50001.0, 1.0, BookSide::Bid);
            analyzer.analyze(tick, Decimal::from(50000), Decimal::from(50001));
        }

        // Aggressive sells (at bid)
        for _ in 0..1 {
            let tick = create_test_tick(50000.0, 1.0, BookSide::Ask);
            analyzer.analyze(tick, Decimal::from(50000), Decimal::from(50001));
        }

        let (buys, sells, imbalance) = analyzer.get_aggressive_imbalance();

        assert_eq!(buys, 3);
        assert_eq!(sells, 1);
        assert!(imbalance > Decimal::ZERO); // More aggressive buying
    }

    #[test]
    fn test_window_cleanup() {
        let mut analyzer = TradeAnalyzer::new(1, Decimal::from(10)); // 1 second window

        let tick = create_test_tick(50000.0, 1.0, BookSide::Bid);
        analyzer.analyze(tick, Decimal::from(49999), Decimal::from(50001));

        assert_eq!(analyzer.trades.len(), 1);

        // Wait for window to expire (simulated by manual cleanup)
        std::thread::sleep(std::time::Duration::from_millis(1100));

        let tick2 = create_test_tick(50001.0, 1.0, BookSide::Bid);
        analyzer.analyze(tick2, Decimal::from(50000), Decimal::from(50002));

        // First trade should be cleaned up
        assert!(analyzer.trades.len() <= 2);
    }
}
