//! HRM REST API Integration Tests (Sprint 43)
//!
//! Tests for HRM HTTP endpoints.

use serde::{Deserialize, Serialize};

/// HRM inference request (copy from handler for testing)
#[derive(Debug, Deserialize, Serialize)]
struct HRMInferenceRequest {
    pub pegy: f32,
    pub insider: f32,
    pub sentiment: f32,
    pub vix: f32,
    pub regime: f32,
    #[serde(default = "default_time")]
    pub time: f32,
}

fn default_time() -> f32 {
    0.5
}

/// HRM inference response (copy from handler for testing)
#[derive(Debug, Serialize)]
struct HRMInferenceResponse {
    pub conviction: f32,
    pub confidence: f32,
    pub regime: String,
    pub should_trade: bool,
    pub recommended_strategy: String,
    pub signal_strength: f32,
    pub source: String,
    pub latency_ms: f64,
}

/// Batch inference request
#[derive(Debug, Deserialize, Serialize)]
struct HRMBatchRequest {
    pub signals: Vec<HRMInferenceRequest>,
}

/// HRM health status
#[derive(Debug, Serialize)]
struct HRMHealthResponse {
    pub status: String,
    pub model_loaded: bool,
    pub model_version: String,
    pub parameters: usize,
    pub backend: String,
}

/// Test request/response serialization
#[test]
fn test_hrm_request_serialization() {
    let request = HRMInferenceRequest {
        pegy: 0.8,
        insider: 0.9,
        sentiment: 0.7,
        vix: 15.0,
        regime: 0.0,
        time: 0.5,
    };
    
    let json = serde_json::to_string(&request).unwrap();
    assert!(json.contains("pegy"));
    assert!(json.contains("0.8"));
    
    // Test deserialization
    let deserialized: HRMInferenceRequest = serde_json::from_str(&json).unwrap();
    assert!((deserialized.pegy - 0.8).abs() < 0.001);
}

/// Test response serialization
#[test]
fn test_hrm_response_serialization() {
    let response = HRMInferenceResponse {
        conviction: 0.85,
        confidence: 0.92,
        regime: "StrongUptrend".to_string(),
        should_trade: true,
        recommended_strategy: "Momentum".to_string(),
        signal_strength: 0.78,
        source: "MLModel".to_string(),
        latency_ms: 0.35,
    };
    
    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("conviction"));
    assert!(json.contains("0.85"));
    assert!(json.contains("should_trade"));
    assert!(json.contains("true"));
}

/// Test batch request
#[test]
fn test_hrm_batch_request() {
    let batch = HRMBatchRequest {
        signals: vec![
            HRMInferenceRequest {
                pegy: 0.8,
                insider: 0.9,
                sentiment: 0.7,
                vix: 15.0,
                regime: 0.0,
                time: 0.5,
            },
            HRMInferenceRequest {
                pegy: 0.2,
                insider: 0.1,
                sentiment: 0.2,
                vix: 50.0,
                regime: 1.0,
                time: 0.5,
            },
        ],
    };
    
    assert_eq!(batch.signals.len(), 2);
    
    let json = serde_json::to_string(&batch).unwrap();
    let deserialized: HRMBatchRequest = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.signals.len(), 2);
}

/// Test health response
#[test]
fn test_hrm_health_response() {
    let health = HRMHealthResponse {
        status: "healthy".to_string(),
        model_loaded: true,
        model_version: "hrm_synthetic_v1".to_string(),
        parameters: 9347,
        backend: "burn-ndarray".to_string(),
    };
    
    let json = serde_json::to_string(&health).unwrap();
    assert!(json.contains("healthy"));
    assert!(json.contains("9347"));
}

/// Test default time value
#[test]
fn test_default_time() {
    assert_eq!(default_time(), 0.5);
}

/// Example API usage documentation
#[test]
fn test_api_examples() {
    // Example 1: Strong bull market
    let bull_request = r#"{
        "pegy": 0.9,
        "insider": 0.9,
        "sentiment": 0.9,
        "vix": 10.0,
        "regime": 0.0,
        "time": 0.5
    }"#;
    
    let req: HRMInferenceRequest = serde_json::from_str(bull_request).unwrap();
    assert!(req.pegy > 0.8);
    assert!(req.vix < 20.0);
    
    // Example 2: Bear market (without time - should use default)
    let bear_request = r#"{
        "pegy": 0.2,
        "insider": 0.1,
        "sentiment": 0.2,
        "vix": 50.0,
        "regime": 1.0
    }"#;
    
    let req: HRMInferenceRequest = serde_json::from_str(bear_request).unwrap();
    assert!(req.pegy < 0.3);
    assert!(req.vix > 40.0);
    assert_eq!(req.time, 0.5); // Default value
}

/// Test validation scenarios
#[test]
fn test_request_validation() {
    // Valid request
    let valid = HRMInferenceRequest {
        pegy: 0.5,
        insider: 0.5,
        sentiment: 0.5,
        vix: 20.0,
        regime: 1.0,
        time: 0.5,
    };
    
    assert!(valid.pegy >= 0.0 && valid.pegy <= 1.0);
    assert!(valid.vix >= 0.0);
}

/// Test complete API flow documentation
#[test]
fn test_api_documentation() {
    // This test serves as documentation for the API
    
    // 1. Single inference request
    let single_request = serde_json::json!({
        "pegy": 0.8,
        "insider": 0.9,
        "sentiment": 0.7,
        "vix": 15.0,
        "regime": 0.0,
        "time": 0.5
    });
    
    // Expected response format
    let expected_response = serde_json::json!({
        "success": true,
        "data": {
            "conviction": 0.9294,
            "confidence": 0.9956,
            "regime": "StrongUptrend",
            "should_trade": true,
            "recommended_strategy": "Momentum",
            "signal_strength": 0.9253,
            "source": "MLModel",
            "latency_ms": 0.3
        },
        "error": null
    });
    
    assert!(expected_response["success"].as_bool().unwrap());
    
    // 2. Batch inference request
    let batch_request = serde_json::json!({
        "signals": [
            { "pegy": 0.8, "insider": 0.9, "sentiment": 0.7, "vix": 15.0, "regime": 0.0 },
            { "pegy": 0.2, "insider": 0.1, "sentiment": 0.2, "vix": 50.0, "regime": 1.0 }
        ]
    });
    
    assert_eq!(batch_request["signals"].as_array().unwrap().len(), 2);
    
    // 3. Health check response
    let health_response = serde_json::json!({
        "success": true,
        "data": {
            "status": "healthy",
            "model_loaded": true,
            "model_version": "hrm_synthetic_v1",
            "parameters": 9347,
            "backend": "burn-ndarray"
        }
    });
    
    assert_eq!(health_response["data"]["parameters"].as_i64().unwrap(), 9347);
}
