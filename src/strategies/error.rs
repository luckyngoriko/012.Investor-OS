//! Strategies module errors

use thiserror::Error;

pub type Result<T> = std::result::Result<T, StrategyError>;

#[derive(Error, Debug, Clone)]
pub enum StrategyError {
    #[error("Insufficient data: need {required}, have {available}")]
    InsufficientData { required: usize, available: usize },
    
    #[error("Signal generation failed: {0}")]
    SignalGenerationFailed(String),
    
    #[error("Invalid parameters: {0}")]
    InvalidParameters(String),
    
    #[error("Correlation calculation failed: {0}")]
    CorrelationError(String),
    
    #[error("Backtest failed: {0}")]
    BacktestFailed(String),
    
    #[error("Strategy not initialized")]
    NotInitialized,
    
    #[error("Risk limit exceeded: {0}")]
    RiskLimitExceeded(String),
}
