//! GPU Detection Utility
//! Sprint 48: Multi-Backend GPU Support
//!
//! Run with: cargo run --bin gpu-check

use investor_os::hrm::gpu::{GpuDetector, init_gpu_backend, is_gpu_accelerated};

fn main() {
    println!("╔═══════════════════════════════════════════════════════════════╗");
    println!("║       Investor OS - GPU Backend Detection (Sprint 48)         ║");
    println!("╚═══════════════════════════════════════════════════════════════╝");
    println!();

    // Run diagnostics
    let detector = GpuDetector::new();
    detector.print_diagnostics();
    
    println!();
    println!("───────────────────────────────────────────────────────────────");
    println!("Initializing selected backend...");
    
    // Initialize and get info
    let backend = init_gpu_backend();
    
    if let Some(info) = backend.device_info() {
        println!();
        println!("✅ Active Backend:");
        println!("   Name:        {}", info.name);
        println!("   Type:        {:?}", info.backend_type);
        println!("   Latency:     ~{:.1}ms", backend.estimated_latency_ms());
        
        if let Some(memory) = info.memory_mb {
            println!("   GPU Memory:  {} MB", memory);
        }
        
        if let Some(cu) = info.compute_units {
            println!("   Compute Units: {}", cu);
        }
        
        if let Some(driver) = info.driver_version {
            println!("   Driver:      {}", driver);
        }
    }
    
    println!();
    println!("───────────────────────────────────────────────────────────────");
    
    if is_gpu_accelerated() {
        println!("🎮 GPU Acceleration: ENABLED");
        println!("   Your HRM inference is using GPU acceleration!");
    } else {
        println!("⚠️  GPU Acceleration: DISABLED");
        println!("   Using CPU fallback.");
        println!();
        println!("   To enable GPU acceleration:");
        println!("   • NVIDIA: Install CUDA Toolkit + run with --features cuda");
        println!("   • AMD:    Install ROCm + run with --features rocm");
        println!("   • Intel:  Install oneAPI + run with --features intel");
        println!("   • Auto:   Run with --features gpu-auto (detects all)");
    }
    
    println!();
    println!("───────────────────────────────────────────────────────────────");
    println!("API Endpoints:");
    println!("  GET /api/v1/gpu/info        - Current GPU info");
    println!("  GET /api/v1/gpu/diagnostics - Full diagnostics");
    println!("  GET /api/v1/gpu/benchmark   - Backend benchmarks");
    println!("───────────────────────────────────────────────────────────────");
}
