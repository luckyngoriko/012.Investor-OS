//! HRM Demo - Investor OS
//!
//! Run with: cargo run --example hrm_demo

use investor_os::hrm::{DeviceConfig, HRMBuilder, HRMConfig, HRM};

fn main() {
    println!("╔════════════════════════════════════════════════════════════╗");
    println!("║         Investor OS - HRM (Sprint 36) Demo                 ║");
    println!("║         RTX 3090 Ready | Native Rust ML                    ║");
    println!("╚════════════════════════════════════════════════════════════╝\n");

    // Method 1: Create with default config
    let config = HRMConfig::default();
    let hrm = HRM::new(&config).expect("Failed to create HRM");

    println!("✅ HRM created with default config");
    println!("   📥 Input size:  {}", hrm.config().input_size);
    println!("   🧠 High hidden: {}", hrm.config().high_hidden_size);
    println!("   ⚡ Low hidden:  {}", hrm.config().low_hidden_size);
    println!("   🎯 Threshold:   {}", hrm.config().confidence_threshold);

    // Method 2: Builder pattern
    let _hrm2 = HRMBuilder::new()
        .with_input_size(6)
        .with_hidden_sizes(128, 64)
        .with_confidence_threshold(0.75)
        .with_device(DeviceConfig::Auto)
        .build()
        .expect("Failed to build HRM");

    println!("\n✅ HRM created with builder pattern");

    // Test scenarios
    let scenarios = vec![
        ("🐂 Bull Market", vec![0.8, 0.9, 0.7, 15.0, 0.0, 0.5]),
        ("🐻 Bear Market", vec![0.3, 0.2, 0.2, 45.0, 1.0, 0.5]),
        ("📊 Sideways", vec![0.5, 0.5, 0.5, 20.0, 2.0, 0.5]),
        ("⚠️  Crisis", vec![0.1, 0.1, 0.1, 80.0, 3.0, 0.5]),
    ];

    println!("\n📊 Inference Results:");
    println!(
        "{:<20} {:>10} {:>12} {:>12}",
        "Scenario", "Conviction", "Confidence", "Trade?"
    );
    println!("{:-<60}", "");

    for (name, signals) in &scenarios {
        let result = hrm.infer(&signals).expect("Inference failed");
        let trade = if result.should_trade(0.7) {
            "✅ YES"
        } else {
            "❌ NO"
        };

        println!(
            "{:<20} {:>10.2} {:>12.2} {:>12}",
            name, result.conviction, result.confidence, trade
        );
    }

    // Stats
    let stats = hrm.stats();
    println!("\n📈 Model Stats:");
    println!("   🔢 Parameters:     {}", stats.parameters);
    println!("   🎮 GPU enabled:    {}", stats.gpu_enabled);
    println!("   💾 Weights loaded: {}", stats.weights_loaded);
    println!("   📥 Input dim:      {}", stats.input_size);
    println!("   📤 Output dim:     {}", stats.output_size);

    // Batch inference
    println!("\n⚡ Batch Inference Test:");
    let batch: Vec<Vec<f32>> = scenarios
        .iter()
        .map(|(_, signals)| signals.clone())
        .collect();

    let batch_results = hrm.infer_batch(&batch).expect("Batch inference failed");
    println!("   Processed {} signals in batch", batch_results.len());

    let avg_conviction: f32 =
        batch_results.iter().map(|r| r.conviction).sum::<f32>() / batch_results.len() as f32;

    println!("   Average conviction: {:.2}", avg_conviction);

    println!("\n╔════════════════════════════════════════════════════════════╗");
    println!("║                    Demo Complete ✅                         ║");
    println!("╚════════════════════════════════════════════════════════════╝");
}
