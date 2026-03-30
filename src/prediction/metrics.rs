//! Prometheus metrics for the ML prediction pipeline (Sprint 135).

use lazy_static::lazy_static;
use prometheus::{Histogram, HistogramOpts, IntCounter, IntCounterVec, Opts};

lazy_static! {
    /// Total prediction requests by model and status.
    pub static ref ML_PREDICTION_COUNT: IntCounterVec = IntCounterVec::new(
        Opts::new("ml_prediction_total", "Total ML prediction requests"),
        &["model", "status"],
    )
    .unwrap();

    /// Prediction latency histogram in milliseconds.
    pub static ref ML_PREDICTION_LATENCY: Histogram = Histogram::with_opts(
        HistogramOpts::new("ml_prediction_latency_ms", "ML prediction latency in milliseconds")
            .buckets(vec![5.0, 10.0, 25.0, 50.0, 100.0, 250.0, 500.0, 1000.0, 2500.0, 5000.0]),
    )
    .unwrap();

    /// Fallback prediction counter.
    pub static ref ML_FALLBACK_COUNT: IntCounter = IntCounter::new(
        "ml_prediction_fallback_total",
        "Total predictions using fallback (sidecar unavailable)",
    )
    .unwrap();

    /// Sidecar health status (1 = healthy, 0 = down).
    pub static ref ML_SIDECAR_HEALTH: IntCounter = IntCounter::new(
        "ml_sidecar_health_check_total",
        "Total sidecar health check attempts",
    )
    .unwrap();
}

/// Register all ML metrics with the default Prometheus registry.
pub fn register_ml_metrics() {
    let registry = prometheus::default_registry();
    let _ = registry.register(Box::new(ML_PREDICTION_COUNT.clone()));
    let _ = registry.register(Box::new(ML_PREDICTION_LATENCY.clone()));
    let _ = registry.register(Box::new(ML_FALLBACK_COUNT.clone()));
    let _ = registry.register(Box::new(ML_SIDECAR_HEALTH.clone()));
}

/// Record a prediction event.
pub fn record_prediction(model: &str, latency_ms: f64, is_fallback: bool) {
    let status = if is_fallback { "fallback" } else { "success" };
    ML_PREDICTION_COUNT
        .with_label_values(&[model, status])
        .inc();
    ML_PREDICTION_LATENCY.observe(latency_ms);
    if is_fallback {
        ML_FALLBACK_COUNT.inc();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_prediction_increments_counters() {
        record_prediction("catboost", 45.0, false);
        record_prediction("catboost", 0.0, true);

        // Verify counters are accessible (exact values depend on test ordering)
        let count = ML_PREDICTION_COUNT
            .with_label_values(&["catboost", "success"])
            .get();
        assert!(count >= 1);
    }
}
