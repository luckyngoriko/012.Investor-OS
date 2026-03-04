//! HRM Model Loading Tests
//!
//! Tests loading trained models and running inference.

use investor_os::hrm::{HRMBuilder, HRMConfig, HRM};
use std::path::Path;

/// Test loading a trained SafeTensors model
#[test]
fn test_load_safetensors_model() {
    let model_path = "models/hrm_synthetic_v1.safetensors";

    // Skip if model doesn't exist
    if !Path::new(model_path).exists() {
        println!("⚠️  Model file not found: {}", model_path);
        println!("   Run: python scripts/hrm/train_hrm.py");
        return;
    }

    let hrm = HRMBuilder::new().with_weights(model_path).build();

    assert!(hrm.is_ok(), "Should load model successfully");

    let hrm = hrm.unwrap();
    assert!(
        hrm.is_ready(),
        "Model should be ready after loading weights"
    );

    let stats = hrm.stats();
    println!("📊 Loaded model stats:");
    println!("   Parameters: {}", stats.parameters);
    println!("   Weights loaded: {}", stats.weights_loaded);
    println!("   Network loaded: {}", stats.network_loaded);
}

/// Test inference with loaded model
#[test]
fn test_inference_with_loaded_model() {
    let model_path = "models/hrm_synthetic_v1.safetensors";

    if !Path::new(model_path).exists() {
        println!("⚠️  Skipping test - model not found");
        return;
    }

    let hrm = HRMBuilder::new()
        .with_weights(model_path)
        .build()
        .expect("Should build HRM");

    // Test inference
    let signals = vec![0.8, 0.9, 0.7, 15.0, 0.0, 0.5];
    let result = hrm.infer(&signals).expect("Inference should succeed");

    println!("\n🧠 Inference result:");
    println!("   Conviction: {:.4}", result.conviction);
    println!("   Confidence: {:.4}", result.confidence);
    println!("   Regime: {:?}", result.regime);
    println!("   Latency: {} μs", result.latency_us);

    // Validate output ranges
    assert!(result.conviction >= 0.0 && result.conviction <= 1.0);
    assert!(result.confidence >= 0.0 && result.confidence <= 1.0);
}

/// Test golden dataset comparison
#[test]
fn test_golden_dataset() {
    use std::fs;

    let model_path = "models/hrm_synthetic_v1.safetensors";
    let golden_path = "data/hrm/golden/reference_cases.json";

    if !Path::new(model_path).exists() || !Path::new(golden_path).exists() {
        println!("⚠️  Skipping test - required files not found");
        return;
    }

    let hrm = HRMBuilder::new()
        .with_weights(model_path)
        .build()
        .expect("Should build HRM");

    // Load golden cases
    let golden_data: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(golden_path).expect("Should read golden file"))
            .expect("Should parse JSON");

    let cases = golden_data.as_array().expect("Should be array");

    println!("\n📊 Testing {} golden cases:", cases.len());

    let mut passed = 0;
    let mut failed = 0;

    for case in cases.iter().take(10) {
        // Test first 10
        let name = case["name"].as_str().unwrap_or("unknown");
        let input: Vec<f32> = case["input"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_f64().unwrap_or(0.0) as f32)
            .collect();

        let expected = case["expected"]["conviction"].as_f64().unwrap_or(0.0) as f32;

        let result = hrm.infer(&input).expect("Inference failed");
        let actual = result.conviction;

        // Allow 10% tolerance (model might differ slightly from ground truth)
        let diff = (actual - expected).abs();
        let tolerance = 0.10;

        if diff <= tolerance {
            println!(
                "  ✅ {}: expected {:.2}, got {:.2} (diff: {:.2})",
                name, expected, actual, diff
            );
            passed += 1;
        } else {
            println!(
                "  ❌ {}: expected {:.2}, got {:.2} (diff: {:.2})",
                name, expected, actual, diff
            );
            failed += 1;
        }
    }

    println!("\n📈 Results: {} passed, {} failed", passed, failed);

    // Require at least 70% pass rate
    let pass_rate = passed as f32 / (passed + failed) as f32;
    assert!(
        pass_rate >= 0.7,
        "Golden dataset pass rate {:.1}% below threshold 70%",
        pass_rate * 100.0
    );
}

/// Test batch inference performance
#[test]
fn test_batch_performance() {
    use std::time::Instant;

    let model_path = "models/hrm_synthetic_v1.safetensors";

    if !Path::new(model_path).exists() {
        println!("⚠️  Skipping test - model not found");
        return;
    }

    let hrm = HRMBuilder::new()
        .with_weights(model_path)
        .build()
        .expect("Should build HRM");

    // Create batch
    let batch: Vec<Vec<f32>> = (0..100)
        .map(|_| vec![0.5, 0.5, 0.5, 25.0, 1.0, 0.5])
        .collect();

    let start = Instant::now();
    let results = hrm.infer_batch(&batch).expect("Batch inference failed");
    let elapsed = start.elapsed();

    let avg_ms = elapsed.as_millis() as f64 / batch.len() as f64;

    println!("\n⚡ Batch Performance:");
    println!("   Samples: {}", batch.len());
    println!("   Total time: {:?}", elapsed);
    println!("   Average: {:.3} ms per inference", avg_ms);

    // Should be reasonably fast (placeholder is < 1ms)
    assert!(
        avg_ms < 10.0,
        "Average inference too slow: {:.3} ms",
        avg_ms
    );
}
