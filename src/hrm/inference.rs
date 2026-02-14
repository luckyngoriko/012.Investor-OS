//! HRM Inference Engine
//!
//! Provides fast, type-safe inference for real-time trading decisions.
//! Optimized for low-latency execution (< 5ms p99).

use super::{HRMError, MarketRegime, Result};
use std::time::Instant;

/// Result of HRM inference
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InferenceResult {
    /// Conviction Quotient score (0.0 - 1.0)
    pub conviction: f32,

    /// Model confidence in prediction (0.0 - 1.0)
    pub confidence: f32,

    /// Detected market regime
    pub regime: MarketRegime,

    /// Inference latency in microseconds
    pub latency_us: u64,
}

impl InferenceResult {
    /// Create a new inference result
    pub fn new(conviction: f32, confidence: f32, regime: MarketRegime, latency_us: u64) -> Self {
        Self {
            conviction: conviction.clamp(0.0, 1.0),
            confidence: confidence.clamp(0.0, 1.0),
            regime,
            latency_us,
        }
    }

    /// Check if this result meets the confidence threshold
    pub fn is_confident(&self, threshold: f32) -> bool {
        self.confidence >= threshold
    }

    /// Returns true if we should trade based on this signal
    pub fn should_trade(&self, min_conviction: f32) -> bool {
        self.conviction >= min_conviction
    }
}

/// Inference engine for HRM
///
/// Handles the runtime execution of HRM models with optimized
/// batching and device management.
#[derive(Debug)]
pub struct InferenceEngine {
    /// Whether to use GPU acceleration
    use_gpu: bool,

    /// Batch size for processing multiple signals
    batch_size: usize,

    /// Inference timeout in microseconds
    timeout_us: u64,
}

impl Default for InferenceEngine {
    fn default() -> Self {
        Self {
            use_gpu: true,
            batch_size: 1,
            timeout_us: 5000, // 5ms timeout
        }
    }
}

impl InferenceEngine {
    /// Create a new inference engine
    pub fn new() -> Self {
        Self::default()
    }

    /// Configure GPU usage
    pub fn with_gpu(mut self, enabled: bool) -> Self {
        self.use_gpu = enabled;
        self
    }

    /// Configure batch size
    pub fn with_batch_size(mut self, size: usize) -> Self {
        self.batch_size = size.max(1);
        self
    }

    /// Configure timeout
    pub fn with_timeout(mut self, timeout_us: u64) -> Self {
        self.timeout_us = timeout_us;
        self
    }

    /// Run inference on a single set of signals
    ///
    /// # Arguments
    /// * `signals` - Input features [PEGY, Insider, Sentiment, VIX, MarketPhase, TimeOfDay]
    ///
    /// # Returns
    /// * `InferenceResult` containing CQ score, confidence, and regime
    ///
    /// # Errors
    /// * `HRMError::InferenceError` if inference fails
    /// * `HRMError::InvalidInputShape` if input size is wrong
    pub fn infer(&self, signals: &[f32]) -> Result<InferenceResult> {
        let start = Instant::now();

        // Validate input
        if signals.len() != 6 {
            return Err(HRMError::InvalidInputShape {
                expected: 6,
                actual: signals.len(),
            });
        }

        // TODO: Replace with actual HRM model inference
        // This is a placeholder implementation for Sprint 36 scaffolding
        let result = self.placeholder_inference(signals);

        let elapsed = start.elapsed().as_micros() as u64;

        // Check timeout
        if elapsed > self.timeout_us {
            return Err(HRMError::InferenceError(format!(
                "Inference timeout: {}us > {}us",
                elapsed, self.timeout_us
            )));
        }

        Ok(InferenceResult {
            latency_us: elapsed,
            ..result
        })
    }

    /// Run batch inference on multiple signals
    ///
    /// More efficient than multiple single inferences due to
    /// vectorization and reduced overhead.
    pub fn infer_batch(&self, batch: &[Vec<f32>]) -> Result<Vec<InferenceResult>> {
        batch.iter().map(|signals| self.infer(signals)).collect()
    }

    /// Placeholder inference for Sprint 36 scaffolding
    ///
    /// Returns reasonable defaults based on input signals.
    /// Replace with actual model inference in Sprint 36 implementation.
    fn placeholder_inference(&self, signals: &[f32]) -> InferenceResult {
        let pegy = signals[0];
        let insider = signals[1];
        let sentiment = signals[2];
        let vix = signals[3];
        let regime_input = signals[4];
        let _time = signals[5];

        // Simple heuristic for Sprint 36 scaffolding
        let base_conviction = pegy * 0.3 + insider * 0.3 + sentiment * 0.4;

        // Adjust for volatility
        let volatility_factor = 1.0 - (vix / 100.0).min(1.0);
        let conviction = base_conviction * volatility_factor;

        // Confidence based on signal strength
        let avg_signal = (pegy + insider + sentiment) / 3.0;
        let confidence = 0.5 + avg_signal * 0.5;

        let regime = MarketRegime::from(regime_input);

        InferenceResult::new(conviction, confidence, regime, 0)
    }

    /// Get engine statistics
    pub fn stats(&self) -> EngineStats {
        EngineStats {
            use_gpu: self.use_gpu,
            batch_size: self.batch_size,
            timeout_us: self.timeout_us,
        }
    }
}

/// Engine statistics
#[derive(Debug, Clone)]
pub struct EngineStats {
    pub use_gpu: bool,
    pub batch_size: usize,
    pub timeout_us: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inference_result_creation() {
        let result = InferenceResult::new(0.8, 0.9, MarketRegime::Bull, 100);
        assert_eq!(result.conviction, 0.8);
        assert_eq!(result.confidence, 0.9);
        assert_eq!(result.regime, MarketRegime::Bull);
        assert_eq!(result.latency_us, 100);
    }

    #[test]
    fn test_conviction_clamping() {
        let result = InferenceResult::new(1.5, 0.5, MarketRegime::Bull, 0);
        assert_eq!(result.conviction, 1.0);

        let result = InferenceResult::new(-0.5, 0.5, MarketRegime::Bull, 0);
        assert_eq!(result.conviction, 0.0);
    }

    #[test]
    fn test_is_confident() {
        let result = InferenceResult::new(0.8, 0.8, MarketRegime::Bull, 0);
        assert!(result.is_confident(0.7));
        assert!(!result.is_confident(0.9));
    }

    #[test]
    fn test_should_trade() {
        let result = InferenceResult::new(0.8, 0.9, MarketRegime::Bull, 0);
        assert!(result.should_trade(0.7));
        assert!(!result.should_trade(0.9));
    }

    #[test]
    fn test_infer_valid_input() {
        let engine = InferenceEngine::new();
        let signals = vec![0.8, 0.9, 0.7, 15.0, 0.0, 0.5];

        let result = engine.infer(&signals).unwrap();
        assert!(result.conviction >= 0.0 && result.conviction <= 1.0);
        assert!(result.confidence >= 0.0 && result.confidence <= 1.0);
    }

    #[test]
    fn test_infer_invalid_input_size() {
        let engine = InferenceEngine::new();
        let signals = vec![0.8, 0.9]; // Too few

        let result = engine.infer(&signals);
        assert!(matches!(
            result,
            Err(HRMError::InvalidInputShape { expected: 6, actual: 2 })
        ));
    }

    #[test]
    fn test_infer_batch() {
        let engine = InferenceEngine::new().with_batch_size(2);
        let batch = vec![
            vec![0.8, 0.9, 0.7, 15.0, 0.0, 0.5],
            vec![0.5, 0.6, 0.4, 25.0, 1.0, 0.3],
        ];

        let results = engine.infer_batch(&batch).unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_engine_builder() {
        let engine = InferenceEngine::new()
            .with_gpu(false)
            .with_batch_size(8)
            .with_timeout(10000);

        let stats = engine.stats();
        assert!(!stats.use_gpu);
        assert_eq!(stats.batch_size, 8);
        assert_eq!(stats.timeout_us, 10000);
    }

    #[test]
    fn test_placeholder_inference_logic() {
        let engine = InferenceEngine::new();

        // High VIX (volatility) should reduce conviction
        let low_vix = vec![0.8, 0.8, 0.8, 10.0, 0.0, 0.5];
        let high_vix = vec![0.8, 0.8, 0.8, 50.0, 0.0, 0.5];

        let result_low = engine.infer(&low_vix).unwrap();
        let result_high = engine.infer(&high_vix).unwrap();

        assert!(result_low.conviction > result_high.conviction);
    }
}
