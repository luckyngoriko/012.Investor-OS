//! Phoenix Mode: Autonomous Learning System
//! 
//! Self-learning trading system that:
//! 1. Trains on historical data with paper trading
//! 2. Uses RAG memory to learn from past experiences
//! 3. Employs LLM strategist for decision making
//! 4. Graduates through realistic criteria (NOT 200x in 5 years)
//!
//! Built on:
//! - Sprint 5: neurocod-rag (memory)
//! - Sprint 5: TimescaleDB (historical data)
//! - Sprint 6: IB API pattern (paper trading)
//! - Sprint 7: ML pipeline (features)

pub mod assessment;
pub mod graduation;

use rust_decimal::Decimal;
use std::collections::HashMap;

pub use graduation::*;

/// Phoenix Engine - Main autonomous learning orchestrator (skeleton)
pub struct PhoenixEngine {
    config: PhoenixConfig,
}

impl PhoenixEngine {
    pub fn new(config: PhoenixConfig) -> Self {
        Self { config }
    }
    
    pub fn run_training_epoch(&self) -> EpochResult {
        // Skeleton implementation
        EpochResult {
            epoch_number: 1,
            metrics: EpochMetrics::default(),
            daily_results: vec![],
        }
    }
}

#[derive(Debug, Clone)]
pub struct PhoenixConfig {
    pub initial_capital: Decimal,
    pub currency: String,
    pub min_epochs_before_graduation: u32,
    pub max_epochs: u32,
    pub patience: u32,
    pub assessment_frequency: u32,
    pub graduation_config: GraduationConfig,
}

impl Default for PhoenixConfig {
    fn default() -> Self {
        Self {
            initial_capital: Decimal::from(1000),
            currency: "EUR".to_string(),
            min_epochs_before_graduation: 5,
            max_epochs: 100,
            patience: 10,
            assessment_frequency: 5,
            graduation_config: GraduationConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct EpochResult {
    pub epoch_number: u32,
    pub metrics: EpochMetrics,
    pub daily_results: Vec<DailyResult>,
}

#[derive(Debug, Clone, Default)]
pub struct EpochMetrics {
    pub final_portfolio_value: Decimal,
    pub total_return: Decimal,
    pub cagr: f64,
    pub sharpe_ratio: f64,
    pub max_drawdown: Decimal,
    pub win_rate: f64,
    pub profit_factor: f64,
}

#[derive(Debug, Clone, Default)]
pub struct DailyResult {
    pub decision: TradingDecision,
}

#[derive(Debug, Clone, Default)]
pub struct TradingDecision {
    pub action: Action,
    pub ticker: String,
    pub quantity: Option<u32>,
    pub confidence: f64,
    pub rationale: String,
}

#[derive(Debug, Clone, Default)]
pub enum Action {
    #[default]
    Hold,
    Buy,
    Sell,
}

#[derive(Debug, Clone, Default)]
pub struct TrainingOutcome;

#[derive(Debug, Clone, Default)]
pub struct FeatureVector {
    pub features: HashMap<String, f64>,
    pub ticker: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl FeatureVector {
    pub fn default() -> Self {
        Self {
            features: HashMap::new(),
            ticker: "UNKNOWN".to_string(),
            timestamp: chrono::Utc::now(),
        }
    }
}
