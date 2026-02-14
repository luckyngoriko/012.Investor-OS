//! Distributed HRM Inference Module
//! Sprint 49: Distributed Inference with Load Balancing
//!
//! Provides horizontal scaling for HRM inference across multiple nodes.
//!
//! # Components
//! - `HRMNode`: gRPC server for inference on individual nodes
//! - `LoadBalancer`: Distributes requests across nodes
//! - `ServiceDiscovery`: Dynamic node registration and discovery
//! - `FaultTolerance`: Retry logic and circuit breakers
//!
//! # Architecture
//! ```
//! Client → Load Balancer → HRM Node (1..N)
//!              ↓
//!         Service Discovery
//! ```

pub mod discovery;
pub mod fault_tolerance;
pub mod load_balancer;
pub mod node;
pub mod proto;

pub use discovery::{ServiceDiscovery, StaticDiscovery, NodeInfo, NodeId};
pub use fault_tolerance::{CircuitBreaker, RetryPolicy, BreakerState};
pub use load_balancer::{LoadBalancer, LoadBalancingStrategy, NodeHandle};
pub use node::{HRMNode, NodeConfig, NodeMetrics};

use thiserror::Error;

/// Distributed inference errors
#[derive(Error, Debug)]
pub enum DistributedError {
    #[error("No available nodes")]
    NoAvailableNodes,
    
    #[error("Node {0} unreachable")]
    NodeUnreachable(String),
    
    #[error("Circuit breaker open for node {0}")]
    CircuitBreakerOpen(String),
    
    #[error("Service discovery failed: {0}")]
    DiscoveryError(String),
    
    #[error("All retry attempts exhausted")]
    RetryExhausted,
    
    #[error("gRPC error: {0}")]
    GrpcError(String),
}

/// Result type for distributed operations
pub type Result<T> = std::result::Result<T, DistributedError>;

/// Distributed HRM client
pub struct DistributedHRM {
    load_balancer: LoadBalancer,
    discovery: Box<dyn ServiceDiscovery>,
}

impl DistributedHRM {
    /// Create new distributed HRM client
    pub async fn new(discovery: Box<dyn ServiceDiscovery>) -> Result<Self> {
        let nodes = discovery.discover().await
            .map_err(|e| DistributedError::DiscoveryError(e.to_string()))?;
        
        let load_balancer = LoadBalancer::new(nodes);
        
        Ok(Self {
            load_balancer,
            discovery,
        })
    }
    
    /// Run inference on available node
    pub async fn infer(&self, _signals: &[f32]) -> Result<proto::InferenceResponse> {
        let node = self.load_balancer.select_node()
            .ok_or(DistributedError::NoAvailableNodes)?;
        
        // TODO: Implement actual gRPC call
        // For now, return mock response
        Ok(proto::InferenceResponse {
            conviction: 0.5,
            confidence: 0.8,
            regime: "Neutral".to_string(),
            latency_micros: 300,
            node_id: node.id.0.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = DistributedError::NoAvailableNodes;
        assert_eq!(err.to_string(), "No available nodes");
    }
}
