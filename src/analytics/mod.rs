//! Advanced Analytics Module
//!
//! Sprint 7: Advanced Analytics & Backtesting
//! - S7-D1: Backtesting Engine
//! - S7-D2: Risk Metrics (VaR, Sharpe, Drawdown)
//! - S7-D3: Performance Attribution
//! - S7-D4: ML Feature Pipeline
//! - S7-D5: XGBoost CQ Model
//! - S7-D6: Anomaly Detection
//! - S7-D7: Backtest API
//! - S7-D8: Analytics Dashboard

pub mod attribution;
pub mod backtest;
pub mod ml;
pub mod risk;
pub mod service;

pub use service::AnalyticsService;

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Analytics-specific errors
#[derive(Error, Debug, Clone)]
pub enum AnalyticsError {
    #[error("Insufficient data: {0}")]
    InsufficientData(String),

    #[error("Calculation error: {0}")]
    Calculation(String),

    #[error("Invalid parameters: {0}")]
    InvalidParameters(String),

    #[error("Model error: {0}")]
    Model(String),
}

/// Result type for analytics operations
pub type Result<T> = std::result::Result<T, AnalyticsError>;

/// Trade record for backtesting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trade {
    pub timestamp: DateTime<Utc>,
    pub ticker: String,
    pub side: crate::broker::OrderSide,
    pub quantity: Decimal,
    pub price: Decimal,
    pub commission: Decimal,
}

/// Daily portfolio snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailySnapshot {
    pub date: DateTime<Utc>,
    pub nav: Decimal, // Net Asset Value
    pub cash: Decimal,
    pub positions_value: Decimal,
    pub positions: Vec<PositionSnapshot>,
}

/// Position snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionSnapshot {
    pub ticker: String,
    pub quantity: Decimal,
    pub market_price: Decimal,
    pub market_value: Decimal,
}

/// Price bar for historical data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceBar {
    pub timestamp: DateTime<Utc>,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: i64,
}

/// Trading strategy trait
#[async_trait::async_trait]
pub trait Strategy: Send + Sync {
    /// Strategy name
    fn name(&self) -> &str;

    /// Generate trading signals for current market data
    async fn generate_signals(&self, data: &MarketData) -> Vec<Signal>;

    /// Calculate position size for a signal
    fn position_size(&self, signal: &Signal, portfolio_value: Decimal) -> Decimal;
}

/// Market data container
#[derive(Debug, Clone)]
pub struct MarketData {
    pub timestamp: DateTime<Utc>,
    pub prices: std::collections::HashMap<String, PriceBar>,
}

/// Trading signal
#[derive(Debug, Clone)]
pub struct Signal {
    pub ticker: String,
    pub direction: SignalDirection,
    pub strength: f64, // 0.0 to 1.0
    pub confidence: f64,
}

/// Signal direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalDirection {
    Long,
    Short,
    Neutral,
}

impl SignalDirection {
    pub fn as_i8(&self) -> i8 {
        match self {
            SignalDirection::Long => 1,
            SignalDirection::Short => -1,
            SignalDirection::Neutral => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signal_direction() {
        assert_eq!(SignalDirection::Long.as_i8(), 1);
        assert_eq!(SignalDirection::Short.as_i8(), -1);
        assert_eq!(SignalDirection::Neutral.as_i8(), 0);
    }
}
