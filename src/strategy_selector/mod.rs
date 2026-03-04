//! ML Strategy Selector
//!
//! Sprint 31: ML Strategy Selector
//! - Automatic strategy selection based on market regime
//! - Performance attribution across strategies
//! - Dynamic strategy switching with confidence thresholds
//! - Strategy recommendation engine

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use std::collections::HashMap;
use thiserror::Error;
use tracing::{info, warn};
use uuid::Uuid;

pub mod attribution;
pub mod recommender;
pub mod selector;
pub mod switcher;

pub use attribution::{AttributionEngine, PerformanceAttribution, StrategyPerformance};
pub use recommender::{Recommendation, StrategyRecommender};
pub use selector::{SelectionCriteria, StrategySelector};
pub use switcher::{StrategySwitcher, SwitchConfig};

// HRM Integration
use crate::hrm::MarketRegime as HRMMarketRegime;
use crate::hrm::{HRMBuilder, HRMConfig, HRM};

/// Strategy errors
#[derive(Error, Debug, Clone)]
pub enum SelectorError {
    #[error("Strategy not found: {0}")]
    StrategyNotFound(String),

    #[error("Insufficient data for selection: {0}")]
    InsufficientData(String),

    #[error("Switch failed: {0}")]
    SwitchFailed(String),

    #[error("Attribution error: {0}")]
    Attribution(String),
}

pub type Result<T> = std::result::Result<T, SelectorError>;

/// Strategy type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StrategyType {
    Momentum,
    MeanReversion,
    PairsTrading,
    Breakout,
    TrendFollowing,
    MarketMaking,
    Arbitrage,
    Hybrid,
}

impl StrategyType {
    pub fn name(&self) -> &'static str {
        match self {
            StrategyType::Momentum => "Momentum",
            StrategyType::MeanReversion => "MeanReversion",
            StrategyType::PairsTrading => "PairsTrading",
            StrategyType::Breakout => "Breakout",
            StrategyType::TrendFollowing => "TrendFollowing",
            StrategyType::MarketMaking => "MarketMaking",
            StrategyType::Arbitrage => "Arbitrage",
            StrategyType::Hybrid => "Hybrid",
        }
    }

    /// Get suitable market regimes for this strategy
    pub fn suitable_regimes(&self) -> Vec<MarketRegime> {
        match self {
            StrategyType::Momentum => vec![MarketRegime::Trending, MarketRegime::StrongUptrend],
            StrategyType::MeanReversion => vec![MarketRegime::Ranging, MarketRegime::WeakTrend],
            StrategyType::PairsTrading => vec![MarketRegime::Ranging, MarketRegime::Normal],
            StrategyType::Breakout => {
                vec![MarketRegime::VolatilityExpansion, MarketRegime::Trending]
            }
            StrategyType::TrendFollowing => vec![
                MarketRegime::Trending,
                MarketRegime::StrongUptrend,
                MarketRegime::StrongDowntrend,
            ],
            StrategyType::MarketMaking => vec![
                MarketRegime::Ranging,
                MarketRegime::Normal,
                MarketRegime::LowVolatility,
            ],
            StrategyType::Arbitrage => vec![
                MarketRegime::Any, // Works in all regimes
            ],
            StrategyType::Hybrid => vec![MarketRegime::Any],
        }
    }

    /// Check if suitable for regime
    pub fn is_suitable_for(&self, regime: MarketRegime) -> bool {
        self.suitable_regimes().contains(&regime)
            || self.suitable_regimes().contains(&MarketRegime::Any)
    }
}

/// Market regime
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MarketRegime {
    Trending,
    Ranging,
    Volatile,
    LowVolatility,
    StrongUptrend,
    StrongDowntrend,
    WeakTrend,
    VolatilityExpansion,
    Normal,
    Crisis,
    Recovery,
    Any,
}

impl MarketRegime {
    pub fn from_indicators(trend_strength: f32, volatility: f32, _volume: f32) -> Self {
        if volatility > 0.8 {
            if trend_strength > 0.7 {
                MarketRegime::Volatile
            } else {
                MarketRegime::VolatilityExpansion
            }
        } else if trend_strength > 0.8 {
            MarketRegime::StrongUptrend
        } else if trend_strength < -0.8 {
            MarketRegime::StrongDowntrend
        } else if trend_strength.abs() > 0.5 {
            MarketRegime::Trending
        } else if trend_strength.abs() < 0.2 {
            MarketRegime::Ranging
        } else {
            MarketRegime::Normal
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            MarketRegime::Trending => "Trending",
            MarketRegime::Ranging => "Ranging",
            MarketRegime::Volatile => "Volatile",
            MarketRegime::LowVolatility => "LowVolatility",
            MarketRegime::StrongUptrend => "StrongUptrend",
            MarketRegime::StrongDowntrend => "StrongDowntrend",
            MarketRegime::WeakTrend => "WeakTrend",
            MarketRegime::VolatilityExpansion => "VolatilityExpansion",
            MarketRegime::Normal => "Normal",
            MarketRegime::Crisis => "Crisis",
            MarketRegime::Recovery => "Recovery",
            MarketRegime::Any => "Any",
        }
    }
}

/// Strategy metadata
#[derive(Debug, Clone)]
pub struct Strategy {
    pub id: Uuid,
    pub name: String,
    pub strategy_type: StrategyType,
    pub description: String,
    pub min_capital: Decimal,
    pub max_drawdown: f32,
    pub avg_return: f32,
    pub sharpe_ratio: f32,
    pub win_rate: f32,
    pub trades_per_month: u32,
    pub created_at: DateTime<Utc>,
    pub is_active: bool,
    pub current_allocation: f32, // Percentage
}

/// Strategy selection score
#[derive(Debug, Clone)]
pub struct SelectionScore {
    pub strategy_id: Uuid,
    pub strategy_type: StrategyType,
    pub regime_fit_score: f32,    // 0.0 - 1.0
    pub performance_score: f32,   // 0.0 - 1.0
    pub risk_adjusted_score: f32, // 0.0 - 1.0
    pub recency_score: f32,       // 0.0 - 1.0
    pub overall_score: f32,       // Weighted composite
    pub confidence: f32,          // Confidence in selection
}

/// Current strategy state
#[derive(Debug, Clone)]
pub struct StrategyState {
    pub strategy_id: Uuid,
    pub strategy_type: StrategyType,
    pub started_at: DateTime<Utc>,
    pub current_pnl: Decimal,
    pub trades_executed: u32,
    pub last_switch_reason: Option<String>,
    pub current_regime: MarketRegime,
}

/// ML Strategy Selector coordinator
#[derive(Debug)]
pub struct StrategySelectorEngine {
    strategies: HashMap<Uuid, Strategy>,
    selector: StrategySelector,
    attribution: AttributionEngine,
    switcher: StrategySwitcher,
    recommender: StrategyRecommender,
    current_state: Option<StrategyState>,
    regime_history: Vec<(DateTime<Utc>, MarketRegime)>,
    /// HRM for ML-based conviction calculation (Sprint 42)
    hrm: Option<HRM>,
}

impl StrategySelectorEngine {
    /// Create new selector engine
    pub fn new() -> Self {
        Self {
            strategies: HashMap::new(),
            selector: StrategySelector::new(),
            attribution: AttributionEngine::new(),
            switcher: StrategySwitcher::new(SwitchConfig::default()),
            recommender: StrategyRecommender::new(),
            current_state: None,
            regime_history: Vec::new(),
            hrm: None,
        }
    }

    /// Initialize HRM with default config (random weights)
    pub fn with_hrm_default(mut self) -> Result<Self> {
        let hrm = HRM::new(&HRMConfig::default())
            .map_err(|e| SelectorError::SwitchFailed(format!("HRM init failed: {}", e)))?;
        crate::monitoring::metrics::set_hrm_model_loaded(hrm.is_ready());
        self.hrm = Some(hrm);
        Ok(self)
    }

    /// Initialize HRM with loaded weights from SafeTensors
    pub fn with_hrm_weights(mut self, weights_path: &str) -> Result<Self> {
        let hrm = HRMBuilder::new()
            .with_weights(weights_path)
            .build()
            .map_err(|e| SelectorError::SwitchFailed(format!("HRM load failed: {}", e)))?;

        if !hrm.is_ready() {
            let warning = hrm.stats().initialization_warning.unwrap_or_else(|| {
                format!(
                    "model is not ready after loading weights from '{}'",
                    weights_path
                )
            });
            crate::monitoring::metrics::set_hrm_model_loaded(false);
            return Err(SelectorError::SwitchFailed(format!(
                "HRM load failed: {}",
                warning
            )));
        }

        self.hrm = Some(hrm);
        crate::monitoring::metrics::set_hrm_model_loaded(true);
        info!("✅ HRM loaded from: {}", weights_path);
        Ok(self)
    }

    /// Check if HRM is available
    pub fn has_hrm(&self) -> bool {
        self.hrm.is_some()
    }

    /// Calculate conviction using HRM ML model
    ///
    /// Falls back to heuristic calculation if HRM is not available
    pub fn calculate_conviction(&self, signals: &HRMInputSignals) -> ConvictionResult {
        let start = std::time::Instant::now();

        let result = if let Some(ref hrm) = self.hrm {
            if hrm.is_ready() {
                // Use ML model
                let input = signals.to_hrm_input();
                match hrm.infer(&input) {
                    Ok(result) => ConvictionResult {
                        conviction: result.conviction,
                        confidence: result.confidence,
                        regime: Self::convert_regime(result.regime),
                        source: ConvictionSource::MLModel,
                    },
                    Err(e) => {
                        warn!("HRM inference failed: {}. Using heuristic fallback.", e);
                        crate::monitoring::metrics::record_hrm_inference_error();
                        self.calculate_heuristic_conviction(signals)
                    }
                }
            } else {
                warn!("HRM configured but model is not ready. Using heuristic fallback.");
                crate::monitoring::metrics::set_hrm_model_loaded(false);
                self.calculate_heuristic_conviction(signals)
            }
        } else {
            // Use heuristic
            self.calculate_heuristic_conviction(signals)
        };

        // Record metrics
        crate::monitoring::metrics::record_hrm_inference(start.elapsed());

        result
    }

    /// Heuristic conviction calculation (fallback)
    fn calculate_heuristic_conviction(&self, signals: &HRMInputSignals) -> ConvictionResult {
        // Original heuristic formula
        let base_conviction = signals.pegy * 0.3 + signals.insider * 0.3 + signals.sentiment * 0.4;
        let volatility_factor = 1.0 - (signals.vix / 100.0).min(1.0);
        let conviction = base_conviction * volatility_factor;

        // Simple confidence based on signal strength
        let confidence = (signals.pegy + signals.insider + signals.sentiment) / 3.0;

        // Determine regime
        let regime = if signals.regime < 0.5 {
            MarketRegime::StrongUptrend
        } else if signals.regime < 1.5 {
            MarketRegime::StrongDowntrend
        } else if signals.regime < 2.5 {
            MarketRegime::Ranging
        } else {
            MarketRegime::Crisis
        };

        ConvictionResult {
            conviction,
            confidence,
            regime,
            source: ConvictionSource::Heuristic,
        }
    }

    /// Convert HRM regime to StrategySelector regime
    fn convert_regime(hrm_regime: HRMMarketRegime) -> MarketRegime {
        match hrm_regime {
            HRMMarketRegime::Bull => MarketRegime::StrongUptrend,
            HRMMarketRegime::Bear => MarketRegime::StrongDowntrend,
            HRMMarketRegime::Sideways => MarketRegime::Ranging,
            HRMMarketRegime::Crisis => MarketRegime::Crisis,
        }
    }

    /// Register a strategy
    pub fn register_strategy(&mut self, strategy: Strategy) {
        info!(
            "Registering strategy: {} ({:?})",
            strategy.name, strategy.strategy_type
        );
        self.strategies.insert(strategy.id, strategy);
    }

    /// Get all registered strategies
    pub fn get_strategies(&self) -> Vec<&Strategy> {
        self.strategies.values().collect()
    }

    /// Get active strategies only
    pub fn get_active_strategies(&self) -> Vec<&Strategy> {
        self.strategies.values().filter(|s| s.is_active).collect()
    }

    /// Detect current market regime
    pub fn detect_regime(&self, indicators: &MarketIndicators) -> MarketRegime {
        MarketRegime::from_indicators(
            indicators.trend_strength,
            indicators.volatility,
            indicators.volume,
        )
    }

    /// Select best strategy for current regime
    pub fn select_strategy(
        &self,
        regime: MarketRegime,
        criteria: SelectionCriteria,
    ) -> Result<SelectionScore> {
        let active_strategies: Vec<_> = self.get_active_strategies();

        if active_strategies.is_empty() {
            return Err(SelectorError::InsufficientData(
                "No active strategies available".to_string(),
            ));
        }

        self.selector
            .select_best(&active_strategies, regime, criteria)
    }

    /// Evaluate switching to a new strategy
    pub fn evaluate_switch(
        &self,
        current_score: &SelectionScore,
        candidate_score: &SelectionScore,
    ) -> bool {
        self.switcher.should_switch(current_score, candidate_score)
    }

    /// Execute strategy switch
    pub fn execute_switch(
        &mut self,
        strategy_id: Uuid,
        regime: MarketRegime,
        reason: String,
    ) -> Result<StrategyState> {
        let strategy = self.strategies.get(&strategy_id).ok_or_else(|| {
            SelectorError::StrategyNotFound(format!("Strategy {} not found", strategy_id))
        })?;

        // Record switch
        self.switcher
            .record_switch(strategy_id, regime, reason.clone());

        // Create new state
        let new_state = StrategyState {
            strategy_id,
            strategy_type: strategy.strategy_type,
            started_at: Utc::now(),
            current_pnl: Decimal::ZERO,
            trades_executed: 0,
            last_switch_reason: Some(reason),
            current_regime: regime,
        };

        self.current_state = Some(new_state.clone());

        info!(
            "Strategy switch executed: {} -> {:?}",
            strategy.name, regime
        );

        Ok(new_state)
    }

    /// Get strategy recommendations
    pub fn get_recommendations(
        &self,
        capital: Decimal,
        risk_tolerance: RiskTolerance,
    ) -> Vec<Recommendation> {
        let active = self.get_active_strategies();
        self.recommender.recommend(&active, capital, risk_tolerance)
    }

    /// Record performance for attribution
    pub fn record_performance(&mut self, strategy_id: Uuid, pnl: Decimal, trades: u32) {
        self.attribution.record_trade(strategy_id, pnl, trades);

        // Update current state if it's the active strategy
        if let Some(ref mut state) = self.current_state {
            if state.strategy_id == strategy_id {
                state.current_pnl += pnl;
                state.trades_executed += trades;
            }
        }
    }

    /// Get performance attribution
    pub fn get_attribution(&self) -> PerformanceAttribution {
        self.attribution.get_attribution()
    }

    /// Get current state
    pub fn current_state(&self) -> Option<&StrategyState> {
        self.current_state.as_ref()
    }

    /// Get regime history
    pub fn regime_history(&self) -> &[(DateTime<Utc>, MarketRegime)] {
        &self.regime_history
    }

    /// Record regime detection
    pub fn record_regime(&mut self, regime: MarketRegime) {
        self.regime_history.push((Utc::now(), regime));
    }

    /// Get strategy count
    pub fn strategy_count(&self) -> usize {
        self.strategies.len()
    }

    /// Get active strategy count
    pub fn active_count(&self) -> usize {
        self.strategies.values().filter(|s| s.is_active).count()
    }
}

impl Default for StrategySelectorEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for StrategySelectorEngine {
    fn drop(&mut self) {
        // HRM is automatically dropped
        info!("StrategySelectorEngine shutting down");
    }
}

/// Market indicators for regime detection
#[derive(Debug, Clone)]
pub struct MarketIndicators {
    pub trend_strength: f32, // -1.0 to 1.0
    pub volatility: f32,     // 0.0 to 1.0
    pub volume: f32,         // 0.0 to 1.0
    pub rsi: f32,            // 0.0 to 100.0
    pub atr: f32,            // Average True Range
}

/// HRM Input Signals (Sprint 42)
#[derive(Debug, Clone)]
pub struct HRMInputSignals {
    /// PEGY score (0.0 - 1.0)
    pub pegy: f32,
    /// Insider activity score (0.0 - 1.0)
    pub insider: f32,
    /// Market sentiment score (0.0 - 1.0)
    pub sentiment: f32,
    /// VIX value (typically 10-80)
    pub vix: f32,
    /// Market regime (0=Bull, 1=Bear, 2=Sideways, 3=Crisis)
    pub regime: f32,
    /// Time of day (0.0 - 1.0, where 0.5 is midday)
    pub time: f32,
}

impl HRMInputSignals {
    /// Create new signals with validation
    pub fn new(pegy: f32, insider: f32, sentiment: f32, vix: f32, regime: f32, time: f32) -> Self {
        Self {
            pegy: pegy.clamp(0.0, 1.0),
            insider: insider.clamp(0.0, 1.0),
            sentiment: sentiment.clamp(0.0, 1.0),
            vix: vix.max(0.0),
            regime: regime.clamp(0.0, 3.0),
            time: time.clamp(0.0, 1.0),
        }
    }

    /// Convert to HRM input vector
    pub fn to_hrm_input(&self) -> Vec<f32> {
        vec![
            self.pegy,
            self.insider,
            self.sentiment,
            self.vix,
            self.regime,
            self.time,
        ]
    }

    /// Create from MarketIndicators
    pub fn from_indicators(indicators: &MarketIndicators, regime: MarketRegime) -> Self {
        // Normalize VIX: typical range 10-80 mapped to 0-100 scale
        let vix_normalized = indicators.volatility * 100.0;

        // Convert regime enum to numeric value
        let regime_val = match regime {
            MarketRegime::StrongUptrend | MarketRegime::Trending => 0.0,
            MarketRegime::StrongDowntrend => 1.0,
            MarketRegime::Ranging | MarketRegime::Normal => 2.0,
            MarketRegime::Crisis | MarketRegime::Volatile => 3.0,
            _ => 2.0, // Default to sideways
        };

        Self {
            pegy: 0.5,                         // Would come from fundamental analysis
            insider: indicators.volume,        // Proxy from volume
            sentiment: indicators.rsi / 100.0, // Normalize RSI
            vix: vix_normalized,
            regime: regime_val,
            time: 0.5, // Default midday
        }
    }
}

/// Conviction calculation result
#[derive(Debug, Clone)]
pub struct ConvictionResult {
    /// Trading conviction (0.0 - 1.0)
    pub conviction: f32,
    /// Confidence in prediction (0.0 - 1.0)
    pub confidence: f32,
    /// Detected market regime
    pub regime: MarketRegime,
    /// Source of calculation
    pub source: ConvictionSource,
}

impl ConvictionResult {
    /// Check if conviction is high enough to trade
    pub fn should_trade(&self, threshold: f32) -> bool {
        self.conviction >= threshold && self.confidence >= 0.5
    }

    /// Get trading signal strength
    pub fn signal_strength(&self) -> f32 {
        self.conviction * self.confidence
    }
}

/// Source of conviction calculation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConvictionSource {
    /// ML-based HRM model
    MLModel,
    /// Heuristic calculation (fallback)
    Heuristic,
}

/// Risk tolerance levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiskTolerance {
    Conservative,
    Moderate,
    Aggressive,
    Speculative,
}

impl RiskTolerance {
    pub fn max_drawdown(&self) -> f32 {
        match self {
            RiskTolerance::Conservative => 0.05, // 5%
            RiskTolerance::Moderate => 0.10,     // 10%
            RiskTolerance::Aggressive => 0.20,   // 20%
            RiskTolerance::Speculative => 0.35,  // 35%
        }
    }

    pub fn min_sharpe(&self) -> f32 {
        match self {
            RiskTolerance::Conservative => 1.5,
            RiskTolerance::Moderate => 1.0,
            RiskTolerance::Aggressive => 0.5,
            RiskTolerance::Speculative => 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;

    fn create_test_strategy(name: &str, strategy_type: StrategyType) -> Strategy {
        Strategy {
            id: Uuid::new_v4(),
            name: name.to_string(),
            strategy_type,
            description: "Test strategy".to_string(),
            min_capital: Decimal::from(10000),
            max_drawdown: 0.15,
            avg_return: 0.12,
            sharpe_ratio: 1.2,
            win_rate: 0.55,
            trades_per_month: 10,
            created_at: Utc::now(),
            is_active: true,
            current_allocation: 0.0,
        }
    }

    #[test]
    fn test_engine_creation() {
        let engine = StrategySelectorEngine::new();
        assert_eq!(engine.strategy_count(), 0);
    }

    #[test]
    fn test_strategy_registration() {
        let mut engine = StrategySelectorEngine::new();
        let strategy = create_test_strategy("Momentum", StrategyType::Momentum);

        engine.register_strategy(strategy);
        assert_eq!(engine.strategy_count(), 1);
    }

    #[test]
    fn test_regime_detection() {
        let engine = StrategySelectorEngine::new();

        // Strong uptrend
        let indicators = MarketIndicators {
            trend_strength: 0.9,
            volatility: 0.3,
            volume: 0.8,
            rsi: 70.0,
            atr: 2.5,
        };

        let regime = engine.detect_regime(&indicators);
        assert_eq!(regime, MarketRegime::StrongUptrend);
    }

    #[test]
    fn test_strategy_suitability() {
        assert!(StrategyType::Momentum.is_suitable_for(MarketRegime::Trending));
        assert!(!StrategyType::Momentum.is_suitable_for(MarketRegime::Ranging));

        assert!(StrategyType::MeanReversion.is_suitable_for(MarketRegime::Ranging));
        assert!(StrategyType::Arbitrage.is_suitable_for(MarketRegime::Trending));
    }

    #[test]
    fn test_risk_tolerance() {
        assert_eq!(RiskTolerance::Conservative.max_drawdown(), 0.05);
        assert_eq!(RiskTolerance::Aggressive.max_drawdown(), 0.20);

        assert_eq!(RiskTolerance::Conservative.min_sharpe(), 1.5);
        assert_eq!(RiskTolerance::Aggressive.min_sharpe(), 0.5);
    }

    #[test]
    fn test_hrm_input_signals() {
        let signals = HRMInputSignals::new(0.8, 0.9, 0.7, 15.0, 0.0, 0.5);

        assert_eq!(signals.pegy, 0.8);
        assert_eq!(signals.to_hrm_input().len(), 6);

        // Test clamping
        let clamped = HRMInputSignals::new(1.5, -0.5, 0.5, 15.0, 5.0, 2.0);
        assert_eq!(clamped.pegy, 1.0);
        assert_eq!(clamped.insider, 0.0);
        assert_eq!(clamped.regime, 3.0);
        assert_eq!(clamped.time, 1.0);
    }

    #[test]
    fn test_conviction_result() {
        let result = ConvictionResult {
            conviction: 0.8,
            confidence: 0.9,
            regime: MarketRegime::Trending,
            source: ConvictionSource::Heuristic,
        };

        assert!(result.should_trade(0.7));
        assert!(!result.should_trade(0.9));
        // Use approximate comparison for floating point
        assert!((result.signal_strength() - 0.72).abs() < 0.001);
    }

    #[test]
    fn test_engine_with_hrm_default() {
        let engine = StrategySelectorEngine::new()
            .with_hrm_default()
            .expect("HRM initialization failed");

        assert!(engine.has_hrm());

        // Test conviction calculation
        let signals = HRMInputSignals::new(0.8, 0.9, 0.7, 15.0, 0.0, 0.5);
        let conviction = engine.calculate_conviction(&signals);

        assert_eq!(conviction.source, ConvictionSource::Heuristic);
        assert!(conviction.conviction >= 0.0 && conviction.conviction <= 1.0);
        assert!(conviction.confidence >= 0.0 && conviction.confidence <= 1.0);
    }

    #[test]
    fn test_engine_with_hrm_weights_invalid_path_fails_explicitly() {
        let result = StrategySelectorEngine::new()
            .with_hrm_weights("models/__missing_hrm_model__.safetensors");

        assert!(result.is_err());
        let error = result.unwrap_err().to_string();
        assert!(
            error.contains("HRM load failed"),
            "expected explicit HRM load failure, got: {}",
            error
        );
    }

    #[test]
    fn test_engine_without_hrm_fallback() {
        let engine = StrategySelectorEngine::new();

        assert!(!engine.has_hrm());

        // Should still work with heuristic
        let signals = HRMInputSignals::new(0.8, 0.9, 0.7, 15.0, 0.0, 0.5);
        let conviction = engine.calculate_conviction(&signals);

        assert_eq!(conviction.source, ConvictionSource::Heuristic);
        assert!(conviction.conviction > 0.0);
    }

    #[test]
    fn test_heuristic_conviction_calculation() {
        let engine = StrategySelectorEngine::new();

        // Strong bull signals
        let strong_bull = HRMInputSignals::new(0.9, 0.9, 0.9, 10.0, 0.0, 0.5);
        let result_bull = engine.calculate_conviction(&strong_bull);

        // Weak bear signals
        let weak_bear = HRMInputSignals::new(0.2, 0.2, 0.2, 50.0, 1.0, 0.5);
        let result_bear = engine.calculate_conviction(&weak_bear);

        // Bull should have higher conviction
        assert!(result_bull.conviction > result_bear.conviction);

        // Check regimes
        assert_eq!(result_bull.regime, MarketRegime::StrongUptrend);
        assert_eq!(result_bear.regime, MarketRegime::StrongDowntrend);
    }
}
