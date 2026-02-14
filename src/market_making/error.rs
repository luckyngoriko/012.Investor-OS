//! Market Making module errors

use thiserror::Error;
use rust_decimal::Decimal;

pub type Result<T> = std::result::Result<T, MarketMakingError>;

#[derive(Error, Debug, Clone)]
pub enum MarketMakingError {
    #[error("Inventory limit exceeded: current {current}, limit {limit}")]
    InventoryLimitExceeded { current: Decimal, limit: Decimal },
    
    #[error("Spread too tight: current {current} bps, minimum {minimum} bps")]
    SpreadTooTight { current: Decimal, minimum: Decimal },
    
    #[error("Quote rejected: {0}")]
    QuoteRejected(String),
    
    #[error("Adverse selection detected: {0}")]
    AdverseSelection(String),
    
    #[error("Hedge failed: {0}")]
    HedgeFailed(String),
    
    #[error("Position limit reached: {0}")]
    PositionLimitReached(String),
    
    #[error("Venue error: {0}")]
    VenueError(String),
}
