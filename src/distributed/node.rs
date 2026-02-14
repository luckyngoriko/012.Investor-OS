//! HRM Node - gRPC Server for Distributed Inference
//! Sprint 49: Distributed Inference

use super::Result;
use crate::hrm::gpu::{CpuBackend, GpuBackend};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
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
        self.latency_micros_sum.fetch_add(latency_micros, Ordering::Relaxed);
        
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
    
    /// Start gRPC server
    pub async fn serve(&self) -> Result<()> {
        info!("Starting HRM node {} on {}", 
            self.config.id.0, 
            self.config.bind_addr
        );
        
        // TODO: Implement actual gRPC server with tonic
        // For now, just keep running
        info!("HRM node {} ready", self.config.id.0);
        
        // Simulate server running
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
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
}
