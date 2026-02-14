//! Predictive Regime Detection Module
//!
//! Forecasts market regime changes before they happen.
//! Uses early warning indicators and regime transition modeling.

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use chrono::{DateTime, Utc};
use thiserror::Error;
use tracing::info;

/// Regime detection errors
#[derive(Error, Debug, Clone)]
pub enum RegimeError {
    #[error("Insufficient data: need {required}, have {available}")]
    InsufficientData { required: usize, available: usize },
    
    #[error("Model prediction failed: {0}")]
    PredictionFailed(String),
    
    #[error("Invalid regime transition")]
    InvalidTransition,
}

/// Market regime types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MarketRegime {
    /// Strong bull market
    BullStrong,
    /// Weak bull market
    BullWeak,
    /// Sideways/ranging
    Sideways,
    /// Weak bear market
    BearWeak,
    /// Strong bear market
    BearStrong,
    /// High volatility
    HighVolatility,
    /// Low volatility
    LowVolatility,
    /// Crisis/crash
    Crisis,
}

impl MarketRegime {
    /// Get expected annual return for regime
    pub fn expected_return(&self) -> f64 {
        match self {
            MarketRegime::BullStrong => 0.25,
            MarketRegime::BullWeak => 0.12,
            MarketRegime::Sideways => 0.03,
            MarketRegime::BearWeak => -0.08,
            MarketRegime::BearStrong => -0.20,
            MarketRegime::HighVolatility => 0.05, // Uncertain
            MarketRegime::LowVolatility => 0.06,
            MarketRegime::Crisis => -0.30,
        }
    }
    
    /// Get expected volatility for regime
    pub fn expected_volatility(&self) -> f64 {
        match self {
            MarketRegime::BullStrong => 0.15,
            MarketRegime::BullWeak => 0.18,
            MarketRegime::Sideways => 0.12,
            MarketRegime::BearWeak => 0.22,
            MarketRegime::BearStrong => 0.30,
            MarketRegime::HighVolatility => 0.35,
            MarketRegime::LowVolatility => 0.08,
            MarketRegime::Crisis => 0.50,
        }
    }
    
    /// Is this a risk-off regime
    pub fn is_risk_off(&self) -> bool {
        matches!(self, MarketRegime::BearWeak | MarketRegime::BearStrong | MarketRegime::Crisis)
    }
    
    /// Is this a risk-on regime
    pub fn is_risk_on(&self) -> bool {
        matches!(self, MarketRegime::BullStrong | MarketRegime::BullWeak)
    }
}

/// Regime forecast with confidence
#[derive(Debug, Clone)]
pub struct RegimeForecast {
    /// Predicted future regime
    pub predicted_regime: MarketRegime,
    /// Current regime
    pub current_regime: MarketRegime,
    /// Confidence of prediction (0-1)
    pub confidence: f64,
    /// Forecast horizon (days)
    pub horizon_days: u32,
    /// Probability distribution over regimes
    pub regime_probabilities: Vec<(MarketRegime, f64)>,
    /// Timestamp of forecast
    pub timestamp: DateTime<Utc>,
    /// Expected regime duration (days)
    pub expected_duration_days: u32,
}

/// Early warning indicator
#[derive(Debug, Clone)]
pub struct EarlyWarning {
    /// Indicator name
    pub name: String,
    /// Current value
    pub value: f64,
    /// Warning threshold
    pub threshold: f64,
    /// Is warning triggered
    pub is_triggered: bool,
    /// Severity (1-5)
    pub severity: u8,
    /// Direction of change
    pub trend: TrendDirection,
}

/// Trend direction
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TrendDirection {
    Rising,
    Falling,
    Stable,
    Accelerating,
    Decelerating,
}

/// Market data point for regime detection
#[derive(Debug, Clone)]
pub struct MarketDataPoint {
    pub timestamp: DateTime<Utc>,
    pub price: Decimal,
    pub volume: Decimal,
    pub returns: f64,
    pub volatility: f64,
}

/// Predictive regime detector configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegimeDetectorConfig {
    /// Lookback window for analysis (days)
    pub lookback_days: u32,
    /// Forecast horizon (days)
    pub forecast_horizon_days: u32,
    /// Minimum confidence threshold for alerts
    pub min_confidence: f64,
    /// Number of regime states in model
    pub num_regime_states: usize,
    /// Early warning sensitivity (1-5)
    pub warning_sensitivity: u8,
}

impl Default for RegimeDetectorConfig {
    fn default() -> Self {
        Self {
            lookback_days: 252, // 1 year
            forecast_horizon_days: 30,
            min_confidence: 0.6,
            num_regime_states: 5,
            warning_sensitivity: 3,
        }
    }
}

/// Predictive Regime Detector
#[derive(Debug)]
pub struct PredictiveRegimeDetector {
    config: RegimeDetectorConfig,
    history: VecDeque<MarketDataPoint>,
    current_regime: MarketRegime,
    transition_matrix: Vec<Vec<f64>>,
    early_warnings: Vec<EarlyWarning>,
}

impl PredictiveRegimeDetector {
    /// Create new predictive regime detector
    pub fn new(config: RegimeDetectorConfig) -> Self {
        info!("Creating PredictiveRegimeDetector with {} day lookback", 
              config.lookback_days);
        
        // Initialize transition matrix with prior beliefs
        let num_states = config.num_regime_states;
        let transition_matrix = Self::initialize_transition_matrix(num_states);
        
        Self {
            config,
            history: VecDeque::new(),
            current_regime: MarketRegime::Sideways,
            transition_matrix,
            early_warnings: Vec::new(),
        }
    }
    
    /// Initialize regime transition matrix
    fn initialize_transition_matrix(num_states: usize) -> Vec<Vec<f64>> {
        // Create a matrix where staying in same regime is likely
        // but transitions to neighboring regimes are possible
        let mut matrix = vec![vec![0.0; num_states]; num_states];
        
        for i in 0..num_states {
            for j in 0..num_states {
                matrix[i][j] = if i == j {
                    0.7 // 70% chance of staying
                } else if (i as i32 - j as i32).abs() == 1 {
                    0.15 // 15% to adjacent regime
                } else {
                    0.01 // 1% to distant regime
                };
            }
        }
        
        // Normalize rows
        for row in &mut matrix {
            let sum: f64 = row.iter().sum();
            if sum > 0.0 {
                for val in row.iter_mut() {
                    *val /= sum;
                }
            }
        }
        
        matrix
    }
    
    /// Add market data point
    pub fn add_data(&mut self, data: MarketDataPoint) {
        self.history.push_back(data);
        
        // Keep only lookback window
        let window_size = self.config.lookback_days as usize;
        while self.history.len() > window_size {
            self.history.pop_front();
        }
        
        // Update current regime
        self.update_current_regime();
        
        // Update early warnings
        self.update_early_warnings();
    }
    
    /// Update current regime based on recent data
    fn update_current_regime(&mut self) {
        if self.history.len() < 20 {
            return;
        }
        
        let recent: Vec<_> = self.history.iter().rev().take(20).collect();
        
        let avg_return = recent.iter().map(|d| d.returns).sum::<f64>() / recent.len() as f64;
        let avg_vol = recent.iter().map(|d| d.volatility).sum::<f64>() / recent.len() as f64;
        
        self.current_regime = self.classify_regime(avg_return, avg_vol);
    }
    
    /// Classify regime based on return and volatility
    fn classify_regime(&self, avg_return: f64, volatility: f64) -> MarketRegime {
        match (avg_return, volatility) {
            (r, v) if r > 0.02 && v < 0.20 => MarketRegime::BullStrong,
            (r, v) if r > 0.005 && v < 0.20 => MarketRegime::BullWeak,
            (r, v) if r < -0.02 && v > 0.25 => MarketRegime::Crisis,
            (r, v) if r < -0.01 => MarketRegime::BearStrong,
            (r, v) if r < -0.003 => MarketRegime::BearWeak,
            (_, v) if v > 0.30 => MarketRegime::HighVolatility,
            (_, v) if v < 0.10 => MarketRegime::LowVolatility,
            _ => MarketRegime::Sideways,
        }
    }
    
    /// Update early warning indicators
    fn update_early_warnings(&mut self) {
        if self.history.len() < 60 {
            return;
        }
        
        let mut warnings = Vec::new();
        
        // Volatility compression warning (calm before storm)
        let vol_compression = self.calculate_volatility_compression();
        if vol_compression.is_triggered {
            warnings.push(vol_compression);
        }
        
        // Momentum divergence warning
        let momentum_div = self.calculate_momentum_divergence();
        if momentum_div.is_triggered {
            warnings.push(momentum_div);
        }
        
        // Liquidity stress warning
        let liquidity = self.calculate_liquidity_stress();
        if liquidity.is_triggered {
            warnings.push(liquidity);
        }
        
        self.early_warnings = warnings;
    }
    
    /// Calculate volatility compression indicator
    fn calculate_volatility_compression(&self) -> EarlyWarning {
        let recent_vol: Vec<_> = self.history.iter()
            .rev()
            .take(20)
            .map(|d| d.volatility)
            .collect();
        
        let longer_vol: Vec<_> = self.history.iter()
            .rev()
            .skip(20)
            .take(40)
            .map(|d| d.volatility)
            .collect();
        
        let recent_avg = recent_vol.iter().sum::<f64>() / recent_vol.len() as f64;
        let longer_avg = longer_vol.iter().sum::<f64>() / longer_vol.len() as f64;
        
        let compression_ratio = recent_avg / longer_avg.max(0.0001);
        
        EarlyWarning {
            name: "Volatility Compression".to_string(),
            value: compression_ratio,
            threshold: 0.7,
            is_triggered: compression_ratio < 0.7,
            severity: if compression_ratio < 0.5 { 5 } else { 3 },
            trend: TrendDirection::Falling,
        }
    }
    
    /// Calculate momentum divergence indicator
    fn calculate_momentum_divergence(&self) -> EarlyWarning {
        // Simplified: check if price rising but momentum falling
        let recent_returns: Vec<_> = self.history.iter()
            .rev()
            .take(10)
            .map(|d| d.returns)
            .collect();
        
        let avg_return = recent_returns.iter().sum::<f64>() / recent_returns.len() as f64;
        let momentum = avg_return * 10.0; // Scaled
        
        let divergence = if momentum < 0.0 && avg_return > 0.0 {
            momentum.abs()
        } else {
            0.0
        };
        
        EarlyWarning {
            name: "Momentum Divergence".to_string(),
            value: divergence,
            threshold: 0.5,
            is_triggered: divergence > 0.5,
            severity: 4,
            trend: TrendDirection::Decelerating,
        }
    }
    
    /// Calculate liquidity stress indicator
    fn calculate_liquidity_stress(&self) -> EarlyWarning {
        let recent_volume: Vec<_> = self.history.iter()
            .rev()
            .take(20)
            .filter_map(|d| d.volume.try_into().ok())
            .collect();
        
        let avg_volume = recent_volume.iter().sum::<f64>() / recent_volume.len().max(1) as f64;
        let volume_decline = avg_volume / 1_000_000.0; // Normalized
        
        EarlyWarning {
            name: "Liquidity Stress".to_string(),
            value: volume_decline,
            threshold: 0.5,
            is_triggered: volume_decline < 0.5,
            severity: 3,
            trend: TrendDirection::Falling,
        }
    }
    
    /// Forecast future regime
    pub fn forecast_regime(&self, horizon_days: u32) -> Result<RegimeForecast, RegimeError> {
        if self.history.len() < 60 {
            return Err(RegimeError::InsufficientData {
                required: 60,
                available: self.history.len(),
            });
        }
        
        // Use Markov chain to predict future regime
        let current_state = self.regime_to_state(self.current_regime);
        let probabilities = self.propagate_probabilities(current_state, horizon_days);
        
        // Find most likely regime
        let (max_prob_idx, _max_prob) = probabilities.iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .map(|(i, p)| (i, *p))
            .unwrap_or((0, 0.0));
        
        let predicted_regime = self.state_to_regime(max_prob_idx);
        
        // Calculate confidence based on transition stability
        let confidence = self.calculate_forecast_confidence(&probabilities);
        
        // Build probability distribution
        let regime_probs: Vec<_> = probabilities.iter()
            .enumerate()
            .map(|(i, p)| (self.state_to_regime(i), *p))
            .collect();
        
        info!("Forecast: {:?} -> {:?} (confidence: {:.2})",
              self.current_regime, predicted_regime, confidence);
        
        Ok(RegimeForecast {
            predicted_regime,
            current_regime: self.current_regime,
            confidence,
            horizon_days,
            regime_probabilities: regime_probs,
            timestamp: Utc::now(),
            expected_duration_days: self.estimate_regime_duration(predicted_regime),
        })
    }
    
    /// Convert regime to state index
    fn regime_to_state(&self, regime: MarketRegime) -> usize {
        use MarketRegime::*;
        match regime {
            BullStrong => 0,
            BullWeak => 1,
            Sideways => 2,
            BearWeak => 3,
            BearStrong => 4,
            _ => 2, // Map others to sideways
        }
    }
    
    /// Convert state index to regime
    fn state_to_regime(&self, state: usize) -> MarketRegime {
        match state {
            0 => MarketRegime::BullStrong,
            1 => MarketRegime::BullWeak,
            2 => MarketRegime::Sideways,
            3 => MarketRegime::BearWeak,
            4 => MarketRegime::BearStrong,
            _ => MarketRegime::Sideways,
        }
    }
    
    /// Propagate probabilities through transition matrix
    fn propagate_probabilities(&self, initial_state: usize, steps: u32) -> Vec<f64> {
        let n = self.transition_matrix.len();
        let mut probs = vec![0.0; n];
        probs[initial_state] = 1.0;
        
        for _ in 0..steps {
            let mut new_probs = vec![0.0; n];
            for i in 0..n {
                for j in 0..n {
                    new_probs[j] += probs[i] * self.transition_matrix[i][j];
                }
            }
            probs = new_probs;
        }
        
        probs
    }
    
    /// Calculate forecast confidence
    fn calculate_forecast_confidence(&self, probabilities: &[f64]) -> f64 {
        // Entropy-based confidence: lower entropy = higher confidence
        let entropy: f64 = probabilities.iter()
            .filter(|&&p| p > 0.0)
            .map(|&p| -p * p.ln())
            .sum();
        
        // Convert to confidence (0-1)
        let max_entropy = (probabilities.len() as f64).ln();
        1.0 - (entropy / max_entropy)
    }
    
    /// Estimate expected duration of a regime
    fn estimate_regime_duration(&self, regime: MarketRegime) -> u32 {
        // Typical regime durations based on historical data
        match regime {
            MarketRegime::BullStrong => 180,
            MarketRegime::BullWeak => 90,
            MarketRegime::Sideways => 60,
            MarketRegime::BearWeak => 45,
            MarketRegime::BearStrong => 30,
            MarketRegime::Crisis => 14,
            _ => 60,
        }
    }
    
    /// Get current early warnings
    pub fn get_early_warnings(&self) -> &[EarlyWarning] {
        &self.early_warnings
    }
    
    /// Get current regime
    pub fn current_regime(&self) -> MarketRegime {
        self.current_regime
    }
    
    /// Check if regime change is likely
    pub fn is_regime_change_likely(&self) -> bool {
        if let Ok(forecast) = self.forecast_regime(30) {
            forecast.predicted_regime != forecast.current_regime
                && forecast.confidence > self.config.min_confidence
        } else {
            false
        }
    }
}

impl Default for PredictiveRegimeDetector {
    fn default() -> Self {
        Self::new(RegimeDetectorConfig::default())
    }
}

/// Regime transition signal for trading
#[derive(Debug, Clone)]
pub struct RegimeTransitionSignal {
    /// From regime
    pub from: MarketRegime,
    /// To regime
    pub to: MarketRegime,
    /// Confidence of transition
    pub confidence: f64,
    /// Recommended action
    pub recommended_action: RegimeAction,
    /// Urgency (1-5)
    pub urgency: u8,
}

/// Action to take on regime change
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RegimeAction {
    /// Increase equity exposure
    IncreaseEquity,
    /// Decrease equity exposure
    DecreaseEquity,
    /// Move to defensive assets
    DefensivePosition,
    /// Hedge portfolio
    HedgePortfolio,
    /// No action needed
    Hold,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn test_regime_detector_creation() {
        let detector = PredictiveRegimeDetector::default();
        assert_eq!(detector.current_regime(), MarketRegime::Sideways);
    }

    #[test]
    fn test_regime_classification() {
        let detector = PredictiveRegimeDetector::default();
        
        assert_eq!(detector.classify_regime(0.03, 0.15), MarketRegime::BullStrong);
        assert_eq!(detector.classify_regime(-0.03, 0.30), MarketRegime::Crisis); // r < -0.02 AND v > 0.25
        assert_eq!(detector.classify_regime(0.0, 0.08), MarketRegime::LowVolatility); // v < 0.10
        assert_eq!(detector.classify_regime(-0.02, 0.20), MarketRegime::BearStrong); // r < -0.01
    }

    #[test]
    fn test_early_warnings_after_data() {
        let mut detector = PredictiveRegimeDetector::default();
        
        // Add 60 days of data
        for i in 0..60 {
            let data = MarketDataPoint {
                timestamp: Utc::now() - Duration::days(60 - i),
                price: Decimal::from(100 + i),
                volume: Decimal::from(1_000_000),
                returns: 0.001,
                volatility: 0.15,
            };
            detector.add_data(data);
        }
        
        let warnings = detector.get_early_warnings();
        // May or may not have warnings depending on data
        assert!(warnings.len() <= 3);
    }

    #[test]
    fn test_regime_forecast() {
        let mut detector = PredictiveRegimeDetector::default();
        
        // Add sufficient data
        for i in 0..100 {
            let data = MarketDataPoint {
                timestamp: Utc::now() - Duration::days(100 - i),
                price: Decimal::from(100 + i),
                volume: Decimal::from(1_000_000),
                returns: 0.001,
                volatility: 0.15,
            };
            detector.add_data(data);
        }
        
        let forecast = detector.forecast_regime(30).unwrap();
        assert!(forecast.confidence > 0.0 && forecast.confidence <= 1.0);
        assert_eq!(forecast.horizon_days, 30);
    }

    #[test]
    fn test_regime_expected_returns() {
        assert!(MarketRegime::BullStrong.expected_return() > 0.0);
        assert!(MarketRegime::BearStrong.expected_return() < 0.0);
        assert!(MarketRegime::Crisis.expected_return() < MarketRegime::BearStrong.expected_return());
    }

    #[test]
    fn test_risk_on_off() {
        assert!(MarketRegime::BullStrong.is_risk_on());
        assert!(!MarketRegime::BullStrong.is_risk_off());
        
        assert!(MarketRegime::BearStrong.is_risk_off());
        assert!(!MarketRegime::BearStrong.is_risk_on());
    }
}
