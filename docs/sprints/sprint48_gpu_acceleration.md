# Sprint 48: Multi-Backend GPU Acceleration

## Overview
Implement GPU acceleration for HRM inference with automatic device detection and fallback. Support NVIDIA (CUDA), AMD (ROCm), and Intel (oneAPI) GPUs with CPU as default fallback.

## Goals
- Auto-detect available GPU devices
- Support CUDA (NVIDIA), ROCm (AMD), oneAPI (Intel)
- Seamless fallback: GPU → CPU
- Runtime backend switching
- Performance benchmarks per backend
- Feature flags for optional compilation

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    BackendSelector                          │
│                  (Auto-detect & Fallback)                   │
└──────────────────────┬──────────────────────────────────────┘
                       │
        ┌──────────────┼──────────────┬──────────────┐
        ▼              ▼              ▼              ▼
┌──────────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐
│   CUDA       │ │  ROCm    │ │  oneAPI  │ │ NdArray  │
│  (NVIDIA)    │ │  (AMD)   │ │  (Intel) │ │  (CPU)   │
└──────────────┘ └──────────┘ └──────────┘ └──────────┘
```

## Backend Priority
1. CUDA (NVIDIA) - fastest for ML
2. ROCm (AMD) - open source alternative
3. oneAPI (Intel) - integrated GPUs
4. NdArray (CPU) - guaranteed fallback

## Implementation

### GPU Detection
```rust
// src/hrm/gpu/detector.rs
pub struct GpuDetector;

impl GpuDetector {
    pub fn detect_available_backends() -> Vec<BackendType> {
        // Check CUDA
        // Check ROCm
        // Check oneAPI
        // Always include CPU
    }
    
    pub fn get_optimal_backend() -> BackendType {
        // Return fastest available
    }
}
```

### Backend Abstraction
```rust
// src/hrm/gpu/backend.rs
pub enum BackendType {
    Cuda,    // NVIDIA
    Rocm,    // AMD
    OneApi,  // Intel
    Cpu,     // NdArray fallback
}

pub trait GpuBackend {
    fn name(&self) -> &'static str;
    fn is_available(&self) -> bool;
    fn device_info(&self) -> DeviceInfo;
    fn create_tensor(&self, data: &[f32]) -> Result<Tensor>;
}
```

### Feature Flags (Cargo.toml)
```toml
[features]
default = ["cpu"]           # CPU only by default
cpu = ["burn-ndarray"]      # CPU backend
cuda = ["burn-cuda"]        # NVIDIA CUDA
rocm = ["burn-rocm"]        # AMD ROCm
intel = ["burn-intel"]      # Intel oneAPI
gpu-auto = ["cuda", "rocm", "intel"]  # All GPUs
```

## Backend-Specific Details

### CUDA (NVIDIA)
```rust
// src/hrm/gpu/cuda.rs
#[cfg(feature = "cuda")]
pub struct CudaBackend {
    device: CudaDevice,
}

impl GpuBackend for CudaBackend {
    fn is_available(&self) -> bool {
        // Check for NVIDIA GPU
        // Check CUDA driver
    }
}
```

### ROCm (AMD)
```rust
// src/hrm/gpu/rocm.rs
#[cfg(feature = "rocm")]
pub struct RocmBackend {
    device: RocmDevice,
}
```

### oneAPI (Intel)
```rust
// src/hrm/gpu/oneapi.rs
#[cfg(feature = "intel")]
pub struct OneApiBackend {
    device: OneApiDevice,
}
```

## Runtime Selection
```rust
// src/hrm/gpu/mod.rs
pub fn create_hrm_with_best_backend() -> HRM {
    let backends = GpuDetector::detect_available_backends();
    
    for backend_type in backends {
        if let Ok(backend) = create_backend(backend_type) {
            log::info!("Using backend: {}", backend.name());
            return HRM::with_backend(backend);
        }
    }
    
    // Guaranteed fallback to CPU
    HRM::default()
}
```

## Benchmarks
```rust
// benches/gpu_backends.rs
#[bench]
fn bench_cuda_inference(b: &mut Bencher) {
    let hrm = HRM::with_backend(CudaBackend::new().unwrap());
    b.iter(|| hrm.infer(&input));
}

#[bench]
fn bench_cpu_inference(b: &mut Bencher) {
    let hrm = HRM::default();
    b.iter(|| hrm.infer(&input));
}
```

## Expected Performance

| Backend | Expected Latency | Requirements |
|---------|------------------|--------------|
| CUDA    | ~0.1ms          | NVIDIA GPU |
| ROCm    | ~0.2ms          | AMD GPU |
| oneAPI  | ~0.3ms          | Intel GPU |
| CPU     | ~0.3ms          | Any |

## Status: 🔄 IN PROGRESS

---
**Prev**: Sprint 47 - Frontend WebSocket  
**Next**: Sprint 49 - Distributed Inference
