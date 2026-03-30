//! Fallback prediction when the ML sidecar is unavailable (Sprint 114).
//!
//! Delegates to the existing HRM deterministic policy to maintain
//! prediction capability even without the Python sidecar.

use chrono::Utc;
use uuid::Uuid;

use super::types::{PredictionRequest, PredictionResult};

/// Generate a fallback prediction using HRM heuristic policy.
///
/// This is intentionally conservative — returns Hold with low confidence
/// when the ML sidecar is not reachable. The HRM module can be wired in
/// for richer fallback once the integration is complete.
pub fn fallback_predict(request: &PredictionRequest) -> PredictionResult {
    // Conservative heuristic: Hold with low confidence
    let conviction = 0.5;
    let confidence = 0.2; // Low — we're guessing without ML

    let action = "hold";
    let prediction = serde_json::json!({
        "direction": action,
        "return_pct": 0.0,
        "note": "fallback — ML sidecar unavailable"
    });

    PredictionResult {
        id: Uuid::new_v4(),
        model: "hrm_fallback".to_string(),
        model_version: "0.0.1".to_string(),
        symbol: request.symbol.clone(),
        prediction,
        confidence,
        latency_ms: 0,
        fallback: true,
        ai_decision_log_id: None,
        timestamp: Utc::now(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fallback_returns_hold_with_low_confidence() {
        let req = PredictionRequest {
            model: "catboost".to_string(),
            symbol: "BTCUSDT".to_string(),
            features: serde_json::json!({}),
            horizon: Some("1d".to_string()),
        };

        let result = fallback_predict(&req);

        assert!(result.fallback);
        assert_eq!(result.model, "hrm_fallback");
        assert_eq!(result.symbol, "BTCUSDT");
        assert!(result.confidence < 0.5);
        assert_eq!(result.latency_ms, 0);
        assert!(result.prediction["direction"] == "hold");
    }
}
