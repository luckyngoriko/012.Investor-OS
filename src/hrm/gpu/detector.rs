//! GPU Detection Module
//! Sprint 48: Multi-Backend GPU Support
//!
//! Detects available GPU devices and returns them in priority order.

use super::backend::BackendType;

/// Detected backend with metadata
#[derive(Debug, Clone)]
pub struct DetectedBackend {
    pub backend_type: BackendType,
    pub device_name: String,
    pub available: bool,
    pub priority: u8,
}

/// GPU device detector
#[derive(Debug)]
pub struct GpuDetector;

impl GpuDetector {
    /// Create new detector
    pub fn new() -> Self {
        Self
    }

    /// Detect all available backends (sorted by priority)
    pub fn detect_all(&self) -> Vec<DetectedBackend> {
        let mut backends = Vec::new();

        // Try CUDA first (highest priority)
        if let Some(cuda) = self.detect_cuda() {
            backends.push(cuda);
        }

        // Try ROCm (AMD)
        if let Some(rocm) = self.detect_rocm() {
            backends.push(rocm);
        }

        // Try oneAPI (Intel)
        if let Some(intel) = self.detect_oneapi() {
            backends.push(intel);
        }

        // CPU is always available (lowest priority)
        backends.push(DetectedBackend {
            backend_type: BackendType::Cpu,
            device_name: "CPU (NdArray)".to_string(),
            available: true,
            priority: BackendType::Cpu.priority(),
        });

        // Sort by priority
        backends.sort_by_key(|b| b.priority);

        backends
    }

    /// Get best available backend
    pub fn best_available(&self) -> Option<DetectedBackend> {
        self.detect_all().into_iter().find(|b| b.available)
    }

    /// Check if any GPU is available (not just CPU)
    pub fn has_gpu(&self) -> bool {
        self.detect_all()
            .iter()
            .any(|b| b.available && b.backend_type != BackendType::Cpu)
    }

    /// Detect CUDA devices
    fn detect_cuda(&self) -> Option<DetectedBackend> {
        #[cfg(not(feature = "cuda"))]
        return None;

        #[cfg(feature = "cuda")]
        {
            // Check for nvidia-smi
            let output = Command::new("nvidia-smi")
                .args(["--query-gpu=name,memory.total", "--format=csv,noheader"])
                .output();

            match output {
                Ok(output) if output.status.success() => {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    let first_line = stdout.lines().next()?;
                    let parts: Vec<&str> = first_line.split(',').collect();

                    let name = parts.get(0)?.trim().to_string();
                    let memory = parts
                        .get(1)
                        .map(|m| m.trim().to_string())
                        .unwrap_or_default();

                    Some(DetectedBackend {
                        backend_type: BackendType::Cuda,
                        device_name: format!("{} ({})", name, memory),
                        available: true,
                        priority: BackendType::Cuda.priority(),
                    })
                }
                _ => {
                    // Check for CUDA via env or /proc
                    if std::env::var("CUDA_VISIBLE_DEVICES").is_ok()
                        || std::path::Path::new("/proc/driver/nvidia/gpus").exists()
                    {
                        Some(DetectedBackend {
                            backend_type: BackendType::Cuda,
                            device_name: "NVIDIA GPU (detected)".to_string(),
                            available: true,
                            priority: BackendType::Cuda.priority(),
                        })
                    } else {
                        None
                    }
                }
            }
        }
    }

    /// Detect ROCm devices (AMD)
    fn detect_rocm(&self) -> Option<DetectedBackend> {
        #[cfg(not(feature = "rocm"))]
        return None;

        #[cfg(feature = "rocm")]
        {
            // Check for rocm-smi
            let output = Command::new("rocm-smi")
                .args(["--showproductname"])
                .output();

            match output {
                Ok(output) if output.status.success() => {
                    let stdout = String::from_utf8_lossy(&output.stdout);

                    // Parse output for GPU name
                    if let Some(line) = stdout.lines().find(|l| l.contains("GPU")) {
                        let name = line
                            .split(':')
                            .nth(1)
                            .map(|s| s.trim().to_string())
                            .unwrap_or_else(|| "AMD GPU".to_string());

                        Some(DetectedBackend {
                            backend_type: BackendType::Rocm,
                            device_name: name,
                            available: true,
                            priority: BackendType::Rocm.priority(),
                        })
                    } else {
                        None
                    }
                }
                _ => {
                    // Check for ROCm installation
                    if std::path::Path::new("/opt/rocm").exists() {
                        Some(DetectedBackend {
                            backend_type: BackendType::Rocm,
                            device_name: "AMD GPU (ROCm detected)".to_string(),
                            available: true,
                            priority: BackendType::Rocm.priority(),
                        })
                    } else {
                        None
                    }
                }
            }
        }
    }

    /// Detect oneAPI devices (Intel)
    fn detect_oneapi(&self) -> Option<DetectedBackend> {
        #[cfg(not(feature = "intel"))]
        return None;

        #[cfg(feature = "intel")]
        {
            // Check for sycl-ls
            let output = Command::new("sycl-ls").output();

            match output {
                Ok(output) if output.status.success() => {
                    let stdout = String::from_utf8_lossy(&output.stdout);

                    // Look for GPU devices
                    if let Some(line) = stdout
                        .lines()
                        .find(|l| l.contains("[gpu]") || l.contains("GPU"))
                    {
                        let name = line
                            .split(':')
                            .last()
                            .map(|s| s.trim().to_string())
                            .unwrap_or_else(|| "Intel GPU".to_string());

                        Some(DetectedBackend {
                            backend_type: BackendType::OneApi,
                            device_name: name,
                            available: true,
                            priority: BackendType::OneApi.priority(),
                        })
                    } else {
                        None
                    }
                }
                _ => {
                    // Check for ONEAPI_ROOT
                    if std::env::var("ONEAPI_ROOT").is_ok() {
                        Some(DetectedBackend {
                            backend_type: BackendType::OneApi,
                            device_name: "Intel GPU (oneAPI detected)".to_string(),
                            available: true,
                            priority: BackendType::OneApi.priority(),
                        })
                    } else {
                        None
                    }
                }
            }
        }
    }

    /// Print diagnostic information
    pub fn print_diagnostics(&self) {
        println!("╔═══════════════════════════════════════════════════════════╗");
        println!("║           GPU Detection Diagnostics (Sprint 48)           ║");
        println!("╚═══════════════════════════════════════════════════════════╝");

        let backends = self.detect_all();

        if backends.is_empty() {
            println!("❌ No backends detected!");
            return;
        }

        for (i, backend) in backends.iter().enumerate() {
            let icon = if backend.available { "✅" } else { "❌" };
            let selected = if i == 0 { " ← SELECTED" } else { "" };

            println!();
            println!("{}. {} {}", i + 1, icon, backend.backend_type.name());
            println!("   Device: {}", backend.device_name);
            println!("   Priority: {}{}", backend.priority, selected);
        }

        println!();
        println!("═══════════════════════════════════════════════════════════");

        if self.has_gpu() {
            println!("🎮 GPU acceleration available!");
        } else {
            println!("⚠️  No GPU detected, using CPU fallback");
        }
    }
}

impl Default for GpuDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detector_new() {
        let detector = GpuDetector::new();
        assert!(!detector.detect_all().is_empty()); // At least CPU
    }

    #[test]
    fn test_cpu_always_detected() {
        let detector = GpuDetector::new();
        let backends = detector.detect_all();

        assert!(backends.iter().any(|b| b.backend_type == BackendType::Cpu));
    }

    #[test]
    fn test_priority_ordering() {
        let detector = GpuDetector::new();
        let backends = detector.detect_all();

        // Check sorted by priority
        for i in 1..backends.len() {
            assert!(backends[i - 1].priority <= backends[i].priority);
        }
    }

    #[test]
    fn test_best_available() {
        let detector = GpuDetector::new();
        let best = detector.best_available();

        assert!(best.is_some());
        assert!(best.unwrap().available);
    }
}
