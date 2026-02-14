//! gRPC Protocol Definitions
//! Sprint 49: Distributed Inference
//!
//! Protocol buffer message types for HRM service communication.
//! In production, these would be generated from .proto files using tonic-build.

/// Inference request
#[derive(Debug, Clone)]
pub struct InferenceRequest {
    pub signals: Vec<f32>,
    pub request_id: String,
}

/// Inference response
#[derive(Debug, Clone)]
pub struct InferenceResponse {
    pub conviction: f32,
    pub confidence: f32,
    pub regime: String,
    pub latency_micros: u64,
    pub node_id: String,
}

/// Health check request
#[derive(Debug, Clone, Default)]
pub struct HealthRequest {}

/// Health check response
#[derive(Debug, Clone)]
pub struct HealthResponse {
    pub healthy: bool,
    pub node_id: String,
    pub backend: String,
    pub active_requests: u64,
    pub total_requests: u64,
    pub avg_latency_micros: u64,
}

/// Node registration request
#[derive(Debug, Clone)]
pub struct RegisterRequest {
    pub node_id: String,
    pub addr: String,
    pub backend: String,
}

/// Node registration response
#[derive(Debug, Clone)]
pub struct RegisterResponse {
    pub success: bool,
    pub message: String,
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
}
