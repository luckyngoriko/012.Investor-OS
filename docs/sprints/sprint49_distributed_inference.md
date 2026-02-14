# Sprint 49: Distributed HRM Inference

## Overview
Scale HRM inference across multiple nodes with automatic load balancing, service discovery, and fault tolerance. Handle high-throughput scenarios with horizontal scaling.

## Goals
- Multi-node HRM cluster
- gRPC communication between nodes
- Load balancing (round-robin, least-latency)
- Service discovery (etcd or static config)
- Fault tolerance (retry, circuit breaker)
- Horizontal scaling support

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        Client Request                            │
└─────────────────────────────────┬───────────────────────────────┘
                                  │
                    ┌─────────────▼─────────────┐
                    │      Load Balancer        │
                    │  (Round-robin / Latency)  │
                    └─────────────┬─────────────┘
                                  │
            ┌─────────────────────┼─────────────────────┐
            │                     │                     │
    ┌───────▼──────┐    ┌────────▼───────┐   ┌────────▼───────┐
    │  HRM Node 1  │    │   HRM Node 2   │   │   HRM Node N   │
    │  (GPU/CPU)   │    │    (GPU/CPU)   │   │    (GPU/CPU)   │
    └──────────────┘    └────────────────┘   └────────────────┘
            │                     │                     │
            └─────────────────────┼─────────────────────┘
                                  │
                    ┌─────────────▼─────────────┐
                    │    Service Discovery      │
                    │        (etcd)             │
                    └───────────────────────────┘
```

## Components

### 1. HRM Node (gRPC Server)
```rust
// src/distributed/node.rs
pub struct HRMNode {
    id: NodeId,
    backend: Box<dyn GpuBackend>,
    metrics: NodeMetrics,
}

impl HRMNode {
    pub async fn serve(&self, addr: SocketAddr) -> Result<()> {
        // gRPC server for inference requests
    }
}
```

### 2. Load Balancer
```rust
// src/distributed/load_balancer.rs
pub enum LoadBalancingStrategy {
    RoundRobin,
    LeastLatency,
    Weighted,
}

pub struct LoadBalancer {
    nodes: Vec<NodeHandle>,
    strategy: LoadBalancingStrategy,
}
```

### 3. Service Discovery
```rust
// src/distributed/discovery.rs
pub trait ServiceDiscovery {
    async fn register(&self, node: NodeInfo) -> Result<()>;
    async fn discover(&self) -> Vec<NodeInfo>;
    async fn heartbeat(&self, node_id: NodeId) -> Result<()>;
}
```

### 4. Fault Tolerance
```rust
// src/distributed/fault_tolerance.rs
pub struct CircuitBreaker {
    failure_threshold: u32,
    reset_timeout: Duration,
    state: BreakerState,
}

pub struct RetryPolicy {
    max_attempts: u32,
    backoff: ExponentialBackoff,
}
```

## gRPC Protocol
```protobuf
// proto/hrm.proto
service HRMService {
  rpc Infer (InferenceRequest) returns (InferenceResponse);
  rpc StreamInfer (stream InferenceRequest) returns (stream InferenceResponse);
  rpc HealthCheck (HealthRequest) returns (HealthResponse);
}

message InferenceRequest {
  repeated float signals = 1;
  string request_id = 2;
}

message InferenceResponse {
  float conviction = 1;
  float confidence = 2;
  string regime = 3;
  int64 latency_micros = 4;
  string node_id = 5;
}
```

## Deployment

### Docker Compose (3-node cluster)
```yaml
version: '3.8'
services:
  hrm-node-1:
    image: investor-os:latest
    command: ["hrm-node", "--id=node-1", "--port=50051"]
    
  hrm-node-2:
    image: investor-os:latest
    command: ["hrm-node", "--id=node-2", "--port=50052"]
    
  hrm-node-3:
    image: investor-os:latest
    command: ["hrm-node", "--id=node-3", "--port=50053"]
    
  load-balancer:
    image: investor-os:latest
    command: ["hrm-lb", "--nodes=node-1:50051,node-2:50052,node-3:50053"]
    ports:
      - "8080:8080"
```

## Performance Targets

| Metric | Single Node | 3-Node Cluster | 10-Node Cluster |
|--------|-------------|----------------|-----------------|
| Throughput | 3,000/sec | 9,000/sec | 30,000/sec |
| Latency (p99) | 0.3ms | 0.5ms | 1.0ms |
| Availability | 99.9% | 99.99% | 99.999% |

## Status: 🔄 IN PROGRESS

---
**Prev**: Sprint 48 - GPU Acceleration  
**Next**: Sprint 50 - Auto-Scaling
