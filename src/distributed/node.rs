//! HRM Node - gRPC Server for Distributed Inference
//! Sprint 49: Distributed Inference

use super::DistributedError;
use super::Result;
use crate::hrm::gpu::{CpuBackend, GpuBackend};
use axum::extract::State;
use axum::Json;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use tracing::info;

/// Unique node identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NodeId(pub String);

impl NodeId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

/// Node configuration
#[derive(Debug, Clone)]
pub struct NodeConfig {
    pub id: NodeId,
    pub bind_addr: SocketAddr,
    pub max_concurrent_requests: usize,
    pub request_timeout_ms: u64,
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            id: NodeId::new(format!("node-{}", uuid::Uuid::new_v4())),
            bind_addr: "0.0.0.0:50051".parse().unwrap(),
            max_concurrent_requests: 1000,
            request_timeout_ms: 5000,
        }
    }
}

/// Node metrics
#[derive(Debug, Default)]
pub struct NodeMetrics {
    pub requests_total: AtomicU64,
    pub requests_active: AtomicU64,
    pub requests_failed: AtomicU64,
    pub latency_micros_sum: AtomicU64,
}

impl NodeMetrics {
    pub fn record_request(&self, latency_micros: u64, success: bool) {
        self.requests_total.fetch_add(1, Ordering::Relaxed);
        self.latency_micros_sum
            .fetch_add(latency_micros, Ordering::Relaxed);

        if !success {
            self.requests_failed.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn increment_active(&self) {
        self.requests_active.fetch_add(1, Ordering::Relaxed);
    }

    pub fn decrement_active(&self) {
        self.requests_active.fetch_sub(1, Ordering::Relaxed);
    }

    pub fn avg_latency_micros(&self) -> f64 {
        let total = self.requests_total.load(Ordering::Relaxed);
        if total == 0 {
            return 0.0;
        }
        let sum = self.latency_micros_sum.load(Ordering::Relaxed);
        sum as f64 / total as f64
    }
}

/// HRM Inference Node
pub struct HRMNode {
    pub config: NodeConfig,
    pub backend: Arc<dyn GpuBackend>,
    pub metrics: Arc<NodeMetrics>,
    pub health: Arc<RwLock<NodeHealth>>,
}

/// Node health status
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NodeHealth {
    Healthy,
    Degraded,
    Unhealthy,
}

/// Execute a transport-level inference call for a remote node.
///
/// This simulates the request/response lifecycle used by a gRPC hop:
/// 1) decode wire payload
/// 2) execute inference logic
/// 3) encode response payload
pub async fn grpc_infer_transport(
    addr: &str,
    node_id: &NodeId,
    payload: Vec<u8>,
) -> Result<Vec<u8>> {
    if addr.trim().is_empty() || addr.parse::<SocketAddr>().is_err() {
        return Err(DistributedError::NodeUnreachable(node_id.0.clone()));
    }

    let request =
        super::proto::InferenceRequest::from_wire(&payload).map_err(DistributedError::GrpcError)?;

    if request.signals.is_empty() {
        return Err(DistributedError::GrpcError(
            "inference request contains no signals".to_string(),
        ));
    }

    let started = Instant::now();
    let capped_len = request.signals.len().min(64);
    let sum: f32 = request.signals.iter().take(capped_len).copied().sum();
    let avg = sum / capped_len as f32;
    let normalized = avg.clamp(-1.0, 1.0);
    let conviction = ((normalized + 1.0) / 2.0).clamp(0.0, 1.0);
    let confidence = (0.55 + (capped_len as f32 / 100.0)).clamp(0.55, 0.99);
    let regime = if conviction >= 0.66 {
        "Bull"
    } else if conviction <= 0.34 {
        "Bear"
    } else {
        "Neutral"
    };

    let response = super::proto::InferenceResponse {
        conviction,
        confidence,
        regime: regime.to_string(),
        latency_micros: started.elapsed().as_micros() as u64,
        node_id: node_id.0.clone(),
    };

    response.to_wire().map_err(DistributedError::GrpcError)
}

impl HRMNode {
    /// Create new HRM node
    pub fn new(config: NodeConfig) -> Self {
        info!("Creating HRM node: {} with CPU backend", config.id.0);

        Self {
            config,
            backend: Arc::new(CpuBackend::new()),
            metrics: Arc::new(NodeMetrics::default()),
            health: Arc::new(RwLock::new(NodeHealth::Healthy)),
        }
    }

    /// Start HTTP inference server on the configured bind address.
    ///
    /// Endpoints:
    /// - `POST /infer` — JSON `InferenceRequest` → `InferenceResponse`
    /// - `GET  /health` — JSON `HealthResponse`
    pub async fn serve(self: Arc<Self>) -> Result<()> {
        use axum::routing::{get, post};
        use axum::Router;

        let node = self.clone();
        let app = Router::new()
            .route("/infer", post(Self::handle_infer))
            .route("/health", get(Self::handle_health))
            .with_state(node);

        let listener = tokio::net::TcpListener::bind(self.config.bind_addr)
            .await
            .map_err(|e| DistributedError::GrpcError(format!("bind failed: {e}")))?;

        info!(
            "HRM node {} listening on {}",
            self.config.id.0, self.config.bind_addr
        );

        axum::serve(listener, app)
            .await
            .map_err(|e| DistributedError::GrpcError(format!("serve error: {e}")))
    }

    async fn handle_infer(
        State(node): State<Arc<Self>>,
        Json(request): Json<super::proto::InferenceRequest>,
    ) -> std::result::Result<Json<super::proto::InferenceResponse>, axum::http::StatusCode> {
        node.metrics.increment_active();
        let started = Instant::now();

        let payload = request.to_wire().map_err(|_| {
            node.metrics.decrement_active();
            axum::http::StatusCode::BAD_REQUEST
        })?;

        let response_bytes =
            grpc_infer_transport(&node.config.bind_addr.to_string(), &node.config.id, payload)
                .await
                .map_err(|_| {
                    node.metrics
                        .record_request(started.elapsed().as_micros() as u64, false);
                    node.metrics.decrement_active();
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR
                })?;

        let response =
            super::proto::InferenceResponse::from_wire(&response_bytes).map_err(|_| {
                node.metrics.decrement_active();
                axum::http::StatusCode::INTERNAL_SERVER_ERROR
            })?;

        node.metrics
            .record_request(started.elapsed().as_micros() as u64, true);
        node.metrics.decrement_active();
        Ok(Json(response))
    }

    async fn handle_health(State(node): State<Arc<Self>>) -> Json<super::proto::HealthResponse> {
        Json(node.health_check().await)
    }

    /// Get current health
    pub async fn health(&self) -> NodeHealth {
        *self.health.read().await
    }

    /// Update health status
    pub async fn set_health(&self, health: NodeHealth) {
        let mut h = self.health.write().await;
        *h = health;
    }

    /// Perform health check
    pub async fn health_check(&self) -> super::proto::HealthResponse {
        let health = self.health().await;

        super::proto::HealthResponse {
            healthy: health == NodeHealth::Healthy,
            node_id: self.config.id.0.clone(),
            backend: self.backend.name().to_string(),
            active_requests: self.metrics.requests_active.load(Ordering::Relaxed),
            total_requests: self.metrics.requests_total.load(Ordering::Relaxed),
            avg_latency_micros: self.metrics.avg_latency_micros() as u64,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::distributed::proto::{InferenceRequest, InferenceResponse};

    #[test]
    fn test_node_id() {
        let id = NodeId::new("test-node");
        assert_eq!(id.0, "test-node");
    }

    #[test]
    fn test_node_metrics() {
        let metrics = NodeMetrics::default();

        metrics.record_request(100, true);
        metrics.record_request(200, true);
        metrics.record_request(300, false);

        assert_eq!(metrics.requests_total.load(Ordering::Relaxed), 3);
        assert_eq!(metrics.requests_failed.load(Ordering::Relaxed), 1);
        assert_eq!(metrics.avg_latency_micros(), 200.0);
    }

    #[test]
    fn test_grpc_infer_transport_roundtrip() {
        let rt = tokio::runtime::Runtime::new().expect("runtime");
        let node_id = NodeId::new("node-test");
        let request = InferenceRequest {
            signals: vec![0.8, 0.4, -0.1, 0.2],
            request_id: "req-1".to_string(),
        };

        let payload = request.to_wire().expect("encode request");
        let response_payload = rt
            .block_on(grpc_infer_transport("127.0.0.1:50051", &node_id, payload))
            .expect("transport should succeed");
        let response = InferenceResponse::from_wire(&response_payload).expect("decode response");

        assert_eq!(response.node_id, "node-test");
        assert!(response.confidence >= 0.55);
    }

    #[test]
    fn test_grpc_infer_transport_rejects_invalid_addr() {
        let rt = tokio::runtime::Runtime::new().expect("runtime");
        let node_id = NodeId::new("node-invalid");
        let request = InferenceRequest {
            signals: vec![0.2, 0.3],
            request_id: "req-invalid".to_string(),
        };
        let payload = request.to_wire().expect("encode request");

        let err = rt
            .block_on(grpc_infer_transport("bad_addr", &node_id, payload))
            .expect_err("invalid address should fail");

        match err {
            DistributedError::NodeUnreachable(id) => assert_eq!(id, "node-invalid"),
            _ => panic!("expected node unreachable error"),
        }
    }
}
