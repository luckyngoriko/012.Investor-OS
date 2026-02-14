//! Anomaly Detection
//!
//! Detects anomalies in metric patterns

use chrono::{DateTime, Utc};
use std::collections::HashMap;
use tracing::debug;

use super::Metric;

/// Anomaly detector
#[derive(Debug)]
pub struct AnomalyDetector {
    thresholds: HashMap<String, AnomalyThreshold>,
    z_score_threshold: f64,
    min_data_points: usize,
    anomaly_count: usize,
}

/// Anomaly threshold configuration
#[derive(Debug, Clone)]
pub struct AnomalyThreshold {
    pub spike_threshold: f64,      // Percentage increase
    pub drop_threshold: f64,       // Percentage decrease
    pub volatility_threshold: f64, // Standard deviation multiplier
}

impl Default for AnomalyThreshold {
    fn default() -> Self {
        Self {
            spike_threshold: 20.0,   // 20% spike
            drop_threshold: -20.0,   // 20% drop
            volatility_threshold: 3.0, // 3 sigma
        }
    }
}

/// Type of anomaly
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnomalyType {
    Spike,      // Sudden increase
    Drop,       // Sudden decrease
    Trend,      // Sustained trend
    Volatility, // Increased volatility
    Critical,   // Critical threshold breach
}

/// Detection result
#[derive(Debug, Clone)]
pub struct DetectionResult {
    pub metric_key: String,
    pub anomaly_type: AnomalyType,
    pub score: f64,         // 0.0 to 1.0
    pub value: f64,
    pub expected_value: f64,
    pub timestamp: DateTime<Utc>,
    pub description: String,
}

impl AnomalyDetector {
    /// Create new detector
    pub fn new() -> Self {
        Self {
            thresholds: HashMap::new(),
            z_score_threshold: 3.0,
            min_data_points: 10,
            anomaly_count: 0,
        }
    }
    
    /// Set threshold for metric
    pub fn set_threshold(&mut self, metric_key: &str, threshold: AnomalyThreshold) {
        self.thresholds.insert(metric_key.to_string(), threshold);
    }
    
    /// Detect anomalies in metric
    pub fn detect(&mut self, metric: &Metric) -> Option<DetectionResult> {
        let key = if let Some(ref symbol) = metric.symbol {
            format!("{}:{}", symbol, metric.metric_type.name())
        } else {
            metric.metric_type.name().to_string()
        };
        
        // Need minimum data points
        if metric.history.len() < self.min_data_points {
            return None;
        }
        
        let threshold = self.thresholds.get(&key).cloned().unwrap_or_default();
        
        // Check for spike/drop
        if let Some(result) = self.detect_spike_drop(metric, &key, &threshold) {
            self.anomaly_count += 1;
            return Some(result);
        }
        
        // Check for volatility anomaly
        if let Some(result) = self.detect_volatility(metric, &key, &threshold) {
            self.anomaly_count += 1;
            return Some(result);
        }
        
        // Check for trend anomaly
        if let Some(result) = self.detect_trend(metric, &key) {
            self.anomaly_count += 1;
            return Some(result);
        }
        
        None
    }
    
    /// Detect spike or drop
    fn detect_spike_drop(
        &self,
        metric: &Metric,
        key: &str,
        threshold: &AnomalyThreshold,
    ) -> Option<DetectionResult> {
        let current = metric.current_value;
        let previous = metric.previous_value;
        
        if previous == 0.0 {
            return None;
        }
        
        let change_pct = ((current - previous) / previous.abs()) * 100.0;
        
        if change_pct > threshold.spike_threshold {
            return Some(DetectionResult {
                metric_key: key.to_string(),
                anomaly_type: AnomalyType::Spike,
                score: (change_pct / 100.0).min(1.0),
                value: current,
                expected_value: previous,
                timestamp: Utc::now(),
                description: format!(
                    "Spike detected: {:.1}% increase (threshold: {:.1}%)",
                    change_pct, threshold.spike_threshold
                ),
            });
        }
        
        if change_pct < threshold.drop_threshold {
            return Some(DetectionResult {
                metric_key: key.to_string(),
                anomaly_type: AnomalyType::Drop,
                score: (change_pct.abs() / 100.0).min(1.0),
                value: current,
                expected_value: previous,
                timestamp: Utc::now(),
                description: format!(
                    "Drop detected: {:.1}% decrease (threshold: {:.1}%)",
                    change_pct.abs(), threshold.drop_threshold.abs()
                ),
            });
        }
        
        None
    }
    
    /// Detect volatility anomaly
    fn detect_volatility(
        &self,
        metric: &Metric,
        key: &str,
        threshold: &AnomalyThreshold,
    ) -> Option<DetectionResult> {
        use chrono::Duration;
        
        let values: Vec<f64> = metric.history.iter()
            .map(|p| p.value)
            .collect();
        
        if values.len() < 10 {
            return None;
        }
        
        // Calculate rolling volatility
        let window_size = 10;
        let recent_values = &values[values.len() - window_size..];
        
        let mean = recent_values.iter().sum::<f64>() / recent_values.len() as f64;
        let variance = recent_values.iter()
            .map(|v| (v - mean).powi(2))
            .sum::<f64>() / recent_values.len() as f64;
        let std_dev = variance.sqrt();
        
        // Calculate historical volatility
        let historical_values = &values[..values.len() - window_size];
        if historical_values.len() < 10 {
            return None;
        }
        
        let hist_mean = historical_values.iter().sum::<f64>() / historical_values.len() as f64;
        let hist_variance = historical_values.iter()
            .map(|v| (v - hist_mean).powi(2))
            .sum::<f64>() / historical_values.len() as f64;
        let hist_std_dev = hist_variance.sqrt();
        
        if hist_std_dev == 0.0 {
            return None;
        }
        
        let volatility_ratio = std_dev / hist_std_dev;
        
        if volatility_ratio > threshold.volatility_threshold {
            return Some(DetectionResult {
                metric_key: key.to_string(),
                anomaly_type: AnomalyType::Volatility,
                score: (volatility_ratio / 5.0).min(1.0),
                value: std_dev,
                expected_value: hist_std_dev,
                timestamp: Utc::now(),
                description: format!(
                    "Volatility increased: {:.2}x normal (threshold: {:.1}x)",
                    volatility_ratio, threshold.volatility_threshold
                ),
            });
        }
        
        None
    }
    
    /// Detect trend anomaly
    fn detect_trend(&self, metric: &Metric, key: &str) -> Option<DetectionResult> {
        let values: Vec<f64> = metric.history.iter()
            .map(|p| p.value)
            .collect();
        
        if values.len() < 20 {
            return None;
        }
        
        // Simple linear regression
        let n = values.len() as f64;
        let half = values.len() / 2;
        
        let first_half_avg = values[..half].iter().sum::<f64>() / half as f64;
        let second_half_avg = values[half..].iter().sum::<f64>() / (values.len() - half) as f64;
        
        let trend = second_half_avg - first_half_avg;
        let trend_pct = if first_half_avg != 0.0 {
            (trend / first_half_avg.abs()) * 100.0
        } else {
            0.0
        };
        
        // Significant sustained trend
        if trend_pct.abs() > 15.0 {
            let trend_type = if trend > 0.0 { "upward" } else { "downward" };
            
            return Some(DetectionResult {
                metric_key: key.to_string(),
                anomaly_type: AnomalyType::Trend,
                score: (trend_pct.abs() / 50.0).min(1.0),
                value: second_half_avg,
                expected_value: first_half_avg,
                timestamp: Utc::now(),
                description: format!(
                    "Sustained {} trend: {:.1}% change over period",
                    trend_type, trend_pct.abs()
                ),
            });
        }
        
        None
    }
    
    /// Detect using z-score
    pub fn detect_zscore(&self, values: &[f64], new_value: f64) -> Option<f64> {
        if values.len() < 5 {
            return None;
        }
        
        let mean = values.iter().sum::<f64>() / values.len() as f64;
        let variance = values.iter()
            .map(|v| (v - mean).powi(2))
            .sum::<f64>() / values.len() as f64;
        let std_dev = variance.sqrt();
        
        if std_dev == 0.0 {
            return None;
        }
        
        let z_score = (new_value - mean) / std_dev;
        
        if z_score.abs() > self.z_score_threshold {
            Some(z_score)
        } else {
            None
        }
    }
    
    /// Get anomaly count
    pub fn count(&self) -> usize {
        self.anomaly_count
    }
    
    /// Set z-score threshold
    pub fn set_zscore_threshold(&mut self, threshold: f64) {
        self.z_score_threshold = threshold.max(1.0);
    }
    
    /// Set minimum data points
    pub fn set_min_data_points(&mut self, min: usize) {
        self.min_data_points = min.max(5);
    }
}

impl Default for AnomalyDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn create_test_metric(values: Vec<f64>) -> Metric {
        let mut metric = Metric::new(super::super::MetricType::Pnl);
        for value in values {
            metric.update(value);
        }
        metric
    }

    #[test]
    fn test_detector_creation() {
        let detector = AnomalyDetector::new();
        assert_eq!(detector.count(), 0);
        assert_eq!(detector.z_score_threshold, 3.0);
    }

    #[test]
    fn test_spike_detection() {
        let mut detector = AnomalyDetector::new();
        detector.set_min_data_points(2);
        
        let mut metric = create_test_metric(vec![100.0; 10]);
        metric.update(150.0); // 50% spike
        
        let result = detector.detect(&metric);
        
        assert!(result.is_some());
        let r = result.unwrap();
        assert_eq!(r.anomaly_type, AnomalyType::Spike);
        assert!(r.score > 0.0);
    }

    #[test]
    fn test_drop_detection() {
        let mut detector = AnomalyDetector::new();
        detector.set_min_data_points(2);
        
        let mut metric = create_test_metric(vec![100.0; 10]);
        metric.update(50.0); // 50% drop
        
        let result = detector.detect(&metric);
        
        assert!(result.is_some());
        let r = result.unwrap();
        assert_eq!(r.anomaly_type, AnomalyType::Drop);
    }

    #[test]
    fn test_no_anomaly_normal_data() {
        let mut detector = AnomalyDetector::new();
        detector.set_min_data_points(5);
        
        // Normal fluctuating data
        let values: Vec<f64> = (0..20).map(|i| 100.0 + (i as f64 * 0.5)).collect();
        let metric = create_test_metric(values);
        
        let result = detector.detect(&metric);
        
        // Should not detect anomaly in steady trend
        assert!(result.is_none());
    }

    #[test]
    fn test_volatility_detection() {
        let mut detector = AnomalyDetector::new();
        
        // Low volatility period
        let mut values: Vec<f64> = (0..20).map(|_| 100.0).collect();
        // High volatility period
        values.extend((0..10).map(|i| if i % 2 == 0 { 150.0 } else { 50.0 }));
        
        let metric = create_test_metric(values);
        
        let result = detector.detect(&metric);
        
        // Should detect volatility increase
        assert!(result.is_some());
    }

    #[test]
    fn test_trend_detection() {
        let mut detector = AnomalyDetector::new();
        
        // Strong upward trend
        let values: Vec<f64> = (0..30).map(|i| 100.0 + (i as f64 * 10.0)).collect();
        let metric = create_test_metric(values);
        
        let result = detector.detect(&metric);
        
        assert!(result.is_some());
        let r = result.unwrap();
        assert_eq!(r.anomaly_type, AnomalyType::Trend);
    }

    #[test]
    fn test_zscore_detection() {
        let detector = AnomalyDetector::new();
        
        let values = vec![10.0, 11.0, 10.5, 11.5, 10.8, 11.2, 10.9, 11.1, 10.7, 11.3];
        
        // Normal value
        let normal = detector.detect_zscore(&values, 11.0);
        assert!(normal.is_none());
        
        // Outlier
        let outlier = detector.detect_zscore(&values, 50.0);
        assert!(outlier.is_some());
        assert!(outlier.unwrap() > 3.0);
    }

    #[test]
    fn test_threshold_configuration() {
        let mut detector = AnomalyDetector::new();
        
        let threshold = AnomalyThreshold {
            spike_threshold: 50.0,
            drop_threshold: -30.0,
            volatility_threshold: 2.0,
        };
        
        detector.set_threshold("test", threshold);
        
        assert!(detector.thresholds.contains_key("test"));
    }

    #[test]
    fn test_insufficient_data() {
        let mut detector = AnomalyDetector::new();
        detector.set_min_data_points(20);
        
        let metric = create_test_metric(vec![100.0; 5]);
        
        let result = detector.detect(&metric);
        assert!(result.is_none());
    }

    #[test]
    fn test_detection_result_fields() {
        let result = DetectionResult {
            metric_key: "test_metric".to_string(),
            anomaly_type: AnomalyType::Spike,
            score: 0.85,
            value: 150.0,
            expected_value: 100.0,
            timestamp: Utc::now(),
            description: "Test anomaly".to_string(),
        };
        
        assert_eq!(result.metric_key, "test_metric");
        assert_eq!(result.anomaly_type, AnomalyType::Spike);
        assert!(result.score > 0.0 && result.score <= 1.0);
    }
}
