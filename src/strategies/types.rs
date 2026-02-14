//! Strategy types and signal definitions

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// Trading signal direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SignalDirection {
    Long,
    Short,
    Neutral,
}

/// Trading signal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Signal {
    pub symbol: String,
    pub direction: SignalDirection,
    pub strength: Decimal, // 0.0 to 1.0
    pub confidence: Decimal, // 0.0 to 1.0
    pub timestamp: DateTime<Utc>,
    pub metadata: SignalMetadata,
}

/// Signal metadata
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SignalMetadata {
    pub strategy_id: String,
    pub entry_price: Option<Decimal>,
    pub stop_loss: Option<Decimal>,
    pub take_profit: Option<Decimal>,
    pub expected_return: Option<Decimal>,
    pub rationale: Option<String>,
}

impl Signal {
    /// Create new signal
    pub fn new(
        symbol: impl Into<String>,
        direction: SignalDirection,
        strength: Decimal,
    ) -> Self {
        Self {
            symbol: symbol.into(),
            direction,
            strength: strength.clamp(Decimal::ZERO, Decimal::ONE),
            confidence: Decimal::ZERO,
            timestamp: Utc::now(),
            metadata: SignalMetadata::default(),
        }
    }
    
    /// Check if signal is actionable
    pub fn is_actionable(&self) -> bool {
        self.strength > Decimal::try_from(0.3).unwrap()
            && self.direction != SignalDirection::Neutral
    }
    
    /// Set confidence
    pub fn with_confidence(mut self, confidence: Decimal) -> Self {
        self.confidence = confidence.clamp(Decimal::ZERO, Decimal::ONE);
        self
    }
    
    /// Set metadata
    pub fn with_metadata(mut self, metadata: SignalMetadata) -> Self {
        self.metadata = metadata;
        self
    }
}

/// Strategy performance metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StrategyPerformance {
    pub total_return: Decimal,
    pub sharpe_ratio: Decimal,
    pub max_drawdown: Decimal,
    pub win_rate: Decimal,
    pub profit_factor: Decimal,
    pub avg_trade_return: Decimal,
    pub total_trades: usize,
    pub winning_trades: usize,
    pub losing_trades: usize,
}

/// Strategy type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StrategyType {
    StatisticalArbitrage,
    Momentum,
    MeanReversion,
    TrendFollowing,
    PairsTrading,
    MultiFactor,
}

impl StrategyType {
    pub fn name(&self) -> &'static str {
        match self {
            StrategyType::StatisticalArbitrage => "Statistical Arbitrage",
            StrategyType::Momentum => "Momentum",
            StrategyType::MeanReversion => "Mean Reversion",
            StrategyType::TrendFollowing => "Trend Following",
            StrategyType::PairsTrading => "Pairs Trading",
            StrategyType::MultiFactor => "Multi-Factor",
        }
    }
}

/// Market regime
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MarketRegime {
    TrendingUp,
    TrendingDown,
    Ranging,
    Volatile,
    Unknown,
}

/// Price data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceData {
    pub timestamp: DateTime<Utc>,
    pub open: Decimal,
    pub high: Decimal,
    pub low: Decimal,
    pub close: Decimal,
    pub volume: Decimal,
}

impl PriceData {
    /// Create from close price only
    pub fn from_close(close: Decimal, timestamp: DateTime<Utc>) -> Self {
        Self {
            timestamp,
            open: close,
            high: close,
            low: close,
            close,
            volume: Decimal::ZERO,
        }
    }
    
    /// Typical price (H+L+C)/3
    pub fn typical_price(&self) -> Decimal {
        (self.high + self.low + self.close) / Decimal::from(3)
    }
    
    /// True range
    pub fn true_range(&self, prev_close: Decimal) -> Decimal {
        let tr1 = self.high - self.low;
        let tr2 = (self.high - prev_close).abs();
        let tr3 = (self.low - prev_close).abs();
        tr1.max(tr2).max(tr3)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_signal_creation() {
        let signal = Signal::new("BTC", SignalDirection::Long, Decimal::try_from(0.8).unwrap());
        
        assert_eq!(signal.symbol, "BTC");
        assert_eq!(signal.direction, SignalDirection::Long);
        assert_eq!(signal.strength, Decimal::try_from(0.8).unwrap());
        assert!(signal.is_actionable());
    }
    
    #[test]
    fn test_weak_signal_not_actionable() {
        let signal = Signal::new("BTC", SignalDirection::Long, Decimal::try_from(0.1).unwrap());
        assert!(!signal.is_actionable());
    }
    
    #[test]
    fn test_neutral_signal_not_actionable() {
        let signal = Signal::new("BTC", SignalDirection::Neutral, Decimal::try_from(0.9).unwrap());
        assert!(!signal.is_actionable());
    }
    
    #[test]
    fn test_price_data_typical_price() {
        let data = PriceData {
            timestamp: Utc::now(),
            open: Decimal::from(100),
            high: Decimal::from(110),
            low: Decimal::from(95),
            close: Decimal::from(105),
            volume: Decimal::from(1000),
        };
        
        // (110 + 95 + 105) / 3 = 103.33
        let tp = data.typical_price();
        assert!(tp > Decimal::from(103) && tp < Decimal::from(104));
    }
}
