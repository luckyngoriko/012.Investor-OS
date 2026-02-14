//! Margin module errors

use rust_decimal::Decimal;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, MarginError>;

#[derive(Error, Debug, Clone)]
pub enum MarginError {
    #[error("Insufficient margin: required {required}, available {available}")]
    InsufficientMargin { required: Decimal, available: Decimal },
    
    #[error("Margin call triggered: equity {equity}, required {required}")]
    MarginCall { equity: Decimal, required: Decimal },
    
    #[error("Account liquidated: remaining equity {equity}")]
    AccountLiquidated { equity: Decimal },
    
    #[error("Position not found: {0}")]
    PositionNotFound(String),
    
    #[error("Invalid leverage: {0}x (max {1}x)")]
    InvalidLeverage(Decimal, Decimal),
    
    #[error("Max positions exceeded: {0} of {1}")]
    MaxPositionsExceeded(usize, usize),
    
    #[error("Price unavailable for {0}")]
    PriceUnavailable(String),
    
    #[error("Treasury error: {0}")]
    TreasuryError(String),
}
