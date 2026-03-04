//! Machine Learning Pipeline
//!
//! S7-D4: ML feature engineering
//! S7-D5: XGBoost CQ prediction model
//! S7-D6: Anomaly detection

use crate::analytics::{AnalyticsError, Result};

/// ML feature pipeline
pub struct FeaturePipeline;

/// Feature vector for ML model
#[derive(Debug, Clone)]
pub struct FeatureVector {
    pub ticker: String,
    pub features: Vec<f64>,
    pub feature_names: Vec<String>,
    pub label: Option<f64>, // Target variable (e.g., future return)
}

/// Feature engineering for CQ prediction
impl FeaturePipeline {
    /// Generate features from signals
    pub fn generate_features(
        ticker: &str,
        signals: &crate::signals::TickerSignals,
    ) -> FeatureVector {
        let features = vec![
            // Quality features
            signals.quality_score.inner(),
            signals.value_score.inner(),
            signals.momentum_score.inner(),
            // Insider features
            signals.insider_score.inner(),
            signals.insider_flow_ratio,
            if signals.insider_cluster_signal {
                1.0
            } else {
                0.0
            },
            // Sentiment features
            signals.sentiment_score.inner(),
            signals.news_sentiment,
            signals.social_sentiment,
            // Regime features
            signals.regime_fit.inner(),
            signals.vix_level,
            signals.market_breadth,
            // Technical features
            signals.breakout_score,
            signals.atr_trend,
            signals.rsi_14,
            signals.macd_signal,
            // Interactions
            signals.quality_score.inner() * signals.value_score.inner(),
            signals.momentum_score.inner() * signals.regime_fit.inner(),
            signals.insider_score.inner() * signals.sentiment_score.inner(),
        ];

        let feature_names = vec![
            "quality_score".to_string(),
            "value_score".to_string(),
            "momentum_score".to_string(),
            "insider_score".to_string(),
            "insider_flow_ratio".to_string(),
            "insider_cluster".to_string(),
            "sentiment_score".to_string(),
            "news_sentiment".to_string(),
            "social_sentiment".to_string(),
            "regime_fit".to_string(),
            "vix_level".to_string(),
            "market_breadth".to_string(),
            "breakout_score".to_string(),
            "atr_trend".to_string(),
            "rsi_14".to_string(),
            "macd_signal".to_string(),
            "quality_x_value".to_string(),
            "momentum_x_regime".to_string(),
            "insider_x_sentiment".to_string(),
        ];

        FeatureVector {
            ticker: ticker.to_string(),
            features,
            feature_names,
            label: None,
        }
    }

    /// Generate time-series features
    pub fn generate_ts_features(price_history: &[f64]) -> Vec<f64> {
        if price_history.len() < 20 {
            return vec![0.0; 10];
        }

        // Returns
        let returns_1d = Self::calculate_return(price_history, 1);
        let returns_5d = Self::calculate_return(price_history, 5);
        let returns_20d = Self::calculate_return(price_history, 20);

        // Volatility
        let volatility = Self::calculate_volatility(price_history, 20);

        // Moving averages
        let sma_20 = Self::calculate_sma(price_history, 20);
        let sma_50 = if price_history.len() >= 50 {
            Self::calculate_sma(price_history, 50)
        } else {
            0.0
        };

        // RSI
        let rsi = Self::calculate_rsi(price_history, 14);

        vec![
            returns_1d,
            returns_5d,
            returns_20d,
            volatility,
            sma_20,
            sma_50,
            rsi,
            price_history.last().copied().unwrap_or(0.0) / sma_20 - 1.0, // Distance from SMA
        ]
    }

    /// Normalize features to [0, 1] range
    pub fn normalize(features: &[f64]) -> Vec<f64> {
        let min = features.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = features.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

        let range = max - min;
        if range == 0.0 {
            return features.to_vec();
        }

        features.iter().map(|f| (f - min) / range).collect()
    }

    // Private helper methods

    fn calculate_return(prices: &[f64], period: usize) -> f64 {
        if prices.len() < period + 1 {
            return 0.0;
        }

        let current = prices[prices.len() - 1];
        let past = prices[prices.len() - 1 - period];

        if past == 0.0 {
            return 0.0;
        }

        (current - past) / past
    }

    fn calculate_volatility(prices: &[f64], period: usize) -> f64 {
        if prices.len() < period {
            return 0.0;
        }

        let returns: Vec<f64> = prices[prices.len() - period..]
            .windows(2)
            .map(|w| (w[1] - w[0]) / w[0])
            .collect();

        if returns.is_empty() {
            return 0.0;
        }

        let mean = returns.iter().sum::<f64>() / returns.len() as f64;
        let variance =
            returns.iter().map(|r| (r - mean) * (r - mean)).sum::<f64>() / returns.len() as f64;

        variance.sqrt()
    }

    fn calculate_sma(prices: &[f64], period: usize) -> f64 {
        if prices.len() < period {
            return 0.0;
        }

        let sum: f64 = prices[prices.len() - period..].iter().sum();
        sum / period as f64
    }

    fn calculate_rsi(prices: &[f64], period: usize) -> f64 {
        if prices.len() < period + 1 {
            return 50.0;
        }

        let mut gains = 0.0;
        let mut losses = 0.0;

        for i in prices.len() - period..prices.len() {
            let change = prices[i] - prices[i - 1];
            if change > 0.0 {
                gains += change;
            } else {
                losses += change.abs();
            }
        }

        let avg_gain = gains / period as f64;
        let avg_loss = losses / period as f64;

        if avg_loss == 0.0 {
            return 100.0;
        }

        let rs = avg_gain / avg_loss;
        100.0 - (100.0 / (1.0 + rs))
    }
}

/// XGBoost-style CQ prediction model (simplified)
pub struct CQPredictor {
    weights: Vec<f64>,
    bias: f64,
    threshold: f64,
}

impl CQPredictor {
    /// Create a new predictor with default weights
    pub fn new() -> Self {
        // Simplified weights - in production would be trained
        Self {
            weights: vec![
                0.25,  // quality_score
                0.20,  // value_score
                0.20,  // momentum_score
                0.10,  // insider_score
                0.05,  // insider_flow_ratio
                0.05,  // insider_cluster
                0.10,  // sentiment_score
                0.03,  // news_sentiment
                0.02,  // social_sentiment
                0.10,  // regime_fit
                -0.05, // vix_level
                0.05,  // market_breadth
                0.15,  // breakout_score
                0.05,  // atr_trend
                0.05,  // rsi_14
                0.05,  // macd_signal
                0.10,  // quality_x_value
                0.05,  // momentum_x_regime
                0.05,  // insider_x_sentiment
            ],
            bias: 0.0,
            threshold: 0.65, // CQ threshold
        }
    }

    /// Predict CQ score from features
    pub fn predict(&self, features: &[f64]) -> Result<f64> {
        if features.len() != self.weights.len() {
            return Err(AnalyticsError::InvalidParameters(format!(
                "Expected {} features, got {}",
                self.weights.len(),
                features.len()
            )));
        }

        let weighted_sum: f64 = features
            .iter()
            .zip(self.weights.iter())
            .map(|(f, w)| f * w)
            .sum();

        // Sigmoid activation for [0, 1] output
        let score = Self::sigmoid(weighted_sum + self.bias);

        Ok(score)
    }

    /// Predict with confidence
    pub fn predict_with_confidence(&self, features: &[f64]) -> Result<(f64, f64)> {
        let score = self.predict(features)?;

        // Confidence based on distance from threshold
        let confidence = (score - self.threshold).abs() * 2.0;
        let confidence = confidence.min(1.0).max(0.0);

        Ok((score, confidence))
    }

    /// Check if prediction is above threshold
    pub fn should_trade(&self, features: &[f64]) -> Result<bool> {
        let score = self.predict(features)?;
        Ok(score > self.threshold)
    }

    /// Feature importance
    pub fn feature_importance(&self, feature_names: &[String]) -> Vec<(String, f64)> {
        self.weights
            .iter()
            .zip(feature_names.iter())
            .map(|(w, name)| (name.clone(), w.abs()))
            .collect()
    }

    fn sigmoid(x: f64) -> f64 {
        1.0 / (1.0 + (-x).exp())
    }
}

impl Default for CQPredictor {
    fn default() -> Self {
        Self::new()
    }
}

/// Anomaly detector for regime changes
pub struct AnomalyDetector {
    threshold: f64,
    lookback: usize,
    baseline_mean: Option<f64>,
    baseline_std: Option<f64>,
}

impl AnomalyDetector {
    /// Create new anomaly detector
    pub fn new(threshold: f64, lookback: usize) -> Self {
        Self {
            threshold,
            lookback,
            baseline_mean: None,
            baseline_std: None,
        }
    }

    /// Set baseline statistics
    pub fn set_baseline(&mut self, data: &[f64]) {
        if data.is_empty() {
            return;
        }

        let mean = data.iter().sum::<f64>() / data.len() as f64;
        let variance =
            data.iter().map(|x| (x - mean) * (x - mean)).sum::<f64>() / data.len() as f64;
        let std = variance.sqrt();

        self.baseline_mean = Some(mean);
        self.baseline_std = Some(std);
    }

    /// Detect anomalies in new data point
    pub fn detect(&self, value: f64) -> AnomalyResult {
        let (mean, std) = match (self.baseline_mean, self.baseline_std) {
            (Some(m), Some(s)) => (m, s),
            _ => return AnomalyResult::Normal,
        };

        if std == 0.0 {
            return AnomalyResult::Normal;
        }

        let z_score = (value - mean) / std;

        if z_score.abs() > self.threshold {
            AnomalyResult::Anomaly {
                z_score,
                severity: if z_score.abs() > self.threshold * 2.0 {
                    AnomalySeverity::High
                } else {
                    AnomalySeverity::Medium
                },
            }
        } else {
            AnomalyResult::Normal
        }
    }

    /// Detect regime change using change point detection
    pub fn detect_regime_change(
        &self,
        recent_data: &[f64],
        previous_data: &[f64],
    ) -> Option<RegimeChange> {
        if recent_data.len() < self.lookback || previous_data.len() < self.lookback {
            return None;
        }

        let recent_mean = recent_data.iter().sum::<f64>() / recent_data.len() as f64;
        let previous_mean = previous_data.iter().sum::<f64>() / previous_data.len() as f64;

        let recent_std = Self::calculate_std(recent_data);
        let previous_std = Self::calculate_std(previous_data);

        // Welch's t-test for difference in means
        let t_stat = (recent_mean - previous_mean)
            / (recent_std.powf(2.0) / recent_data.len() as f64
                + previous_std.powf(2.0) / previous_data.len() as f64)
                .sqrt();

        if t_stat.abs() > self.threshold {
            Some(RegimeChange {
                timestamp: chrono::Utc::now(),
                t_statistic: t_stat,
                old_regime_mean: previous_mean,
                new_regime_mean: recent_mean,
            })
        } else {
            None
        }
    }

    fn calculate_std(data: &[f64]) -> f64 {
        let mean = data.iter().sum::<f64>() / data.len() as f64;
        let variance =
            data.iter().map(|x| (x - mean) * (x - mean)).sum::<f64>() / data.len() as f64;
        variance.sqrt()
    }
}

/// Anomaly detection result
#[derive(Debug, Clone)]
pub enum AnomalyResult {
    Normal,
    Anomaly {
        z_score: f64,
        severity: AnomalySeverity,
    },
}

/// Anomaly severity
#[derive(Debug, Clone, Copy)]
pub enum AnomalySeverity {
    Low,
    Medium,
    High,
}

/// Regime change detection
#[derive(Debug, Clone)]
pub struct RegimeChange {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub t_statistic: f64,
    pub old_regime_mean: f64,
    pub new_regime_mean: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cq_predictor() {
        let predictor = CQPredictor::new();
        let features = vec![0.5; 19]; // All features at 0.5

        let score = predictor.predict(&features).unwrap();

        // Score should be in [0, 1]
        assert!(score >= 0.0 && score <= 1.0);
    }

    #[test]
    fn test_anomaly_detector() {
        let mut detector = AnomalyDetector::new(2.0, 20);

        // Set baseline with normal data (with some variation)
        let baseline: Vec<f64> = (0..100).map(|i| 10.0 + (i as f64 * 0.01)).collect();
        detector.set_baseline(&baseline);

        // Normal value
        let result = detector.detect(10.5);
        assert!(matches!(result, AnomalyResult::Normal));

        // Anomalous value
        let result = detector.detect(20.0);
        assert!(matches!(result, AnomalyResult::Anomaly { .. }));
    }

    #[test]
    fn test_feature_normalization() {
        let features = vec![0.0, 50.0, 100.0];
        let normalized = FeaturePipeline::normalize(&features);

        assert_eq!(normalized[0], 0.0);
        assert_eq!(normalized[2], 1.0);
    }
}
