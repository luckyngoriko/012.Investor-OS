//! GPU Backend Abstraction
//! Sprint 48: Multi-Backend GPU Support

use burn::tensor::backend::Backend;
use std::fmt::Debug;

/// Backend types supported by HRM
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendType {
    /// NVIDIA CUDA
    Cuda,
    /// AMD ROCm
    Rocm,
    /// Intel oneAPI
    OneApi,
    /// CPU fallback (NdArray)
    Cpu,
}

impl BackendType {
    /// Get human-readable name
    pub fn name(&self) -> &'static str {
        match self {
            BackendType::Cuda => "CUDA (NVIDIA)",
            BackendType::Rocm => "ROCm (AMD)",
            BackendType::OneApi => "oneAPI (Intel)",
            BackendType::Cpu => "CPU (NdArray)",
        }
    }

    /// Get priority (lower = higher priority)
    pub fn priority(&self) -> u8 {
        match self {
            BackendType::Cuda => 1, // Fastest
            BackendType::Rocm => 2,
            BackendType::OneApi => 3,
            BackendType::Cpu => 100, // Last resort
        }
    }
}

/// Device information
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub backend_type: BackendType,
    pub name: String,
    pub memory_mb: Option<u64>,
    pub compute_units: Option<u32>,
    pub driver_version: Option<String>,
}

/// GPU Backend trait
pub trait GpuBackend: Send + Sync + Debug {
    /// Backend type identifier
    fn backend_type(&self) -> BackendType;

    /// Human-readable name
    fn name(&self) -> &'static str;

    /// Check if backend is functional
    fn is_available(&self) -> bool;

    /// Get device information
    fn device_info(&self) -> Option<DeviceInfo>;

    /// Get inference latency estimate (ms)
    fn estimated_latency_ms(&self) -> f32;
}

/// CPU Backend (NdArray) - Guaranteed fallback
#[derive(Debug)]
pub struct CpuBackend;

impl Default for CpuBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl CpuBackend {
    pub fn new() -> Self {
        Self
    }
}

impl GpuBackend for CpuBackend {
    fn backend_type(&self) -> BackendType {
        BackendType::Cpu
    }

    fn name(&self) -> &'static str {
        "CPU (NdArray)"
    }

    fn is_available(&self) -> bool {
        true // Always available
    }

    fn device_info(&self) -> Option<DeviceInfo> {
        Some(DeviceInfo {
            backend_type: BackendType::Cpu,
            name: "CPU".to_string(),
            memory_mb: None,
            compute_units: std::thread::available_parallelism()
                .ok()
                .map(|n| n.get() as u32),
            driver_version: None,
        })
    }

    fn estimated_latency_ms(&self) -> f32 {
        0.3 // Baseline
    }
}

/// CUDA Backend (NVIDIA)
#[cfg(feature = "cuda")]
#[derive(Debug)]
pub struct CudaBackend;

#[cfg(feature = "cuda")]
impl CudaBackend {
    pub fn new() -> Result<Self, String> {
        // Check if CUDA is available
        if Self::is_cuda_available() {
            Ok(Self)
        } else {
            Err("CUDA not available".to_string())
        }
    }

    fn is_cuda_available() -> bool {
        // Check for CUDA devices via burn-cuda
        // This is a simplified check
        std::env::var("CUDA_VISIBLE_DEVICES").is_ok() || Self::check_cuda_devices()
    }

    fn check_cuda_devices() -> bool {
        // Try to detect via /proc/driver/nvidia/gpus or nvidia-smi
        std::path::Path::new("/proc/driver/nvidia/gpus").exists()
            || std::process::Command::new("nvidia-smi")
                .arg("-L")
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
    }

    fn get_gpu_name() -> Option<String> {
        std::process::Command::new("nvidia-smi")
            .args(["--query-gpu=name", "--format=csv,noheader"])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
    }

    fn get_memory_mb() -> Option<u64> {
        std::process::Command::new("nvidia-smi")
            .args(["--query-gpu=memory.total", "--format=csv,noheader,nounits"])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .and_then(|s| s.trim().parse::<u64>().ok())
    }

    fn get_driver_version() -> Option<String> {
        std::process::Command::new("nvidia-smi")
            .args(["--query-gpu=driver_version", "--format=csv,noheader"])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
    }
}

#[cfg(feature = "cuda")]
impl GpuBackend for CudaBackend {
    fn backend_type(&self) -> BackendType {
        BackendType::Cuda
    }

    fn name(&self) -> &'static str {
        "CUDA (NVIDIA)"
    }

    fn is_available(&self) -> bool {
        Self::is_cuda_available()
    }

    fn device_info(&self) -> Option<DeviceInfo> {
        Some(DeviceInfo {
            backend_type: BackendType::Cuda,
            name: Self::get_gpu_name().unwrap_or_else(|| "NVIDIA GPU".to_string()),
            memory_mb: Self::get_memory_mb(),
            compute_units: None, // Would need CUDA API
            driver_version: Self::get_driver_version(),
        })
    }

    fn estimated_latency_ms(&self) -> f32 {
        0.1 // Fastest
    }
}

/// ROCm Backend (AMD)
#[cfg(feature = "rocm")]
#[derive(Debug)]
pub struct RocmBackend;

#[cfg(feature = "rocm")]
impl RocmBackend {
    pub fn new() -> Result<Self, String> {
        if Self::is_rocm_available() {
            Ok(Self)
        } else {
            Err("ROCm not available".to_string())
        }
    }

    fn is_rocm_available() -> bool {
        std::path::Path::new("/opt/rocm").exists()
            || std::process::Command::new("rocm-smi")
                .arg("-l")
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
    }

    fn get_gpu_name() -> Option<String> {
        std::process::Command::new("rocm-smi")
            .args(["--showproductname"])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| {
                s.lines()
                    .find(|l| l.contains("GPU"))
                    .map(|l| l.split(':').nth(1).unwrap_or("AMD GPU").trim().to_string())
                    .unwrap_or_else(|| "AMD GPU".to_string())
            })
    }
}

#[cfg(feature = "rocm")]
impl GpuBackend for RocmBackend {
    fn backend_type(&self) -> BackendType {
        BackendType::Rocm
    }

    fn name(&self) -> &'static str {
        "ROCm (AMD)"
    }

    fn is_available(&self) -> bool {
        Self::is_rocm_available()
    }

    fn device_info(&self) -> Option<DeviceInfo> {
        Some(DeviceInfo {
            backend_type: BackendType::Rocm,
            name: Self::get_gpu_name().unwrap_or_else(|| "AMD GPU".to_string()),
            memory_mb: None,
            compute_units: None,
            driver_version: None,
        })
    }

    fn estimated_latency_ms(&self) -> f32 {
        0.2
    }
}

/// oneAPI Backend (Intel)
#[cfg(feature = "intel")]
#[derive(Debug)]
pub struct OneApiBackend;

#[cfg(feature = "intel")]
impl OneApiBackend {
    pub fn new() -> Result<Self, String> {
        if Self::is_oneapi_available() {
            Ok(Self)
        } else {
            Err("oneAPI not available".to_string())
        }
    }

    fn is_oneapi_available() -> bool {
        std::env::var("ONEAPI_ROOT").is_ok()
            || std::process::Command::new("sycl-ls")
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
    }
}

#[cfg(feature = "intel")]
impl GpuBackend for OneApiBackend {
    fn backend_type(&self) -> BackendType {
        BackendType::OneApi
    }

    fn name(&self) -> &'static str {
        "oneAPI (Intel)"
    }

    fn is_available(&self) -> bool {
        Self::is_oneapi_available()
    }

    fn device_info(&self) -> Option<DeviceInfo> {
        Some(DeviceInfo {
            backend_type: BackendType::OneApi,
            name: "Intel GPU".to_string(),
            memory_mb: None,
            compute_units: None,
            driver_version: None,
        })
    }

    fn estimated_latency_ms(&self) -> f32 {
        0.25
    }
}
