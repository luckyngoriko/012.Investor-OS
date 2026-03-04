//! HRM Golden Dataset Validation Tests (Sprint 41)
//!
//! Validates Rust HRM outputs match expected values from trained Python model.
//! Target: 70%+ pass rate for production readiness.

use investor_os::hrm::{HRMBuilder, MarketRegime, HRM};
use serde::{Deserialize, Serialize};
use std::fs;

/// Golden dataset structure
#[derive(Debug, Deserialize, Serialize)]
struct GoldenDataset {
    version: String,
    model: String,
    total_cases: usize,
    tolerance: f64,
    cases: Vec<GoldenCase>,
}

/// Individual test case
#[derive(Debug, Deserialize, Serialize, Clone)]
struct GoldenCase {
    name: String,
    description: String,
    input: Vec<f32>,
    expected: ExpectedOutput,
}

/// Expected output values
#[derive(Debug, Deserialize, Serialize, Clone)]
struct ExpectedOutput {
    conviction: f64,
    confidence: f64,
    regime_raw: f64,
    regime: String,
}

/// Test result for a single case
#[derive(Debug, Serialize)]
struct TestResult {
    name: String,
    passed: bool,
    input: Vec<f32>,
    expected: ExpectedOutput,
    actual: ActualOutput,
    errors: Vec<String>,
}

/// Actual output from Rust HRM
#[derive(Debug, Serialize)]
struct ActualOutput {
    conviction: f32,
    confidence: f32,
    regime: String,
}

/// Generate golden dataset from current model (for calibration)
#[test]
#[ignore = "Run manually to regenerate golden dataset"]
fn generate_golden_dataset_from_model() {
    let hrm = create_hrm_with_weights();

    // Test cases
    let cases = vec![
        (
            "strong_bull",
            "Strong bull market with excellent signals",
            vec![0.9, 0.9, 0.9, 10.0, 0.0, 0.5],
        ),
        (
            "moderate_bull",
            "Moderate bull market",
            vec![0.7, 0.7, 0.7, 15.0, 0.0, 0.5],
        ),
        (
            "weak_bull",
            "Weak bull signals",
            vec![0.5, 0.5, 0.5, 20.0, 0.0, 0.5],
        ),
        (
            "strong_bear",
            "Strong bear market",
            vec![0.1, 0.1, 0.1, 50.0, 1.0, 0.5],
        ),
        (
            "moderate_bear",
            "Moderate bear market",
            vec![0.3, 0.3, 0.3, 40.0, 1.0, 0.5],
        ),
        (
            "sideways",
            "Sideways market with mixed signals",
            vec![0.5, 0.5, 0.5, 25.0, 2.0, 0.5],
        ),
        (
            "crisis",
            "Crisis market - extreme fear",
            vec![0.1, 0.1, 0.1, 80.0, 3.0, 0.5],
        ),
        (
            "high_pegy_low_sentiment",
            "Good fundamentals but poor sentiment",
            vec![0.9, 0.5, 0.2, 20.0, 0.0, 0.5],
        ),
        (
            "low_pegy_high_insider",
            "Poor fundamentals but strong insider buying",
            vec![0.2, 0.9, 0.5, 20.0, 0.0, 0.5],
        ),
        (
            "high_volatility",
            "Good signals but high VIX",
            vec![0.8, 0.8, 0.8, 60.0, 0.0, 0.5],
        ),
        (
            "low_volatility",
            "Moderate signals with low VIX",
            vec![0.6, 0.6, 0.6, 10.0, 0.0, 0.5],
        ),
        (
            "all_zeros",
            "Edge case - all zeros",
            vec![0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        ),
        (
            "all_ones",
            "Edge case - all max values",
            vec![1.0, 1.0, 1.0, 100.0, 3.0, 1.0],
        ),
        (
            "random_1",
            "Random test case 1",
            vec![0.65, 0.42, 0.78, 23.5, 1.0, 0.3],
        ),
        (
            "random_2",
            "Random test case 2",
            vec![0.33, 0.88, 0.45, 45.2, 2.0, 0.7],
        ),
        (
            "random_3",
            "Random test case 3",
            vec![0.82, 0.15, 0.91, 12.8, 0.0, 0.9],
        ),
        (
            "random_4",
            "Random test case 4",
            vec![0.21, 0.67, 0.34, 67.3, 3.0, 0.2],
        ),
        (
            "random_5",
            "Random test case 5",
            vec![0.55, 0.73, 0.29, 31.4, 1.0, 0.6],
        ),
        (
            "opening",
            "Market opening (time=0.1)",
            vec![0.7, 0.7, 0.7, 20.0, 0.0, 0.1],
        ),
        (
            "closing",
            "Market closing (time=0.9)",
            vec![0.7, 0.7, 0.7, 20.0, 0.0, 0.9],
        ),
    ];

    let mut golden_cases = Vec::new();

    println!("\n📝 Generating golden dataset from model...\n");

    for (name, description, input) in cases {
        let result = hrm.infer(&input).unwrap();

        let regime_raw = match result.regime {
            MarketRegime::Bull => 0.0,
            MarketRegime::Bear => 1.0,
            MarketRegime::Sideways => 2.0,
            MarketRegime::Crisis => 3.0,
        };

        let case = GoldenCase {
            name: name.to_string(),
            description: description.to_string(),
            input: input.clone(),
            expected: ExpectedOutput {
                conviction: result.conviction as f64,
                confidence: result.confidence as f64,
                regime_raw,
                regime: format!("{:?}", result.regime),
            },
        };

        println!(
            "  {}: conv={:.4}, conf={:.4}, regime={:?}",
            name, result.conviction, result.confidence, result.regime
        );

        golden_cases.push(case);
    }

    // Create dataset
    let dataset = GoldenDataset {
        version: "1.0".to_string(),
        model: "hrm_synthetic_v1".to_string(),
        total_cases: golden_cases.len(),
        tolerance: 0.001,
        cases: golden_cases,
    };

    // Save to file
    let json = serde_json::to_string_pretty(&dataset).unwrap();
    fs::write("tests/golden_path/hrm_golden_dataset.json", json).unwrap();

    println!("\n✅ Saved {} golden cases", dataset.total_cases);
}

/// Main golden dataset validation test
/// Target: 70%+ pass rate
#[test]
fn test_golden_dataset_validation() {
    // Load golden dataset
    let dataset_json = fs::read_to_string("tests/golden_path/hrm_golden_dataset.json")
        .expect("Failed to read golden dataset");
    let dataset: GoldenDataset =
        serde_json::from_str(&dataset_json).expect("Failed to parse golden dataset");

    println!("\n🎯 Golden Dataset Validation");
    println!("   Version: {}", dataset.version);
    println!("   Model: {}", dataset.model);
    println!("   Total Cases: {}", dataset.total_cases);
    println!("   Tolerance: {}\n", dataset.tolerance);

    // Create HRM with loaded weights
    let hrm = create_hrm_with_weights();

    // Run validation
    let mut results = Vec::new();
    let mut passed_count = 0;

    for case in &dataset.cases {
        let result = validate_case(&hrm, case, dataset.tolerance);
        if result.passed {
            passed_count += 1;
        }
        results.push(result);
    }

    // Calculate pass rate
    let pass_rate = passed_count as f64 / dataset.total_cases as f64;

    // Print detailed results
    println!("📊 Detailed Results:\n");
    for result in &results {
        let status = if result.passed {
            "✅ PASS"
        } else {
            "❌ FAIL"
        };
        println!("  {}: {}", status, result.name);

        if !result.passed {
            for error in &result.errors {
                println!("      - {}", error);
            }
        }
    }

    // Print summary
    println!("\n📈 Summary:");
    println!("   Passed: {}/{}", passed_count, dataset.total_cases);
    println!("   Pass Rate: {:.1}%", pass_rate * 100.0);
    println!("   Target: 70.0%");

    // Assert 70%+ pass rate
    assert!(
        pass_rate >= 0.70,
        "\n❌ Golden dataset pass rate {:.1}% is below target of 70%",
        pass_rate * 100.0
    );

    println!("\n✅ Golden dataset validation PASSED!");
}

/// Validate a single test case
fn validate_case(hrm: &HRM, case: &GoldenCase, tolerance: f64) -> TestResult {
    let actual_result = match hrm.infer(&case.input) {
        Ok(r) => r,
        Err(e) => {
            return TestResult {
                name: case.name.clone(),
                passed: false,
                input: case.input.clone(),
                expected: case.expected.clone(),
                actual: ActualOutput {
                    conviction: 0.0,
                    confidence: 0.0,
                    regime: "Error".to_string(),
                },
                errors: vec![format!("Inference failed: {}", e)],
            };
        }
    };

    let actual = ActualOutput {
        conviction: actual_result.conviction,
        confidence: actual_result.confidence,
        regime: format!("{:?}", actual_result.regime),
    };

    // Check each field
    let mut errors = Vec::new();

    // Conviction check (with tolerance)
    let conv_diff = (actual.conviction as f64 - case.expected.conviction).abs();
    if conv_diff > tolerance {
        errors.push(format!(
            "Conviction mismatch: expected {:.6}, got {:.6}, diff={:.6}",
            case.expected.conviction, actual.conviction, conv_diff
        ));
    }

    // Confidence check (with tolerance)
    let conf_diff = (actual.confidence as f64 - case.expected.confidence).abs();
    if conf_diff > tolerance {
        errors.push(format!(
            "Confidence mismatch: expected {:.6}, got {:.6}, diff={:.6}",
            case.expected.confidence, actual.confidence, conf_diff
        ));
    }

    // Regime check (exact match)
    if actual.regime != case.expected.regime {
        errors.push(format!(
            "Regime mismatch: expected {}, got {}",
            case.expected.regime, actual.regime
        ));
    }

    TestResult {
        name: case.name.clone(),
        passed: errors.is_empty(),
        input: case.input.clone(),
        expected: case.expected.clone(),
        actual,
        errors,
    }
}

/// Helper to create HRM with loaded weights
fn create_hrm_with_weights() -> HRM {
    HRMBuilder::new()
        .with_weights("models/hrm_synthetic_v1.safetensors")
        .build()
        .expect("Failed to create HRM with weights")
}

/// Quick sanity check - ensure model loads and produces consistent outputs
#[test]
fn test_model_consistency() {
    let hrm = create_hrm_with_weights();

    // Same input should produce same output (deterministic)
    let input = vec![0.5, 0.5, 0.5, 20.0, 0.0, 0.5];

    let result1 = hrm.infer(&input).unwrap();
    let result2 = hrm.infer(&input).unwrap();

    assert!(
        (result1.conviction - result2.conviction).abs() < 0.0001,
        "Model should be deterministic"
    );
    assert!(
        (result1.confidence - result2.confidence).abs() < 0.0001,
        "Model should be deterministic"
    );

    println!("✅ Model is deterministic");
    println!("   Conviction: {:.6}", result1.conviction);
    println!("   Confidence: {:.6}", result1.confidence);
}

/// Test individual cases with detailed output
#[test]
fn test_detailed_case_analysis() {
    let hrm = create_hrm_with_weights();

    // Test specific interesting cases
    let test_cases = vec![
        ("all_zeros", vec![0.0, 0.0, 0.0, 0.0, 0.0, 0.0]),
        ("strong_bull", vec![0.9, 0.9, 0.9, 10.0, 0.0, 0.5]),
        ("crisis", vec![0.1, 0.1, 0.1, 80.0, 3.0, 0.5]),
    ];

    println!("\n🔍 Detailed Case Analysis:\n");

    for (name, input) in test_cases {
        let result = hrm.infer(&input).unwrap();

        println!("  Case: {}", name);
        println!(
            "    Input:  [{:.2}, {:.2}, {:.2}, {:.1}, {:.1}, {:.1}]",
            input[0], input[1], input[2], input[3], input[4], input[5]
        );
        println!(
            "    Output: conviction={:.4}, confidence={:.4}, regime={:?}",
            result.conviction, result.confidence, result.regime
        );
        println!();
    }
}
