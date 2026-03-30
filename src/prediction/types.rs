//! Shared types for ML prediction pipeline (Sprint 114).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Request sent to the ML sidecar.
#[derive(Debug, Clone, Serialize)]
pub struct PredictionRequest {
    pub model: String,
    pub symbol: String,
    pub features: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub horizon: Option<String>,
}

/// Response received from the ML sidecar.
#[derive(Debug, Clone, Deserialize)]
pub struct PredictionResponse {
    pub model: String,
    pub model_version: String,
    pub prediction: serde_json::Value,
    pub confidence: f64,
    pub latency_ms: u64,
    pub timestamp: DateTime<Utc>,
}

/// Health status reported by the ML sidecar.
#[derive(Debug, Clone, Deserialize)]
pub struct SidecarHealth {
    pub status: String,
    pub version: String,
    pub models_loaded: Vec<String>,
    pub gpu_available: bool,
    pub uptime_seconds: f64,
}

/// Model info from the sidecar's model registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub name: String,
    pub version: String,
    pub model_type: String,
    pub tier: i32,
    pub status: String,
}

/// Prediction result returned to the API caller, enriched with metadata.
#[derive(Debug, Clone, Serialize)]
pub struct PredictionResult {
    pub id: uuid::Uuid,
    pub model: String,
    pub model_version: String,
    pub symbol: String,
    pub prediction: serde_json::Value,
    pub confidence: f64,
    pub latency_ms: u64,
    pub fallback: bool,
    pub ai_decision_log_id: Option<uuid::Uuid>,
    pub timestamp: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prediction_request_serializes() {
        let req = PredictionRequest {
            model: "catboost".to_string(),
            symbol: "BTCUSDT".to_string(),
            features: serde_json::json!({"rsi_14": 65.2}),
            horizon: Some("1d".to_string()),
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("catboost"));
        assert!(json.contains("rsi_14"));
    }

    #[test]
    fn sidecar_health_deserializes() {
        let json = r#"{
            "status": "healthy",
            "version": "0.1.0",
            "models_loaded": [],
            "gpu_available": false,
            "uptime_seconds": 42.5
        }"#;
        let health: SidecarHealth = serde_json::from_str(json).unwrap();
        assert_eq!(health.status, "healthy");
        assert!(!health.gpu_available);
    }
}
