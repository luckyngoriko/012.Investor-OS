//! Load Balancer for Distributed HRM
//! Sprint 49: Distributed Inference

use super::discovery::NodeInfo;
use super::node::{NodeId, NodeMetrics};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;

/// Load balancing strategies
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LoadBalancingStrategy {
    /// Round-robin selection
    RoundRobin,
    /// Select node with least active requests
    LeastConnections,
    /// Select node with lowest latency
    LeastLatency,
    /// Weighted selection based on capacity
    Weighted,
}

/// Handle to a backend node
#[derive(Debug, Clone)]
pub struct NodeHandle {
    pub id: NodeId,
    pub addr: String,
    pub metrics: Arc<NodeMetrics>,
    pub weight: u32,
    pub last_used: Instant,
}

impl From<NodeInfo> for NodeHandle {
    fn from(info: NodeInfo) -> Self {
        Self {
            id: info.id,
            addr: info.addr,
            metrics: Arc::new(NodeMetrics::default()),
            weight: info.weight,
            last_used: Instant::now(),
        }
    }
}

/// Load balancer for distributing inference requests
pub struct LoadBalancer {
    nodes: Vec<NodeHandle>,
    strategy: LoadBalancingStrategy,
    round_robin_counter: AtomicUsize,
}

impl LoadBalancer {
    /// Create new load balancer
    pub fn new(nodes: Vec<NodeInfo>) -> Self {
        let handles: Vec<_> = nodes.into_iter().map(NodeHandle::from).collect();

        Self {
            nodes: handles,
            strategy: LoadBalancingStrategy::RoundRobin,
            round_robin_counter: AtomicUsize::new(0),
        }
    }

    /// Create with specific strategy
    pub fn with_strategy(mut self, strategy: LoadBalancingStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    /// Select a node based on strategy
    pub fn select_node(&self) -> Option<&NodeHandle> {
        if self.nodes.is_empty() {
            return None;
        }

        match self.strategy {
            LoadBalancingStrategy::RoundRobin => self.select_round_robin(),
            LoadBalancingStrategy::LeastConnections => self.select_least_connections(),
            LoadBalancingStrategy::LeastLatency => self.select_least_latency(),
            LoadBalancingStrategy::Weighted => self.select_weighted(),
        }
    }

    /// Round-robin selection
    fn select_round_robin(&self) -> Option<&NodeHandle> {
        let idx = self.round_robin_counter.fetch_add(1, Ordering::Relaxed) % self.nodes.len();
        self.nodes.get(idx)
    }

    /// Select node with least active connections
    fn select_least_connections(&self) -> Option<&NodeHandle> {
        self.nodes
            .iter()
            .min_by_key(|n| n.metrics.requests_active.load(Ordering::Relaxed))
    }

    /// Select node with lowest average latency
    fn select_least_latency(&self) -> Option<&NodeHandle> {
        self.nodes.iter().min_by(|a, b| {
            let a_latency = a.metrics.avg_latency_micros();
            let b_latency = b.metrics.avg_latency_micros();
            a_latency
                .partial_cmp(&b_latency)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    }

    /// Weighted random selection
    fn select_weighted(&self) -> Option<&NodeHandle> {
        let total_weight: u32 = self.nodes.iter().map(|n| n.weight).sum();
        if total_weight == 0 {
            return self.select_round_robin();
        }

        // Simple weighted selection based on counter
        let idx = self.round_robin_counter.fetch_add(1, Ordering::Relaxed) % total_weight as usize;

        let mut current = 0;
        for node in &self.nodes {
            current += node.weight as usize;
            if idx < current {
                return Some(node);
            }
        }

        self.nodes.first()
    }

    /// Update node list
    pub fn update_nodes(&mut self, nodes: Vec<NodeInfo>) {
        self.nodes = nodes.into_iter().map(NodeHandle::from).collect();
    }

    /// Add a node
    pub fn add_node(&mut self, node: NodeInfo) {
        self.nodes.push(node.into());
    }

    /// Remove a node
    pub fn remove_node(&mut self, id: &NodeId) {
        self.nodes.retain(|n| n.id != *id);
    }

    /// Get node count
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Get all nodes
    pub fn nodes(&self) -> &[NodeHandle] {
        &self.nodes
    }
}

/// Load balancer builder
pub struct LoadBalancerBuilder {
    strategy: LoadBalancingStrategy,
}

impl LoadBalancerBuilder {
    pub fn new() -> Self {
        Self {
            strategy: LoadBalancingStrategy::RoundRobin,
        }
    }

    pub fn strategy(mut self, strategy: LoadBalancingStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    pub fn build(self, nodes: Vec<NodeInfo>) -> LoadBalancer {
        LoadBalancer::new(nodes).with_strategy(self.strategy)
    }
}

impl Default for LoadBalancerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_nodes() -> Vec<NodeInfo> {
        vec![
            NodeInfo {
                id: NodeId::new("node-1"),
                addr: "127.0.0.1:50051".to_string(),
                weight: 1,
                last_heartbeat: std::time::Instant::now(),
            },
            NodeInfo {
                id: NodeId::new("node-2"),
                addr: "127.0.0.1:50052".to_string(),
                weight: 2,
                last_heartbeat: std::time::Instant::now(),
            },
            NodeInfo {
                id: NodeId::new("node-3"),
                addr: "127.0.0.1:50053".to_string(),
                weight: 1,
                last_heartbeat: std::time::Instant::now(),
            },
        ]
    }

    #[test]
    fn test_round_robin() {
        let nodes = create_test_nodes();
        let lb = LoadBalancer::new(nodes).with_strategy(LoadBalancingStrategy::RoundRobin);

        let first = lb.select_node().unwrap();
        let second = lb.select_node().unwrap();
        let third = lb.select_node().unwrap();
        let fourth = lb.select_node().unwrap();

        assert_eq!(first.id.0, "node-1");
        assert_eq!(second.id.0, "node-2");
        assert_eq!(third.id.0, "node-3");
        assert_eq!(fourth.id.0, "node-1"); // Wraps around
    }

    #[test]
    fn test_least_connections() {
        let nodes = create_test_nodes();
        let lb = LoadBalancer::new(nodes).with_strategy(LoadBalancingStrategy::LeastConnections);

        // Simulate node-1 having active requests
        lb.nodes[0].metrics.increment_active();
        lb.nodes[0].metrics.increment_active();

        let selected = lb.select_node().unwrap();

        // Should select node with least connections
        assert_ne!(selected.id.0, "node-1");
    }

    #[test]
    fn test_empty_nodes() {
        let lb = LoadBalancer::new(vec![]);
        assert!(lb.select_node().is_none());
    }

    #[test]
    fn test_node_count() {
        let nodes = create_test_nodes();
        let lb = LoadBalancer::new(nodes);
        assert_eq!(lb.node_count(), 3);
    }
}
