//! Hierarchical Reasoning Model (HRM) Native Implementation
//!
//! Production-grade ML inference for adaptive Conviction Quotient calculation.
//! Zero Python dependencies, full Rust implementation using burn framework.
//!
//! # Architecture
//! - High-Level Module: Slow, abstract planning (market regime detection)
//! - Low-Level Module: Fast, detailed execution (signal aggregation)
//! - Cross-connections for hierarchical information flow
//!
//! # Example
//! ```rust
//! use investor_os::hrm::{HRM, HRMConfig, InferenceEngine};
//!
//! let config = HRMConfig::default();
//! let hrm = HRM::new(&config).expect("Failed to initialize HRM");
//!
//! let signals = vec![0.8, 0.9, 0.7, 15.0, 0.0, 0.5]; // PEGY, Insider, Sentiment, VIX, Regime, Time
//! let result = hrm.infer(&signals).expect("Inference failed");
//!
//! assert!(result.conviction >= 0.0 && result.conviction <= 1.0);
//! ```

pub mod config;
pub mod gpu; // Sprint 48: Multi-Backend GPU Support
pub mod inference;
pub mod lstm;
pub mod model;
pub mod weights;

pub use config::{DeviceConfig, HRMConfig};
pub use inference::{InferenceEngine, InferenceResult};
pub use model::{HRMBuilder, HRM};
pub use weights::WeightLoader;

/// HRM Input signals
#[derive(Debug, Clone, Copy)]
pub struct HrmInput {
    pub pegy: f64,
    pub insider_sentiment: f64,
    pub social_sentiment: f64,
    pub vix: f64,
}

impl HrmInput {
    pub fn new(pegy: f64, insider_sentiment: f64, social_sentiment: f64, vix: f64) -> Self {
        Self {
            pegy,
            insider_sentiment,
            social_sentiment,
            vix,
        }
    }
}

/// HRM Output with trading recommendation
#[derive(Debug, Clone, Copy)]
pub struct HrmOutput {
    pub conviction: f64,
    pub recommended_action: Action,
    pub confidence: f64,
}

/// Trading action recommendation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Buy,
    Sell,
    Hold,
}

use thiserror::Error;

/// HRM-specific errors
#[derive(Error, Debug)]
pub enum HRMError {
    #[error("Model initialization failed: {0}")]
    InitializationError(String),

    #[error("Weight loading failed: {0}")]
    WeightLoadError(String),

    #[error("Inference failed: {0}")]
    InferenceError(String),

    #[error("Invalid input shape: expected {expected}, got {actual}")]
    InvalidInputShape { expected: usize, actual: usize },

    #[error("GPU unavailable, CPU fallback failed: {0}")]
    GPUNotAvailable(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),
}

/// Result type for HRM operations
pub type Result<T> = std::result::Result<T, HRMError>;

/// Market regime classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MarketRegime {
    Bull = 0,
    Bear = 1,
    Sideways = 2,
    Crisis = 3,
}

impl From<f32> for MarketRegime {
    fn from(value: f32) -> Self {
        match value.round() as u8 {
            0 => MarketRegime::Bull,
            1 => MarketRegime::Bear,
            2 => MarketRegime::Sideways,
            3 => MarketRegime::Crisis,
            _ => MarketRegime::Sideways, // Default
        }
    }
}

impl From<MarketRegime> for f32 {
    fn from(regime: MarketRegime) -> Self {
        regime as u8 as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_market_regime_from_f32() {
        assert_eq!(MarketRegime::from(0.0), MarketRegime::Bull);
        assert_eq!(MarketRegime::from(1.0), MarketRegime::Bear);
        assert_eq!(MarketRegime::from(2.0), MarketRegime::Sideways);
        assert_eq!(MarketRegime::from(3.0), MarketRegime::Crisis);
        assert_eq!(MarketRegime::from(99.0), MarketRegime::Sideways); // Default
    }

    #[test]
    fn test_market_regime_to_f32() {
        assert_eq!(f32::from(MarketRegime::Bull), 0.0);
        assert_eq!(f32::from(MarketRegime::Bear), 1.0);
        assert_eq!(f32::from(MarketRegime::Sideways), 2.0);
        assert_eq!(f32::from(MarketRegime::Crisis), 3.0);
    }
}
