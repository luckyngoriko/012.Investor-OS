//! Execution module errors

use thiserror::Error;
use rust_decimal::Decimal;

pub type Result<T> = std::result::Result<T, ExecutionError>;

#[derive(Error, Debug, Clone)]
pub enum ExecutionError {
    #[error("Venue not available: {0}")]
    VenueNotAvailable(String),
    
    #[error("Insufficient liquidity: needed {needed}, available {available}")]
    InsufficientLiquidity { needed: Decimal, available: Decimal },
    
    #[error("Execution failed: {0}")]
    ExecutionFailed(String),
    
    #[error("Routing failed: {0}")]
    RoutingFailed(String),
    
    #[error("Cost estimation failed: {0}")]
    CostEstimationFailed(String),
    
    #[error("Algorithm error: {0}")]
    AlgorithmError(String),
    
    #[error("Timeout waiting for fill")]
    Timeout,
    
    #[error("Partial fill: filled {filled} of {requested}")]
    PartialFill { filled: Decimal, requested: Decimal },

    #[error("No market data: {0}")]
    NoMarketData(String),
}
