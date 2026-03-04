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

pub use discovery::{NodeId, NodeInfo, ServiceDiscovery, StaticDiscovery};
pub use fault_tolerance::{BreakerState, CircuitBreaker, NodeCircuitBreakers, RetryPolicy};
pub use load_balancer::{LoadBalancer, LoadBalancingStrategy, NodeHandle};
pub use node::{HRMNode, NodeConfig, NodeMetrics};

use thiserror::Error;
use tokio::sync::RwLock;

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
    load_balancer: RwLock<LoadBalancer>,
    discovery: Box<dyn ServiceDiscovery>,
    breakers: NodeCircuitBreakers,
}

impl DistributedHRM {
    /// Create new distributed HRM client
    pub async fn new(discovery: Box<dyn ServiceDiscovery>) -> Result<Self> {
        let nodes = discovery
            .discover()
            .await
            .map_err(|e| DistributedError::DiscoveryError(e.to_string()))?;

        let load_balancer = LoadBalancer::new(nodes);

        Ok(Self {
            load_balancer: RwLock::new(load_balancer),
            discovery,
            breakers: NodeCircuitBreakers::default(),
        })
    }

    /// Run inference on available node
    pub async fn infer(&self, signals: &[f32]) -> Result<proto::InferenceResponse> {
        let latest_nodes = self
            .discovery
            .discover()
            .await
            .map_err(|e| DistributedError::DiscoveryError(e.to_string()))?;

        {
            let mut lb = self.load_balancer.write().await;
            lb.update_nodes(latest_nodes);
        }

        let request = proto::InferenceRequest {
            signals: signals.to_vec(),
            request_id: uuid::Uuid::new_v4().to_string(),
        };
        let payload = request.to_wire().map_err(DistributedError::GrpcError)?;

        let node_count = {
            let lb = self.load_balancer.read().await;
            lb.node_count()
        };
        if node_count == 0 {
            return Err(DistributedError::NoAvailableNodes);
        }

        let mut last_error: Option<DistributedError> = None;
        let mut skipped_node: Option<String> = None;

        for _ in 0..node_count {
            let node = {
                let lb = self.load_balancer.read().await;
                lb.select_node().cloned()
            };
            let Some(node) = node else {
                continue;
            };

            let breaker = self.breakers.get(&node.id.0);
            if !breaker.can_execute() {
                skipped_node = Some(node.id.0.clone());
                continue;
            }

            match node::grpc_infer_transport(&node.addr, &node.id, payload.clone()).await {
                Ok(response_payload) => {
                    let response = proto::InferenceResponse::from_wire(&response_payload)
                        .map_err(DistributedError::GrpcError)?;
                    breaker.record_success();
                    return Ok(response);
                }
                Err(err) => {
                    breaker.record_failure();
                    last_error = Some(err);
                }
            }
        }

        if let Some(err) = last_error {
            return Err(err);
        }
        if let Some(node_id) = skipped_node {
            return Err(DistributedError::CircuitBreakerOpen(node_id));
        }

        Err(DistributedError::NoAvailableNodes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::distributed::StaticDiscovery;
    use std::collections::HashSet;

    #[test]
    fn test_error_display() {
        let err = DistributedError::NoAvailableNodes;
        assert_eq!(err.to_string(), "No available nodes");
    }

    #[tokio::test]
    async fn test_infer_with_static_discovery() {
        let discovery = Box::new(StaticDiscovery::from_addresses(vec![
            "127.0.0.1:50051".to_string()
        ]));
        let client = DistributedHRM::new(discovery)
            .await
            .expect("client should initialize");

        let response = client
            .infer(&[0.8, 0.1, -0.2, 0.4])
            .await
            .expect("inference should succeed");

        assert_eq!(response.node_id, "static-node-0");
        assert!(response.confidence >= 0.55);
    }

    #[tokio::test]
    async fn test_infer_with_no_nodes() {
        let discovery = Box::new(StaticDiscovery::new());
        let client = DistributedHRM::new(discovery)
            .await
            .expect("empty discovery should still initialize");

        let err = client
            .infer(&[0.1, 0.2])
            .await
            .expect_err("inference should fail with no nodes");

        match err {
            DistributedError::NoAvailableNodes => {}
            _ => panic!("expected NoAvailableNodes"),
        }
    }

    #[tokio::test]
    async fn test_infer_multi_node_routing() {
        let discovery = Box::new(StaticDiscovery::from_addresses(vec![
            "127.0.0.1:50051".to_string(),
            "127.0.0.1:50052".to_string(),
            "127.0.0.1:50053".to_string(),
        ]));
        let client = DistributedHRM::new(discovery)
            .await
            .expect("client should initialize");

        let mut visited_nodes: HashSet<String> = HashSet::new();
        for _ in 0..6 {
            let response = client
                .infer(&[0.3, -0.2, 0.6, 0.1])
                .await
                .expect("inference should succeed");
            visited_nodes.insert(response.node_id);
        }

        assert!(
            visited_nodes.len() >= 2,
            "routing should distribute calls across multiple nodes"
        );
    }

    #[tokio::test]
    async fn test_infer_failover_to_next_node() {
        let discovery = Box::new(StaticDiscovery::from_addresses(vec![
            "invalid_addr".to_string(),
            "127.0.0.1:50052".to_string(),
        ]));
        let client = DistributedHRM::new(discovery)
            .await
            .expect("client should initialize");

        let response = client
            .infer(&[0.5, 0.2, -0.1])
            .await
            .expect("failover should recover on second node");

        assert_eq!(response.node_id, "static-node-1");
    }

    #[tokio::test]
    async fn test_infer_all_nodes_unreachable() {
        let discovery = Box::new(StaticDiscovery::from_addresses(vec![
            "invalid_addr_1".to_string(),
            "invalid_addr_2".to_string(),
        ]));
        let client = DistributedHRM::new(discovery)
            .await
            .expect("client should initialize");

        let err = client
            .infer(&[0.1, 0.2, 0.3])
            .await
            .expect_err("all invalid addresses should fail");

        match err {
            DistributedError::NodeUnreachable(_) => {}
            _ => panic!("expected NodeUnreachable"),
        }
    }
}
