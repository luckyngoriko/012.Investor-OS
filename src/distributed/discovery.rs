//! Service Discovery for Distributed HRM
//! Sprint 49: Distributed Inference

use super::{Result, DistributedError};
pub use super::node::NodeId;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Node information for service discovery
#[derive(Debug, Clone)]
pub struct NodeInfo {
    pub id: NodeId,
    pub addr: String,
    pub weight: u32,
    pub last_heartbeat: Instant,
}

/// Service discovery trait
#[async_trait]
pub trait ServiceDiscovery: Send + Sync {
    /// Register a node
    async fn register(&self, node: NodeInfo) -> Result<()>;
    
    /// Deregister a node
    async fn deregister(&self, node_id: &NodeId) -> Result<()>;
    
    /// Discover all available nodes
    async fn discover(&self) -> Result<Vec<NodeInfo>>;
    
    /// Send heartbeat for a node
    async fn heartbeat(&self, node_id: &NodeId) -> Result<()>;
}

/// Static discovery (configuration-based)
pub struct StaticDiscovery {
    nodes: Arc<Mutex<HashMap<String, NodeInfo>>>,
}

impl StaticDiscovery {
    /// Create from list of addresses
    pub fn from_addresses(addrs: Vec<String>) -> Self {
        let mut nodes = HashMap::new();
        
        for (i, addr) in addrs.into_iter().enumerate() {
            let id = NodeId::new(format!("static-node-{}", i));
            let info = NodeInfo {
                id: id.clone(),
                addr,
                weight: 1,
                last_heartbeat: Instant::now(),
            };
            nodes.insert(id.0.clone(), info);
        }
        
        Self {
            nodes: Arc::new(Mutex::new(nodes)),
        }
    }
    
    /// Create empty
    pub fn new() -> Self {
        Self {
            nodes: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    
    /// Add node manually
    pub fn add_node(&self, info: NodeInfo) {
        let mut nodes = self.nodes.lock().unwrap();
        nodes.insert(info.id.0.clone(), info);
    }
}

impl Default for StaticDiscovery {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ServiceDiscovery for StaticDiscovery {
    async fn register(&self, node: NodeInfo) -> Result<()> {
        let mut nodes = self.nodes.lock().map_err(|_| {
            DistributedError::DiscoveryError("Lock poisoned".to_string())
        })?;
        nodes.insert(node.id.0.clone(), node);
        Ok(())
    }
    
    async fn deregister(&self, node_id: &NodeId) -> Result<()> {
        let mut nodes = self.nodes.lock().map_err(|_| {
            DistributedError::DiscoveryError("Lock poisoned".to_string())
        })?;
        nodes.remove(&node_id.0);
        Ok(())
    }
    
    async fn discover(&self) -> Result<Vec<NodeInfo>> {
        let nodes = self.nodes.lock().map_err(|_| {
            DistributedError::DiscoveryError("Lock poisoned".to_string())
        })?;
        Ok(nodes.values().cloned().collect())
    }
    
    async fn heartbeat(&self, node_id: &NodeId) -> Result<()> {
        let mut nodes = self.nodes.lock().map_err(|_| {
            DistributedError::DiscoveryError("Lock poisoned".to_string())
        })?;
        
        if let Some(node) = nodes.get_mut(&node_id.0) {
            node.last_heartbeat = Instant::now();
            Ok(())
        } else {
            Err(DistributedError::NodeUnreachable(node_id.0.clone()))
        }
    }
}

/// etcd-based service discovery (placeholder for production)
pub struct EtcdDiscovery {
    endpoints: Vec<String>,
    prefix: String,
}

impl EtcdDiscovery {
    pub fn new(endpoints: Vec<String>) -> Self {
        Self {
            endpoints,
            prefix: "/hrm/nodes".to_string(),
        }
    }
}

#[async_trait]
impl ServiceDiscovery for EtcdDiscovery {
    async fn register(&self, _node: NodeInfo) -> Result<()> {
        // TODO: Implement etcd client integration
        // For now, return error
        Err(DistributedError::DiscoveryError(
            "etcd not implemented".to_string()
        ))
    }
    
    async fn deregister(&self, _node_id: &NodeId) -> Result<()> {
        Err(DistributedError::DiscoveryError(
            "etcd not implemented".to_string()
        ))
    }
    
    async fn discover(&self) -> Result<Vec<NodeInfo>> {
        Err(DistributedError::DiscoveryError(
            "etcd not implemented".to_string()
        ))
    }
    
    async fn heartbeat(&self, _node_id: &NodeId) -> Result<()> {
        Err(DistributedError::DiscoveryError(
            "etcd not implemented".to_string()
        ))
    }
}

/// Health checker for nodes
pub struct HealthChecker {
    check_interval: Duration,
    timeout: Duration,
}

impl HealthChecker {
    pub fn new(check_interval: Duration, timeout: Duration) -> Self {
        Self {
            check_interval,
            timeout,
        }
    }
    
    /// Start health checking loop
    pub async fn start<F>(&self, mut check_fn: F)
    where
        F: FnMut() + Send + 'static,
    {
        loop {
            tokio::time::sleep(self.check_interval).await;
            check_fn();
        }
    }
}

impl Default for HealthChecker {
    fn default() -> Self {
        Self::new(
            Duration::from_secs(10),
            Duration::from_secs(5),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_static_discovery() {
        let discovery = StaticDiscovery::from_addresses(vec![
            "127.0.0.1:50051".to_string(),
            "127.0.0.1:50052".to_string(),
        ]);
        
        let rt = tokio::runtime::Runtime::new().unwrap();
        let nodes = rt.block_on(discovery.discover()).unwrap();
        
        assert_eq!(nodes.len(), 2);
    }

    #[test]
    fn test_static_register_deregister() {
        let discovery = StaticDiscovery::new();
        let rt = tokio::runtime::Runtime::new().unwrap();
        
        let node = NodeInfo {
            id: NodeId::new("test-node"),
            addr: "127.0.0.1:50051".to_string(),
            weight: 1,
            last_heartbeat: Instant::now(),
        };
        
        rt.block_on(discovery.register(node.clone())).unwrap();
        
        let nodes = rt.block_on(discovery.discover()).unwrap();
        assert_eq!(nodes.len(), 1);
        
        rt.block_on(discovery.deregister(&node.id)).unwrap();
        
        let nodes = rt.block_on(discovery.discover()).unwrap();
        assert_eq!(nodes.len(), 0);
    }
}
