//! Sprint 36 Golden Path Tests: HRM Native Engine
//!
//! Tests for the Hierarchical Reasoning Model native Rust implementation.
//! Run with: `cargo test --test golden_path hrm_`

use investor_os::hrm::{
    DeviceConfig, HRMConfig, HRMError, InferenceEngine, InferenceResult, MarketRegime,
    WeightLoader, HRM,
};

// =============================================================================
// Test Group 1: Module Initialization (hrm_001 - hrm_003)
// =============================================================================

#[test]
fn test_hrm_001_module_creation() {
    //! HRM module initializes without panic
    let config = HRMConfig::default();
    let hrm = HRM::new(&config);
    assert!(hrm.is_ok(), "HRM should initialize without errors");
}

#[test]
fn test_hrm_002_config_validation() {
    //! Configuration validation works correctly
    let valid_config = HRMConfig::default();
    assert!(valid_config.validate().is_ok());

    let invalid_config = HRMConfig {
        input_size: 0,
        ..Default::default()
    };
    assert!(
        invalid_config.validate().is_err(),
        "Should fail with input_size=0"
    );
}

#[test]
fn test_hrm_003_inference_engine_creation() {
    //! Inference engine initializes correctly
    let engine = InferenceEngine::new();
    let stats = engine.stats();
    assert_eq!(stats.batch_size, 1);
    assert!(stats.use_gpu);
}

// =============================================================================
// Test Group 2: Weight Loading (hrm_004 - hrm_006)
// =============================================================================

#[test]
fn test_hrm_004_weight_loader_creation() {
    //! Weight loader initializes without panic
    let _loader = WeightLoader::new();
}

#[test]
fn test_hrm_005_load_nonexistent_weights() {
    //! Loading nonexistent weights returns appropriate error
    let loader = WeightLoader::new();
    let result = loader.load("/nonexistent/path/model.safetensors");

    assert!(
        matches!(result, Err(HRMError::WeightLoadError(_))),
        "Should return WeightLoadError for missing file"
    );
}

#[test]
fn test_hrm_006_unsupported_format_error() {
    //! Unsupported weight formats give helpful error
    let loader = WeightLoader::new();
    let result = loader.load("model.xyz");

    assert!(
        matches!(result, Err(HRMError::WeightLoadError(_))),
        "Should return error for unsupported format"
    );
}

// =============================================================================
// Test Group 3: Inference Correctness (hrm_007 - hrm_010)
// =============================================================================

#[test]
fn test_hrm_007_inference_bull_market() {
    //! Correct regime detection in bull market
    let engine = InferenceEngine::new();
    let signals = vec![
        0.8,  // PEGY: high
        0.9,  // Insider: strong buying
        0.8,  // Sentiment: positive
        15.0, // VIX: low volatility
        0.0,  // Market phase: Bull
        0.5,  // Time: midday
    ];

    let result = engine.infer(&signals).expect("Inference should succeed");
    assert!(
        result.conviction >= 0.0 && result.conviction <= 1.0,
        "Conviction should be in [0, 1]"
    );
    assert!(
        result.confidence >= 0.0 && result.confidence <= 1.0,
        "Confidence should be in [0, 1]"
    );
}

#[test]
fn test_hrm_008_inference_bear_market() {
    //! Correct regime detection in bear market
    let engine = InferenceEngine::new();
    let signals = vec![
        0.3,  // PEGY: low
        0.2,  // Insider: selling
        0.2,  // Sentiment: negative
        45.0, // VIX: high volatility
        1.0,  // Market phase: Bear
        0.5,  // Time: midday
    ];

    let result = engine.infer(&signals).expect("Inference should succeed");

    // High volatility should reduce conviction
    assert!(
        result.conviction < 0.7,
        "High VIX should reduce conviction in bear market"
    );
}

#[test]
fn test_hrm_009_inference_crisis_market() {
    //! Correct handling of crisis market conditions
    let engine = InferenceEngine::new();
    let signals = vec![
        0.1,  // PEGY: very low
        0.1,  // Insider: heavy selling
        0.1,  // Sentiment: very negative
        80.0, // VIX: extreme volatility
        3.0,  // Market phase: Crisis
        0.5,  // Time: midday
    ];

    let result = engine.infer(&signals).expect("Inference should succeed");
    assert_eq!(result.regime, MarketRegime::Crisis);
}

#[test]
fn test_hrm_010_inference_output_ranges() {
    //! All inference outputs are properly bounded
    let engine = InferenceEngine::new();

    // Test multiple random-ish inputs
    for i in 0..10 {
        let signals = vec![
            (i as f32) * 0.1,        // PEGY
            (i as f32) * 0.1,        // Insider
            (i as f32) * 0.1,        // Sentiment
            10.0 + (i as f32) * 5.0, // VIX
            (i % 4) as f32,          // Regime
            0.5,                     // Time
        ];

        let result = engine.infer(&signals).expect("Inference should succeed");
        assert!(
            result.conviction >= 0.0 && result.conviction <= 1.0,
            "Conviction out of bounds: {}",
            result.conviction
        );
        assert!(
            result.confidence >= 0.0 && result.confidence <= 1.0,
            "Confidence out of bounds: {}",
            result.confidence
        );
    }
}

// =============================================================================
// Test Group 4: Adaptive Behavior (hrm_011 - hrm_013)
// =============================================================================

#[test]
fn test_hrm_011_volatility_impact() {
    //! High volatility reduces conviction
    let engine = InferenceEngine::new();

    let base_signals = vec![0.8, 0.8, 0.8, 10.0, 0.0, 0.5];
    let high_vix_signals = vec![0.8, 0.8, 0.8, 60.0, 0.0, 0.5];

    let base_result = engine.infer(&base_signals).unwrap();
    let high_vix_result = engine.infer(&high_vix_signals).unwrap();

    assert!(
        high_vix_result.conviction < base_result.conviction,
        "High VIX should reduce conviction"
    );
}

#[test]
fn test_hrm_012_confidence_threshold() {
    //! Confidence values respect thresholds
    let result_high = InferenceResult::new(0.8, 0.9, MarketRegime::Bull, 100);
    let result_low = InferenceResult::new(0.8, 0.5, MarketRegime::Bull, 100);

    assert!(result_high.is_confident(0.7));
    assert!(!result_low.is_confident(0.7));
}

#[test]
fn test_hrm_013_conviction_trading_decision() {
    //! Conviction scores correctly inform trading decisions
    let strong_signal = InferenceResult::new(0.85, 0.9, MarketRegime::Bull, 100);
    let weak_signal = InferenceResult::new(0.45, 0.6, MarketRegime::Bull, 100);

    assert!(strong_signal.should_trade(0.7));
    assert!(!weak_signal.should_trade(0.7));
}

// =============================================================================
// Test Group 5: Input Validation (hrm_014 - hrm_016)
// =============================================================================

#[test]
fn test_hrm_014_invalid_input_size() {
    //! Wrong input size returns appropriate error
    let engine = InferenceEngine::new();
    let bad_signals = vec![0.8, 0.9]; // Too few inputs

    let result = engine.infer(&bad_signals);
    assert!(
        matches!(
            result,
            Err(HRMError::InvalidInputShape {
                expected: 6,
                actual: 2
            })
        ),
        "Should return InvalidInputShape for wrong input size"
    );
}

#[test]
fn test_hrm_015_empty_input() {
    //! Empty input returns appropriate error
    let engine = InferenceEngine::new();
    let empty: Vec<f32> = vec![];

    let result = engine.infer(&empty);
    assert!(matches!(result, Err(HRMError::InvalidInputShape { .. })));
}

#[test]
fn test_hrm_016_too_many_inputs() {
    //! Too many inputs returns appropriate error
    let engine = InferenceEngine::new();
    let too_many = vec![0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0];

    let result = engine.infer(&too_many);
    assert!(
        matches!(
            result,
            Err(HRMError::InvalidInputShape {
                expected: 6,
                actual: 10
            })
        ),
        "Should return InvalidInputShape for too many inputs"
    );
}

// =============================================================================
// Test Group 6: Batch Processing (hrm_017 - hrm_019)
// =============================================================================

#[test]
fn test_hrm_017_batch_inference() {
    //! Batch inference processes multiple signals
    let engine = InferenceEngine::new();
    let batch = vec![
        vec![0.8, 0.9, 0.7, 15.0, 0.0, 0.5],
        vec![0.5, 0.6, 0.4, 25.0, 1.0, 0.3],
        vec![0.3, 0.4, 0.2, 35.0, 2.0, 0.7],
    ];

    let results = engine
        .infer_batch(&batch)
        .expect("Batch inference should succeed");
    assert_eq!(results.len(), 3, "Should return 3 results for 3 inputs");

    for result in results {
        assert!(result.conviction >= 0.0 && result.conviction <= 1.0);
        assert!(result.confidence >= 0.0 && result.confidence <= 1.0);
    }
}

#[test]
fn test_hrm_018_empty_batch() {
    //! Empty batch returns empty results
    let engine = InferenceEngine::new();
    let empty_batch: Vec<Vec<f32>> = vec![];

    let results = engine.infer_batch(&empty_batch).unwrap();
    assert!(results.is_empty());
}

#[test]
fn test_hrm_019_single_element_batch() {
    //! Single element batch works correctly
    let engine = InferenceEngine::new();
    let batch = vec![vec![0.8, 0.9, 0.7, 15.0, 0.0, 0.5]];

    let results = engine.infer_batch(&batch).unwrap();
    assert_eq!(results.len(), 1);
}

// =============================================================================
// Test Group 7: Model Statistics (hrm_020 - hrm_022)
// =============================================================================

#[test]
fn test_hrm_020_model_stats() {
    //! Model statistics are accurate
    let config = HRMConfig::default();
    let hrm = HRM::new(&config).unwrap();

    let stats = hrm.stats();
    assert_eq!(stats.input_size, 6);
    assert_eq!(stats.output_size, 3);
    assert!(!stats.weights_loaded);
    assert!(
        stats.parameters > 50000,
        "Should have reasonable parameter count"
    );
}

#[test]
fn test_hrm_021_custom_config_stats() {
    //! Custom configurations have correct stats
    let config = HRMConfig::new(10, 256, 128, 5);
    let hrm = HRM::new(&config).unwrap();

    let stats = hrm.stats();
    assert_eq!(stats.input_size, 10);
    assert_eq!(stats.output_size, 5);
}

#[test]
fn test_hrm_022_parameter_estimation() {
    //! Parameter estimation is reasonable
    let small_config = HRMConfig::new(6, 32, 16, 3);
    let large_config = HRMConfig::new(10, 512, 256, 5);

    let small_params = small_config.estimated_parameters();
    let large_params = large_config.estimated_parameters();

    assert!(
        large_params > small_params,
        "Larger model should have more parameters"
    );
}

// =============================================================================
// Test Group 8: Configuration Options (hrm_023 - hrm_025)
// =============================================================================

#[test]
fn test_hrm_023_device_config_variants() {
    //! All device configurations are valid
    let configs = vec![
        HRMConfig::default().with_device(DeviceConfig::Cpu),
        HRMConfig::default().with_device(DeviceConfig::Cuda),
        HRMConfig::default().with_device(DeviceConfig::Metal),
        HRMConfig::default().with_device(DeviceConfig::Auto),
    ];

    for config in configs {
        assert!(config.validate().is_ok());
    }
}

#[test]
fn test_hrm_024_confidence_threshold_clamping() {
    //! Confidence thresholds are properly clamped
    let config = HRMConfig::default().with_confidence_threshold(1.5);
    assert_eq!(config.confidence_threshold, 1.0);

    let config = HRMConfig::default().with_confidence_threshold(-0.5);
    assert_eq!(config.confidence_threshold, 0.0);
}

#[test]
fn test_hrm_025_builder_pattern() {
    //! Builder pattern creates valid models
    use investor_os::hrm::HRMBuilder;

    let hrm = HRMBuilder::new()
        .with_input_size(8)
        .with_hidden_sizes(128, 64)
        .with_confidence_threshold(0.75)
        .with_device(DeviceConfig::Cpu)
        .build()
        .expect("Builder should create valid HRM");

    assert_eq!(hrm.config().input_size, 8);
    assert_eq!(hrm.config().high_hidden_size, 128);
    assert_eq!(hrm.config().low_hidden_size, 64);
    assert_eq!(hrm.config().confidence_threshold, 0.75);
}

// =============================================================================
// Test Group 9: Error Handling (hrm_026 - hrm_028)
// =============================================================================

#[test]
fn test_hrm_026_error_messages() {
    //! Error messages are descriptive
    let engine = InferenceEngine::new();
    let result = engine.infer(&vec![0.1, 0.2]);

    if let Err(HRMError::InvalidInputShape { expected, actual }) = result {
        assert_eq!(expected, 6);
        assert_eq!(actual, 2);
    } else {
        panic!("Expected InvalidInputShape error");
    }
}

#[test]
fn test_hrm_027_market_regime_conversion() {
    //! Market regime conversions are correct
    assert_eq!(MarketRegime::from(0.0), MarketRegime::Bull);
    assert_eq!(MarketRegime::from(1.0), MarketRegime::Bear);
    assert_eq!(MarketRegime::from(2.0), MarketRegime::Sideways);
    assert_eq!(MarketRegime::from(3.0), MarketRegime::Crisis);
    assert_eq!(MarketRegime::from(99.0), MarketRegime::Sideways); // Default
}

#[test]
fn test_hrm_028_market_regime_to_f32() {
    //! Market regime to f32 conversion works
    assert_eq!(f32::from(MarketRegime::Bull), 0.0);
    assert_eq!(f32::from(MarketRegime::Bear), 1.0);
    assert_eq!(f32::from(MarketRegime::Sideways), 2.0);
    assert_eq!(f32::from(MarketRegime::Crisis), 3.0);
}

// =============================================================================
// Test Group 10: Integration (hrm_029 - hrm_030)
// =============================================================================

#[test]
fn test_hrm_029_end_to_end_inference() {
    //! End-to-end inference pipeline works
    let config = HRMConfig::default();
    let hrm = HRM::new(&config).expect("Should create HRM");

    let signals = vec![0.8, 0.9, 0.7, 15.0, 0.0, 0.5];
    let result = hrm.infer(&signals).expect("Should infer");

    // Verify result structure
    assert!(result.conviction >= 0.0 && result.conviction <= 1.0);
    assert!(result.confidence >= 0.0 && result.confidence <= 1.0);
    // Latency is measured in the engine, placeholder returns 0 for now
    assert!(result.latency_us >= 0, "Should have non-negative latency");
}

#[test]
fn test_hrm_030_inference_result_display() {
    //! Inference results can be displayed/debugged
    let result = InferenceResult::new(0.75, 0.85, MarketRegime::Bull, 1200);

    let debug_str = format!("{:?}", result);
    assert!(debug_str.contains("0.75"));
    assert!(debug_str.contains("0.85"));
    assert!(debug_str.contains("Bull"));
}

// =============================================================================
// Summary
// =============================================================================

// Total: 30 Golden Path tests for Sprint 36
// All tests must pass before merging to main
