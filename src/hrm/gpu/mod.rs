//! GPU Acceleration Module
//! Sprint 48: Multi-Backend GPU Support
//!
//! Provides automatic GPU detection and backend selection for HRM inference.
//! Supports: CUDA (NVIDIA), ROCm (AMD), oneAPI (Intel), CPU fallback.

use std::sync::OnceLock;
use tracing::{info, warn};

pub mod backend;
pub mod detector;

pub use backend::{BackendType, CpuBackend, DeviceInfo, GpuBackend};
pub use detector::{DetectedBackend, GpuDetector};

/// Global GPU backend singleton
static GPU_BACKEND: OnceLock<Box<dyn GpuBackend>> = OnceLock::new();

/// Initialize GPU backend with auto-detection
pub fn init_gpu_backend() -> &'static dyn GpuBackend {
    GPU_BACKEND
        .get_or_init(|| {
            info!("Initializing GPU backend with auto-detection...");

            let detector = GpuDetector::new();
            let available = detector.detect_all();

            info!("Available backends: {:?}", available);

            // Try each backend in priority order
            for detected in &available {
                match create_backend(detected.backend_type) {
                    Ok(backend) => {
                        info!(
                            "✅ Using backend: {} ({})",
                            backend.name(),
                            detected.device_name
                        );
                        return backend;
                    }
                    Err(e) => {
                        warn!("❌ Failed to initialize {:?}: {}", detected.backend_type, e);
                    }
                }
            }

            // Guaranteed CPU fallback
            info!("⚠️  No GPU backend available, using CPU fallback");
            Box::new(backend::CpuBackend::new())
        })
        .as_ref()
}

/// Create specific backend by type
fn create_backend(backend_type: BackendType) -> Result<Box<dyn GpuBackend>, String> {
    match backend_type {
        #[cfg(feature = "cuda")]
        BackendType::Cuda => {
            backend::CudaBackend::new().map(|b| Box::new(b) as Box<dyn GpuBackend>)
        }

        #[cfg(feature = "rocm")]
        BackendType::Rocm => {
            backend::RocmBackend::new().map(|b| Box::new(b) as Box<dyn GpuBackend>)
        }

        #[cfg(feature = "intel")]
        BackendType::OneApi => {
            backend::OneApiBackend::new().map(|b| Box::new(b) as Box<dyn GpuBackend>)
        }

        BackendType::Cpu => Ok(Box::new(backend::CpuBackend::new()) as Box<dyn GpuBackend>),

        #[allow(unreachable_patterns)]
        _ => Err(format!("Backend {:?} not compiled", backend_type)),
    }
}

/// Get current GPU backend info
pub fn get_gpu_info() -> Option<DeviceInfo> {
    init_gpu_backend().device_info()
}

/// Check if running on GPU (not CPU)
pub fn is_gpu_accelerated() -> bool {
    !matches!(init_gpu_backend().backend_type(), BackendType::Cpu)
}

/// List all available backends (for diagnostics)
pub fn list_available_backends() -> Vec<DetectedBackend> {
    GpuDetector::new().detect_all()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_backend_always_available() {
        let cpu = backend::CpuBackend::new();
        assert!(cpu.is_available());
        assert_eq!(cpu.backend_type(), BackendType::Cpu);
    }

    #[test]
    fn test_backend_detection() {
        let detector = GpuDetector::new();
        let backends = detector.detect_all();

        // CPU should always be detected
        assert!(backends.iter().any(|b| b.backend_type == BackendType::Cpu));
    }

    #[test]
    fn test_gpu_info() {
        let info = get_gpu_info();
        assert!(info.is_some());
    }
}
