//! Integration module errors

use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum IntegrationError {
    #[error("Treasury error: {0}")]
    TreasuryError(String),
    
    #[error("Margin error: {0}")]
    MarginError(String),
    
    #[error("Insufficient treasury funds: requested {requested}, available {available}")]
    InsufficientTreasuryFunds { requested: rust_decimal::Decimal, available: rust_decimal::Decimal },
    
    #[error("Margin limit exceeded: exposure {exposure}, limit {limit}")]
    MarginLimitExceeded { exposure: rust_decimal::Decimal, limit: rust_decimal::Decimal },
    
    #[error("Risk threshold breached: {0}")]
    RiskThresholdBreached(String),
    
    #[error("Allocation failed: {0}")]
    AllocationFailed(String),
    
    #[error("Sync error: {0}")]
    SyncError(String),
}

pub type Result<T> = std::result::Result<T, IntegrationError>;
