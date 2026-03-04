//! Risk management and position sizing module
//!
//! Sprint 22: Comprehensive Risk Management
//! Provides tools for:
//! - Position sizing (Kelly, fixed fractional, volatility-based)
//! - Stop-loss and take-profit management
//! - Portfolio risk monitoring (VaR, CVaR, drawdown)
//! - Risk limit enforcement

pub mod advanced;
pub mod portfolio_risk;
pub mod position_sizing;
pub mod risk_manager;
pub mod stop_loss;

pub use advanced::{AdvancedRiskEngine, PortfolioGreeks, StressTestResults, VaRResult};
pub use portfolio_risk::{PortfolioRisk, Position, RiskMetrics, VaRConfig};
pub use position_sizing::{PositionSizer, SizingConfig, SizingMethod};
pub use risk_manager::{RiskAssessment, RiskLimits, RiskManager};
pub use stop_loss::{StopLossManager, StopLossType, TakeProfitConfig};

use rust_decimal::Decimal;
use thiserror::Error;

/// Risk module errors
#[derive(Error, Debug, Clone)]
pub enum RiskError {
    #[error("Insufficient margin: required {required}, available {available}")]
    InsufficientMargin {
        required: Decimal,
        available: Decimal,
    },

    #[error("Position size {size} exceeds maximum {max}")]
    PositionSizeExceeded { size: Decimal, max: Decimal },

    #[error("Risk limit exceeded: {metric} = {value}, limit = {limit}")]
    RiskLimitExceeded {
        metric: String,
        value: Decimal,
        limit: Decimal,
    },

    #[error("Invalid volatility data")]
    InvalidVolatility,

    #[error("Calculation error: {0}")]
    CalculationError(String),
}

pub type Result<T> = std::result::Result<T, RiskError>;

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;

    #[test]
    fn test_risk_error_display() {
        let err = RiskError::PositionSizeExceeded {
            size: Decimal::from(1000),
            max: Decimal::from(500),
        };
        assert!(err.to_string().contains("exceeds maximum"));
    }
}
