# Sprint 46: Performance Monitoring & Observability

## Overview
Implement comprehensive monitoring and observability for Investor OS using Prometheus metrics and Grafana dashboards. Track system health, HRM performance, and trading metrics in real-time.

## Goals
- Prometheus metrics endpoint (/metrics)
- HRM-specific metrics (inference latency, count, errors)
- System metrics (memory, CPU, goroutines)
- API metrics (request count, latency, status codes)
- WebSocket metrics (connections, messages)
- Grafana dashboard configuration

## Implementation

### 1. Dependencies
```toml
[dependencies]
prometheus = "0.13"
lazy_static = "1.4"
```

### 2. Metrics Registry
```rust
// src/monitoring/metrics.rs
use prometheus::{Registry, Counter, Histogram, Gauge, IntGauge};
use lazy_static::lazy_static;

lazy_static! {
    pub static ref REGISTRY: Registry = Registry::new();
    
    // HRM Metrics
    pub static ref HRM_INFERENCE_COUNT: Counter = 
        Counter::new("hrm_inference_total", "Total HRM inferences").unwrap();
    
    pub static ref HRM_INFERENCE_LATENCY: Histogram = 
        Histogram::with_opts(
            HistogramOpts::new("hrm_inference_duration_seconds", "HRM inference latency")
                .buckets(vec![0.0001, 0.0005, 0.001, 0.002, 0.005, 0.01, 0.025])
        ).unwrap();
    
    pub static ref HRM_INFERENCE_ERRORS: Counter = 
        Counter::new("hrm_inference_errors_total", "Total HRM inference errors").unwrap();
    
    // API Metrics
    pub static ref API_REQUESTS: CounterVec = 
        CounterVec::new(
            Opts::new("api_requests_total", "Total API requests"),
            &["method", "endpoint", "status"]
        ).unwrap();
    
    pub static ref API_LATENCY: HistogramVec = 
        HistogramVec::new(
            HistogramOpts::new("api_request_duration_seconds", "API request latency"),
            &["method", "endpoint"]
        ).unwrap();
    
    // WebSocket Metrics
    pub static ref WS_CONNECTIONS: IntGauge = 
        IntGauge::new("websocket_connections_active", "Active WebSocket connections").unwrap();
    
    pub static ref WS_MESSAGES: CounterVec = 
        CounterVec::new(
            Opts::new("websocket_messages_total", "Total WebSocket messages"),
            &["direction", "type"]
        ).unwrap();
}
```

### 3. Metrics Endpoint
```rust
// src/api/handlers/metrics.rs
use axum::response::{IntoResponse, Response};
use prometheus::TextEncoder;

pub async fn metrics_handler() -> Response {
    let encoder = TextEncoder::new();
    let metric_families = REGISTRY.gather();
    let mut buffer = vec![];
    encoder.encode(&metric_families, &mut buffer).unwrap();
    
    Response::builder()
        .header("Content-Type", encoder.format_type())
        .body(buffer.into())
        .unwrap()
}
```

### 4. HRM Instrumentation
```rust
// In HRM inference code
pub fn infer_with_metrics(&self, signals: &[f32]) -> Result<InferenceResult> {
    let timer = HRM_INFERENCE_LATENCY.start_timer();
    
    let result = self.infer(signals);
    
    match &result {
        Ok(_) => {
            HRM_INFERENCE_COUNT.inc();
        }
        Err(_) => {
            HRM_INFERENCE_ERRORS.inc();
        }
    }
    
    timer.observe_duration();
    result
}
```

### 5. Grafana Dashboard
```json
{
  "dashboard": {
    "title": "Investor OS - HRM Performance",
    "panels": [
      {
        "title": "HRM Inference Rate",
        "targets": [{
          "expr": "rate(hrm_inference_total[5m])"
        }]
      },
      {
        "title": "Inference Latency (p99)",
        "targets": [{
          "expr": "histogram_quantile(0.99, hrm_inference_duration_seconds_bucket)"
        }]
      },
      {
        "title": "Active WebSocket Connections",
        "targets": [{
          "expr": "websocket_connections_active"
        }]
      }
    ]
  }
}
```

## Metrics List

### HRM Metrics
| Metric | Type | Description |
|--------|------|-------------|
| hrm_inference_total | Counter | Total inferences |
| hrm_inference_duration_seconds | Histogram | Inference latency |
| hrm_inference_errors_total | Counter | Failed inferences |
| hrm_model_loaded | Gauge | 1 if model loaded |

### API Metrics
| Metric | Type | Description |
|--------|------|-------------|
| api_requests_total | Counter | HTTP requests by method/endpoint/status |
| api_request_duration_seconds | Histogram | Request latency |
| api_active_requests | Gauge | In-flight requests |

### WebSocket Metrics
| Metric | Type | Description |
|--------|------|-------------|
| websocket_connections_active | Gauge | Current connections |
| websocket_connections_total | Counter | Total connections |
| websocket_messages_total | Counter | Messages by direction/type |
| websocket_errors_total | Counter | WebSocket errors |

### System Metrics
| Metric | Type | Description |
|--------|------|-------------|
| process_resident_memory_bytes | Gauge | Memory usage |
| process_cpu_seconds_total | Counter | CPU time |
| tokio_runtime_threads | Gauge | Runtime threads |

## Test Coverage
- Metrics endpoint returns valid Prometheus format
- HRM metrics increment correctly
- Latency histograms populated
- Error counters work

## Acceptance Criteria
- [ ] /metrics endpoint serving Prometheus format
- [ ] HRM inference metrics working
- [ ] API request metrics working
- [ ] WebSocket metrics working
- [ ] Grafana dashboard JSON exported
- [ ] Docker compose with Prometheus + Grafana
- [ ] Alerts configured (optional)

## Status: 🔄 IN PROGRESS

---
**Prev**: Sprint 45 - WebSocket Streaming  
**Next**: Sprint 47 - Latency Optimization
