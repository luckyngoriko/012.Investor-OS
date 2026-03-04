//! gRPC Protocol Definitions
//! Sprint 49: Distributed Inference
//!
//! Protocol buffer message types for HRM service communication.
//! In production, these would be generated from .proto files using tonic-build.

use serde::{Deserialize, Serialize};

/// Inference request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceRequest {
    pub signals: Vec<f32>,
    pub request_id: String,
}

/// Inference response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceResponse {
    pub conviction: f32,
    pub confidence: f32,
    pub regime: String,
    pub latency_micros: u64,
    pub node_id: String,
}

/// Health check request
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HealthRequest {}

/// Health check response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub healthy: bool,
    pub node_id: String,
    pub backend: String,
    pub active_requests: u64,
    pub total_requests: u64,
    pub avg_latency_micros: u64,
}

/// Node registration request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterRequest {
    pub node_id: String,
    pub addr: String,
    pub backend: String,
}

/// Node registration response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterResponse {
    pub success: bool,
    pub message: String,
}

impl InferenceRequest {
    /// Encode request payload for transport.
    pub fn to_wire(&self) -> Result<Vec<u8>, String> {
        serde_json::to_vec(self).map_err(|err| format!("request encode error: {err}"))
    }

    /// Decode request payload from transport.
    pub fn from_wire(payload: &[u8]) -> Result<Self, String> {
        serde_json::from_slice(payload).map_err(|err| format!("request decode error: {err}"))
    }
}

impl InferenceResponse {
    /// Encode response payload for transport.
    pub fn to_wire(&self) -> Result<Vec<u8>, String> {
        serde_json::to_vec(self).map_err(|err| format!("response encode error: {err}"))
    }

    /// Decode response payload from transport.
    pub fn from_wire(payload: &[u8]) -> Result<Self, String> {
        serde_json::from_slice(payload).map_err(|err| format!("response decode error: {err}"))
    }
}

/// Service definition (would be generated from .proto)
pub trait HRMService {
    /// Perform inference
    fn infer(&self, request: InferenceRequest) -> InferenceResponse;

    /// Check health
    fn health_check(&self, request: HealthRequest) -> HealthResponse;

    /// Stream inference (for batch processing)
    fn stream_infer(
        &self,
        requests: impl Iterator<Item = InferenceRequest>,
    ) -> impl Iterator<Item = InferenceResponse>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inference_request() {
        let req = InferenceRequest {
            signals: vec![0.8, 0.9, 0.7, 15.0, 0.0, 0.5],
            request_id: "test-123".to_string(),
        };

        assert_eq!(req.signals.len(), 6);
        assert_eq!(req.request_id, "test-123");
    }

    #[test]
    fn test_inference_response() {
        let resp = InferenceResponse {
            conviction: 0.85,
            confidence: 0.92,
            regime: "Bull".to_string(),
            latency_micros: 300,
            node_id: "node-1".to_string(),
        };

        assert!(resp.conviction > 0.0);
        assert!(resp.confidence > 0.0);
    }

    #[test]
    fn test_request_wire_roundtrip() {
        let req = InferenceRequest {
            signals: vec![0.2, -0.1, 0.4],
            request_id: "wire-req-1".to_string(),
        };

        let payload = req.to_wire().expect("request should encode");
        let decoded = InferenceRequest::from_wire(&payload).expect("request should decode");
        assert_eq!(decoded.request_id, "wire-req-1");
        assert_eq!(decoded.signals.len(), 3);
    }

    #[test]
    fn test_response_wire_roundtrip() {
        let resp = InferenceResponse {
            conviction: 0.71,
            confidence: 0.88,
            regime: "Bull".to_string(),
            latency_micros: 1234,
            node_id: "node-a".to_string(),
        };

        let payload = resp.to_wire().expect("response should encode");
        let decoded = InferenceResponse::from_wire(&payload).expect("response should decode");
        assert_eq!(decoded.node_id, "node-a");
        assert_eq!(decoded.regime, "Bull");
        assert_eq!(decoded.latency_micros, 1234);
    }
}
