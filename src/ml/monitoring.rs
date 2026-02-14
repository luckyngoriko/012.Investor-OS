//! Model Performance Monitoring

use chrono::{DateTime, Duration, Utc};
use rust_decimal::Decimal;
use std::collections::{HashMap, VecDeque};
use tracing::warn;

use super::Prediction;

/// Performance metrics for model monitoring
#[derive(Debug, Clone, Default)]
pub struct PerformanceMetrics {
    /// Total predictions made
    pub total_predictions: u64,
    /// Correct predictions (for classification)
    pub correct_predictions: u64,
    /// Mean absolute error (for regression)
    pub mae: Decimal,
    /// Mean squared error
    pub mse: Decimal,
    /// Root mean squared error
    pub rmse: Decimal,
    /// Prediction latency (ms)
    pub avg_latency_ms: Decimal,
    /// Last updated
    pub last_updated: DateTime<Utc>,
}

/// Drift detection configuration
#[derive(Debug, Clone)]
pub struct DriftConfig {
    /// Window size for drift detection
    pub window_size: usize,
    /// Threshold for drift detection (e.g., 0.05 for 5%)
    pub drift_threshold: Decimal,
    /// Minimum samples before drift detection
    pub min_samples: usize,
}

impl Default for DriftConfig {
    fn default() -> Self {
        Self {
            window_size: 100,
            drift_threshold: Decimal::try_from(0.05).unwrap(),
            min_samples: 50,
        }
    }
}

/// Drift detector for monitoring feature and prediction drift
#[derive(Debug, Clone)]
pub struct DriftDetector {
    config: DriftConfig,
    /// Reference distribution (baseline)
    reference_distribution: HashMap<String, VecDeque<Decimal>>,
    /// Current distribution
    current_distribution: HashMap<String, VecDeque<Decimal>>,
    /// Drift detected flag
    drift_detected: bool,
    /// Last drift check
    last_check: DateTime<Utc>,
}

impl DriftDetector {
    pub fn new(config: DriftConfig) -> Self {
        Self {
            config,
            reference_distribution: HashMap::new(),
            current_distribution: HashMap::new(),
            drift_detected: false,
            last_check: Utc::now(),
        }
    }

    /// Set reference distribution from training data
    pub fn set_reference(&mut self, feature_name: String, values: Vec<Decimal>) {
        let deque: VecDeque<Decimal> = values.into_iter().collect();
        self.reference_distribution.insert(feature_name, deque);
    }

    /// Add new observation
    pub fn observe(&mut self, feature_name: &str, value: Decimal) {
        let entry = self.current_distribution
            .entry(feature_name.to_string())
            .or_insert_with(|| VecDeque::with_capacity(self.config.window_size));

        entry.push_back(value);
        if entry.len() > self.config.window_size {
            entry.pop_front();
        }
    }

    /// Check for drift in a specific feature
    pub fn check_drift(&mut self, feature_name: &str) -> Option<Decimal> {
        let reference = self.reference_distribution.get(feature_name)?;
        let current = self.current_distribution.get(feature_name)?;

        if current.len() < self.config.min_samples {
            return None;
        }

        // Calculate means
        let ref_mean = reference.iter().sum::<Decimal>() / Decimal::from(reference.len() as i64);
        let curr_mean = current.iter().sum::<Decimal>() / Decimal::from(current.len() as i64);

        // Calculate relative drift
        if ref_mean.is_zero() {
            return None;
        }

        let drift = (curr_mean - ref_mean).abs() / ref_mean.abs();
        
        if drift > self.config.drift_threshold {
            warn!(
                "Drift detected in {}: {}% > {}%",
                feature_name,
                drift * Decimal::from(100),
                self.config.drift_threshold * Decimal::from(100)
            );
            self.drift_detected = true;
        }

        self.last_check = Utc::now();
        Some(drift)
    }

    /// Check all features for drift
    pub fn check_all(&mut self) -> HashMap<String, Decimal> {
        let feature_names: Vec<String> = self.reference_distribution.keys().cloned().collect();
        let mut results = HashMap::new();

        for name in feature_names {
            if let Some(drift) = self.check_drift(&name) {
                results.insert(name, drift);
            }
        }

        results
    }

    /// Reset current distribution
    pub fn reset(&mut self) {
        self.current_distribution.clear();
        self.drift_detected = false;
    }

    /// Is drift currently detected?
    pub fn is_drift_detected(&self) -> bool {
        self.drift_detected
    }
}

/// Model monitor for tracking performance over time
#[derive(Debug)]
pub struct ModelMonitor {
    model_id: String,
    /// Historical predictions
    prediction_history: VecDeque<(DateTime<Utc>, Prediction, Option<Decimal>)>, // timestamp, prediction, actual
    /// Performance metrics
    metrics: PerformanceMetrics,
    /// Drift detector
    drift_detector: DriftDetector,
    /// Performance degradation threshold
    degradation_threshold: Decimal,
    /// Alert cooldown period
    alert_cooldown: Duration,
    /// Last alert time
    last_alert: Option<DateTime<Utc>>,
}

impl ModelMonitor {
    pub fn new(model_id: String, drift_config: DriftConfig, degradation_threshold: Decimal) -> Self {
        Self {
            model_id,
            prediction_history: VecDeque::with_capacity(10000),
            metrics: PerformanceMetrics {
                last_updated: Utc::now(),
                ..Default::default()
            },
            drift_detector: DriftDetector::new(drift_config),
            degradation_threshold,
            alert_cooldown: Duration::minutes(15),
            last_alert: None,
        }
    }

    /// Record a prediction
    pub fn record_prediction(&mut self, prediction: Prediction, latency_ms: u64) {
        self.prediction_history.push_back((Utc::now(), prediction, None));
        self.metrics.total_predictions += 1;
        self.metrics.avg_latency_ms = 
            (self.metrics.avg_latency_ms * Decimal::from(self.metrics.total_predictions as i64 - 1)
            + Decimal::from(latency_ms as i64))
            / Decimal::from(self.metrics.total_predictions as i64);
        
        // Trim history if too large
        if self.prediction_history.len() > 10000 {
            self.prediction_history.pop_front();
        }
    }

    /// Record prediction with actual outcome
    pub fn record_outcome(&mut self, prediction: Prediction, actual: Decimal, latency_ms: u64) {
        let now = Utc::now();
        self.prediction_history.push_back((now, prediction.clone(), Some(actual)));
        
        self.metrics.total_predictions += 1;
        self.metrics.avg_latency_ms = 
            (self.metrics.avg_latency_ms * Decimal::from(self.metrics.total_predictions as i64 - 1)
            + Decimal::from(latency_ms as i64))
            / Decimal::from(self.metrics.total_predictions as i64);

        // Update error metrics
        let error = (prediction.value - actual).abs();
        let sq_error = error * error;
        
        let n = Decimal::from(self.metrics.total_predictions as i64);
        self.metrics.mae = (self.metrics.mae * (n - Decimal::ONE) + error) / n;
        self.metrics.mse = (self.metrics.mse * (n - Decimal::ONE) + sq_error) / n;
        self.metrics.rmse = approx_sqrt(self.metrics.mse);

        // Check for correct prediction (if classification with known thresholds)
        if error < Decimal::try_from(0.1).unwrap() {
            self.metrics.correct_predictions += 1;
        }

        self.metrics.last_updated = now;

        // Check for degradation
        self.check_degradation();
    }

    /// Get current metrics
    pub fn get_metrics(&self) -> &PerformanceMetrics {
        &self.metrics
    }

    /// Calculate rolling accuracy over last N predictions
    pub fn rolling_accuracy(&self, window: usize) -> Option<Decimal> {
        let recent: Vec<_> = self.prediction_history.iter().rev().take(window).collect();
        
        if recent.is_empty() {
            return None;
        }

        let with_outcomes: Vec<_> = recent.iter()
            .filter(|(_, _, actual)| actual.is_some())
            .collect();

        if with_outcomes.is_empty() {
            return None;
        }

        let correct = with_outcomes.iter()
            .filter(|(_, pred, actual)| {
                let actual = actual.unwrap();
                (pred.value - actual).abs() < Decimal::try_from(0.1).unwrap()
            })
            .count();

        Some(Decimal::from(correct as i64) / Decimal::from(with_outcomes.len() as i64))
    }

    /// Check if model performance has degraded
    fn check_degradation(&mut self) {
        if self.metrics.total_predictions < 100 {
            return;
        }

        // Calculate recent accuracy
        if let Some(recent_accuracy) = self.rolling_accuracy(100) {
            let baseline_accuracy = Decimal::try_from(0.8).unwrap(); // Expected baseline
            
            if recent_accuracy < baseline_accuracy - self.degradation_threshold {
                // Check cooldown
                if let Some(last) = self.last_alert {
                    if Utc::now() - last < self.alert_cooldown {
                        return;
                    }
                }

                warn!(
                    "Model {} performance degraded: recent accuracy {}% < baseline {}%",
                    self.model_id,
                    recent_accuracy * Decimal::from(100),
                    baseline_accuracy * Decimal::from(100)
                );

                self.last_alert = Some(Utc::now());
            }
        }
    }

    /// Get drift detector reference
    pub fn drift_detector(&self) -> &DriftDetector {
        &self.drift_detector
    }

    /// Get mutable drift detector
    pub fn drift_detector_mut(&mut self) -> &mut DriftDetector {
        &mut self.drift_detector
    }

    /// Get prediction history
    pub fn history(&self) -> &VecDeque<(DateTime<Utc>, Prediction, Option<Decimal>)> {
        &self.prediction_history
    }

    /// Clear history
    pub fn clear_history(&mut self) {
        self.prediction_history.clear();
        self.metrics = PerformanceMetrics {
            last_updated: Utc::now(),
            ..Default::default()
        };
    }

    /// Should retrain model?
    pub fn should_retrain(&self) -> bool {
        if self.drift_detector.is_drift_detected() {
            return true;
        }

        if let Some(recent_accuracy) = self.rolling_accuracy(100) {
            let baseline = Decimal::try_from(0.75).unwrap();
            return recent_accuracy < baseline;
        }

        false
    }
}

fn approx_sqrt(value: Decimal) -> Decimal {
    if value.is_zero() {
        return Decimal::ZERO;
    }

    let mut low = Decimal::ZERO;
    let mut high = value.max(Decimal::ONE);
    let epsilon = Decimal::try_from(0.0001).unwrap();

    if value < Decimal::ONE {
        low = value;
        high = Decimal::ONE;
    }

    for _ in 0..50 {
        let mid = (low + high) / Decimal::from(2);
        let mid_sq = mid * mid;

        if (mid_sq - value).abs() < epsilon {
            return mid;
        }

        if mid_sq < value {
            low = mid;
        } else {
            high = mid;
        }
    }

    (low + high) / Decimal::from(2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_drift_detector() {
        let mut detector = DriftDetector::new(DriftConfig::default());
        
        // Set reference distribution
        detector.set_reference("feature1".to_string(), vec![
            Decimal::from(100), Decimal::from(101), Decimal::from(99), Decimal::from(100), Decimal::from(102)
        ]);

        // Add current observations with slight drift
        for _ in 0..60 {
            detector.observe("feature1", Decimal::from(105)); // Higher values
        }

        let drift = detector.check_drift("feature1").unwrap();
        assert!(drift > Decimal::ZERO);
    }

    #[test]
    fn test_model_monitor() {
        let mut monitor = ModelMonitor::new(
            "test_model".to_string(),
            DriftConfig::default(),
            Decimal::try_from(0.1).unwrap(),
        );

        // Record some predictions
        for i in 0..10 {
            let pred = Prediction::new(Decimal::from(i), Decimal::try_from(0.9).unwrap());
            monitor.record_prediction(pred, 10);
        }

        assert_eq!(monitor.get_metrics().total_predictions, 10);
    }

    #[test]
    fn test_model_monitor_with_outcomes() {
        let mut monitor = ModelMonitor::new(
            "test_model".to_string(),
            DriftConfig::default(),
            Decimal::try_from(0.1).unwrap(),
        );

        // Record predictions with outcomes
        for i in 0..10 {
            let pred = Prediction::new(Decimal::from(i), Decimal::try_from(0.9).unwrap());
            monitor.record_outcome(pred, Decimal::from(i), 10); // Perfect predictions
        }

        assert_eq!(monitor.get_metrics().total_predictions, 10);
        assert_eq!(monitor.get_metrics().correct_predictions, 10);
    }

    #[test]
    fn test_rolling_accuracy() {
        let mut monitor = ModelMonitor::new(
            "test_model".to_string(),
            DriftConfig::default(),
            Decimal::try_from(0.1).unwrap(),
        );

        // 5 correct, 5 incorrect
        for i in 0..5 {
            let pred = Prediction::new(Decimal::from(i), Decimal::ONE);
            monitor.record_outcome(pred, Decimal::from(i), 10); // Correct
        }
        for i in 0..5 {
            let pred = Prediction::new(Decimal::from(i + 100), Decimal::ONE);
            monitor.record_outcome(pred, Decimal::from(i), 10); // Incorrect
        }

        let accuracy = monitor.rolling_accuracy(10).unwrap();
        assert_eq!(accuracy, Decimal::try_from(0.5).unwrap());
    }

    #[test]
    fn test_should_retrain() {
        let mut monitor = ModelMonitor::new(
            "test_model".to_string(),
            DriftConfig::default(),
            Decimal::try_from(0.1).unwrap(),
        );

        // Initially no retrain needed
        assert!(!monitor.should_retrain());

        // Simulate drift
        monitor.drift_detector_mut().drift_detected = true;
        assert!(monitor.should_retrain());
    }
}
