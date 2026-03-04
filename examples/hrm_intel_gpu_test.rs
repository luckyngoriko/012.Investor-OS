//! HRM GPU Backend Test
//!
//! Tests HRM inference with different backends.
//!
//! Backends:
//! - CPU (ndarray): Always available
//! - wgpu (Vulkan): For Intel/AMD GPUs
//! - CUDA: For NVIDIA GPUs
//!
//! Run with:
//!   cargo run --example hrm_intel_gpu_test              # CPU
//!   cargo run --example hrm_intel_gpu_test --features wgpu  # Intel/AMD GPU

use investor_os::hrm::{HRMConfig, InferenceResult, HRM};

fn main() {
    println!("╔════════════════════════════════════════════════════════════╗");
    println!("║            HRM Backend Test - Sprint 36                    ║");
    println!("╚════════════════════════════════════════════════════════════╝\n");

    // Detect backend
    #[cfg(all(feature = "wgpu", not(feature = "cuda")))]
    let backend_name = "wgpu (Vulkan/Intel/AMD)";

    #[cfg(feature = "cuda")]
    let backend_name = "CUDA (NVIDIA)";

    #[cfg(not(any(feature = "wgpu", feature = "cuda")))]
    let backend_name = "ndarray (CPU)";

    println!("🎮 Active Backend: {}", backend_name);

    // Create HRM with default config
    let config = HRMConfig::default();
    let hrm = HRM::new(&config).expect("Failed to create HRM");

    println!("\n📊 Model Configuration:");
    println!("   Input features: {}", config.input_size);
    println!("   High hidden:    {}", config.high_hidden_size);
    println!("   Low hidden:     {}", config.low_hidden_size);
    println!("   Parameters:     ~{}", config.estimated_parameters());

    // Test scenarios
    let scenarios = vec![
        ("Bull Market", vec![0.8f32, 0.9, 0.7, 15.0, 0.0, 0.5]),
        ("Bear Market", vec![0.3, 0.2, 0.2, 45.0, 1.0, 0.5]),
        ("Sideways", vec![0.5, 0.5, 0.5, 20.0, 2.0, 0.5]),
        ("Crisis", vec![0.1, 0.1, 0.1, 80.0, 3.0, 0.5]),
    ];

    println!("\n🧠 Inference Tests:");
    println!(
        "{:<15} {:>10} {:>12} {:>10} {:>12}",
        "Scenario", "Conviction", "Confidence", "Regime", "Trade?"
    );
    println!("{:-<65}", "");

    for (name, signals) in &scenarios {
        let result = hrm.infer(signals).expect("Inference failed");
        let trade = if result.should_trade(0.7) {
            "✅ YES"
        } else {
            "❌ NO"
        };

        println!(
            "{:<15} {:>10.2} {:>12.2} {:>10?} {:>12}",
            name, result.conviction, result.confidence, result.regime, trade
        );
    }

    // Batch inference benchmark
    println!("\n⚡ Batch Inference:");
    let batch: Vec<Vec<f32>> = scenarios.iter().map(|(_, s)| s.clone()).collect();

    let start = std::time::Instant::now();
    let batch_results = hrm.infer_batch(&batch).expect("Batch failed");
    let elapsed = start.elapsed();

    println!(
        "   Processed {} signals in {:?}",
        batch_results.len(),
        elapsed
    );
    println!(
        "   Average time per inference: {:?}",
        elapsed / batch_results.len() as u32
    );

    // Stats
    let stats = hrm.stats();
    println!("\n📈 Model Stats:");
    println!("   GPU Enabled:    {}", stats.gpu_enabled);
    println!("   Weights Loaded: {}", stats.weights_loaded);

    // GPU info if available
    #[cfg(feature = "wgpu")]
    {
        println!("\n🎮 Vulkan GPU Info:");
        println!("   (Check vulkaninfo for device details)");
    }

    println!("\n╔════════════════════════════════════════════════════════════╗");
    println!("║              Backend Test Complete ✅                       ║");
    println!("╚════════════════════════════════════════════════════════════╝");

    // Recommendations
    println!("\n💡 Recommendations:");
    if backend_name == "ndarray (CPU)" {
        println!("   Current: CPU backend");
        if is_vulkan_available() {
            println!("   💡 Intel GPU detected! Run with --features wgpu for acceleration");
        }
    }
}

fn is_vulkan_available() -> bool {
    std::process::Command::new("vulkaninfo")
        .arg("--summary")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}
