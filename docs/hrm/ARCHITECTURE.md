# HRM Architecture

## Overview

The Hierarchical Reasoning Model (HRM) is a native Rust implementation of a dual-module recurrent neural network designed for adaptive trading signal processing.

## Sprint 36 Scope

### Implemented
- Module scaffolding and API design
- Configuration management
- Inference engine with placeholder logic
- Weight loading infrastructure (safetensors/JSON)
- Comprehensive Golden Path tests (30 tests)
- Builder pattern for ergonomic initialization

### Pending (Sprint 37)
- Burn framework tensor operations integration
- Actual LSTM implementation
- GPU acceleration (CUDA/Metal)
- Weight conversion from Python-trained models
- Performance benchmarking

## Module Structure

```
src/hrm/
├── mod.rs         # Public API exports
├── config.rs      # HRMConfig, DeviceConfig
├── model.rs       # HRM struct, HRMBuilder
├── inference.rs   # InferenceEngine, InferenceResult
└── weights.rs     # WeightLoader, ModelWeights
```

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                         HRM Model                               │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │              High-Level Module (Slow)                   │   │
│  │  • LSTM: input=6, hidden=128                            │   │
│  │  • Market regime detection                              │   │
│  │  • Strategic planning                                   │   │
│  │  • Time scale: 100-1000ms                               │   │
│  └─────────────────────────┬───────────────────────────────┘   │
│                            │                                    │
│              ┌─────────────┴─────────────┐                      │
│              │    Cross-Connection       │                      │
│              │    Linear(128 → 64)       │                      │
│              └─────────────┬─────────────┘                      │
│                            │                                    │
│  ┌─────────────────────────▼───────────────────────────────┐   │
│  │               Low-Level Module (Fast)                   │   │
│  │  • LSTM: input=64, hidden=64                            │   │
│  │  • Signal aggregation                                   │   │
│  │  • Tactical execution                                   │   │
│  │  • Time scale: 10-50ms                                  │   │
│  └─────────────────────────┬───────────────────────────────┘   │
│                            │                                    │
│  ┌─────────────────────────▼───────────────────────────────┐   │
│  │                  Output Layer                           │   │
│  │  • Concat([high_state; low_state])                      │   │
│  │  • Linear(192 → 3)                                      │   │
│  │  • Output: [conviction, confidence, regime]             │   │
│  └─────────────────────────────────────────────────────────┘   │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

## Key Design Decisions

### 1. No Python Runtime
- Pure Rust implementation for memory safety
- Single binary deployment
- No GIL limitations

### 2. Burn Framework
- Chosen for native Rust deep learning
- Supports CPU, CUDA, and Metal backends
- Type-safe tensor operations

### 3. Dual-Module Architecture
- Mimics human brain's hierarchical processing
- High-level: Slow, abstract (prefrontal cortex analog)
- Low-level: Fast, detailed (basal ganglia analog)

### 4. Fallback Strategy
- If HRM confidence < threshold, use static CQ
- Ensures system works even if ML fails
- Gradual migration path

## Performance Targets

| Metric | Target | Current (Placeholder) |
|--------|--------|----------------------|
| Latency p50 | < 2ms | ~0.5ms |
| Latency p99 | < 5ms | ~1ms |
| Memory | < 100MB | ~10MB |
| Throughput | 10k infer/sec | N/A |

## Integration Points

### Decision Engine
```rust
// Old (Static CQ)
let cq = calculate_static_cq(signals);

// New (Adaptive CQ with HRM)
let result = hrm.infer(&signals)?;
let cq = if result.is_confident(0.7) {
    result.conviction
} else {
    calculate_static_cq(signals) // Fallback
};
```

### Signal Flow
```
TradingSignals → HRM.infer() → InferenceResult → DecisionEngine
                     ↓
              [conviction, confidence, regime]
```

## Testing Strategy

### Unit Tests
- Each module has comprehensive tests
- Input validation
- Error handling
- Edge cases

### Golden Path Tests
30 tests covering:
1. Module initialization (3 tests)
2. Weight loading (3 tests)
3. Inference correctness (4 tests)
4. Adaptive behavior (3 tests)
5. Input validation (3 tests)
6. Batch processing (3 tests)
7. Model statistics (3 tests)
8. Configuration options (3 tests)
9. Error handling (3 tests)
10. Integration (2 tests)

Run with: `cargo test --test golden_path hrm_`

## Future Work

### Sprint 37
- Burn tensor operations
- LSTM cell implementation
- GPU kernel integration
- Weight conversion pipeline

### Sprint 38
- Training pipeline in Rust
- Online learning
- Model versioning

### Sprint 39
- Distributed inference
- Model quantization
- Edge deployment

## References

- Original HRM Paper: [Hierarchical Reasoning Model (arXiv:2506.21734)](https://arxiv.org/abs/2506.21734)
- Burn Framework: https://burn.dev
- SafeTensors: https://huggingface.co/docs/safetensors
