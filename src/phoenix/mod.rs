//! Phoenix Mode: Autonomous Learning System v2.0
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
pub mod memory;
pub mod simulator;
pub mod strategist;
pub mod engine;
pub mod graph_integration;

// Re-export main components
pub use assessment::GraduationAssessor;
pub use graduation::GraduationAssessment;
pub use graduation::*;
pub use memory::{RagMemory, MemoryStats, RegimeInsight, TradingExperience, ExperienceQuery, OutcomeFilter};
pub use simulator::{PaperTradingSimulator, SimulatorConfig, MarketDataPoint};
pub use strategist::{LlmStrategist, StrategistConfig, Sentiment, DecisionContext};
pub use engine::{PhoenixEngine, TrainingResult, EngineStats};
pub use graph_integration::{PhoenixGraphEngine, CollectSignalsNode, DetectRegimeNode, ApplyStrategyNode, RiskCheckNode, MakeDecisionNode};

use rust_decimal::Decimal;
use std::collections::HashMap;

/// Phoenix Engine Configuration
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

/// Epoch training result
#[derive(Debug, Clone, Default)]
pub struct EpochResult {
    pub epoch_number: u32,
    pub metrics: EpochMetrics,
    pub daily_results: Vec<DailyResult>,
}

/// Epoch performance metrics
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

/// Daily trading result
#[derive(Debug, Clone, Default)]
pub struct DailyResult {
    pub decision: TradingDecision,
}

/// Trading decision
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct TradingDecision {
    pub action: Action,
    pub ticker: String,
    pub quantity: Option<u32>,
    pub confidence: f64,
    pub rationale: String,
}

/// Trading action
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Action {
    #[default]
    Hold,
    Buy,
    Sell,
}

/// Training outcome
#[derive(Debug, Clone, Default)]
pub struct TrainingOutcome;

/// Feature vector for ML
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

/// Helper trait for Decimal conversion
pub trait DecimalExt {
    fn to_f64(&self) -> Option<f64>;
}

impl DecimalExt for Decimal {
    fn to_f64(&self) -> Option<f64> {
        use std::str::FromStr;
        f64::from_str(&self.to_string()).ok()
    }
}
