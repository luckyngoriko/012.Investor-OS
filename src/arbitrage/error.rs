//! Arbitrage module errors

use thiserror::Error;
use rust_decimal::Decimal;

pub type Result<T> = std::result::Result<T, ArbitrageError>;

#[derive(Error, Debug, Clone)]
pub enum ArbitrageError {
    #[error("No arbitrage opportunity found")]
    NoOpportunity,
    
    #[error("Opportunity expired: expected {expected}, got {actual}")]
    OpportunityExpired { expected: Decimal, actual: Decimal },
    
    #[error("Insufficient capital: needed {needed}, available {available}")]
    InsufficientCapital { needed: Decimal, available: Decimal },
    
    #[error("Execution failed: {0}")]
    ExecutionFailed(String),
    
    #[error("Risk limit exceeded: {0}")]
    RiskLimitExceeded(String),
    
    #[error("Latency too high: {0}ms")]
    LatencyTooHigh(u64),
    
    #[error("Price feed error: {0}")]
    PriceFeedError(String),
}
