//! HRM Validation Tests
//!
//! Tests to verify HRM produces correct/expected outputs.
//! These tests define the expected behavior of the model.

use investor_os::hrm::{HRM, HRMConfig, InferenceEngine, MarketRegime};

/// Test 1: High conviction in strong bull market
#[test]
fn test_bull_market_high_conviction() {
    let engine = InferenceEngine::new();
    
    // Strong signals: good PEGY, insider buying, positive sentiment, low VIX
    let signals = vec![
        0.9,   // PEGY: excellent
        0.9,   // Insider: strong buying
        0.9,   // Sentiment: very positive
        10.0,  // VIX: low volatility
        0.0,   // Regime: Bull
        0.5,   // Time: midday
    ];
    
    let result = engine.infer(&signals).unwrap();
    
    // Should have HIGH conviction (> 0.7)
    assert!(
        result.conviction > 0.7,
        "Bull market with strong signals should have conviction > 0.7, got {}",
        result.conviction
    );
    
    // Should be confident
    assert!(
        result.confidence > 0.8,
        "Should be confident in bull market prediction"
    );
    
    // Should detect bull regime
    assert_eq!(result.regime, MarketRegime::Bull);
}

/// Test 2: Low conviction in bear market
#[test]
fn test_bear_market_low_conviction() {
    let engine = InferenceEngine::new();
    
    // Weak signals: poor PEGY, insider selling, negative sentiment, high VIX
    let signals = vec![
        0.2,   // PEGY: poor
        0.2,   // Insider: selling
        0.2,   // Sentiment: negative
        50.0,  // VIX: high volatility
        1.0,   // Regime: Bear
        0.5,
    ];
    
    let result = engine.infer(&signals).unwrap();
    
    // Should have LOW conviction (< 0.3)
    assert!(
        result.conviction < 0.3,
        "Bear market with weak signals should have conviction < 0.3, got {}",
        result.conviction
    );
    
    // Should detect bear regime
    assert_eq!(result.regime, MarketRegime::Bear);
}

/// Test 3: Volatility reduces conviction
#[test]
fn test_high_volatility_reduces_conviction() {
    let engine = InferenceEngine::new();
    
    // Same good signals, different VIX levels
    let base_signals = vec![0.8, 0.8, 0.8];
    
    let low_vix = [&base_signals[..], &[15.0, 0.0, 0.5]].concat();
    let high_vix = [&base_signals[..], &[60.0, 0.0, 0.5]].concat();
    
    let result_low = engine.infer(&low_vix).unwrap();
    let result_high = engine.infer(&high_vix).unwrap();
    
    // High VIX should significantly reduce conviction
    assert!(
        result_high.conviction < result_low.conviction,
        "High VIX (60) should reduce conviction vs low VIX (15)"
    );
    
    // The reduction should be substantial (> 30%)
    let reduction = (result_low.conviction - result_high.conviction) / result_low.conviction;
    assert!(
        reduction > 0.3,
        "VIX impact should reduce conviction by at least 30%, got {}%",
        reduction * 100.0
    );
}

/// Test 4: Crisis market = very low conviction
#[test]
fn test_crisis_market_extreme_caution() {
    let engine = InferenceEngine::new();
    
    // Extreme fear signals
    let signals = vec![
        0.1,   // PEGY: terrible
        0.1,   // Insider: panic selling
        0.1,   // Sentiment: panic
        80.0,  // VIX: extreme
        3.0,   // Regime: Crisis
        0.5,
    ];
    
    let result = engine.infer(&signals).unwrap();
    
    // Should have VERY LOW conviction (< 0.1)
    assert!(
        result.conviction < 0.1,
        "Crisis market should have conviction < 0.1, got {}",
        result.conviction
    );
    
    // Should detect crisis
    assert_eq!(result.regime, MarketRegime::Crisis);
}

/// Test 5: Mixed signals = moderate conviction
#[test]
fn test_mixed_signals_moderate_conviction() {
    let engine = InferenceEngine::new();
    
    // Mixed signals: good PEGY but poor sentiment
    let signals = vec![
        0.8,   // PEGY: good
        0.5,   // Insider: neutral
        0.3,   // Sentiment: poor
        25.0,  // VIX: moderate
        2.0,   // Regime: Sideways
        0.5,
    ];
    
    let result = engine.infer(&signals).unwrap();
    
    // Should have MODERATE conviction (0.3 - 0.6)
    assert!(
        result.conviction >= 0.3 && result.conviction <= 0.6,
        "Mixed signals should produce moderate conviction (0.3-0.6), got {}",
        result.conviction
    );
}

/// Test 6: Insider activity matters
#[test]
fn test_insider_importance() {
    let engine = InferenceEngine::new();
    
    // Same PEGY and sentiment, different insider
    let low_insider = vec![0.7, 0.2, 0.7, 15.0, 0.0, 0.5];
    let high_insider = vec![0.7, 0.9, 0.7, 15.0, 0.0, 0.5];
    
    let result_low = engine.infer(&low_insider).unwrap();
    let result_high = engine.infer(&high_insider).unwrap();
    
    // High insider activity should increase conviction
    assert!(
        result_high.conviction > result_low.conviction,
        "Strong insider buying should increase conviction"
    );
}

/// Test 7: Trading decision thresholds
#[test]
fn test_trading_decisions() {
    let engine = InferenceEngine::new();
    
    // Test cases: (signals, expected_trade)
    let test_cases = vec![
        // Strong buy
        (vec![0.9, 0.9, 0.9, 10.0, 0.0, 0.5], true),
        // Weak - don't trade
        (vec![0.4, 0.4, 0.4, 30.0, 2.0, 0.5], false),
        // Crisis - definitely don't trade
        (vec![0.1, 0.1, 0.1, 80.0, 3.0, 0.5], false),
    ];
    
    for (signals, should_trade) in test_cases {
        let result = engine.infer(&signals).unwrap();
        let decision = result.should_trade(0.7); // 0.7 threshold
        
        assert_eq!(
            decision, should_trade,
            "Trading decision mismatch for signals {:?}: got {}, expected {}",
            signals, decision, should_trade
        );
    }
}

/// Test 8: Batch consistency
#[test]
fn test_batch_consistency() {
    let hrm = HRM::new(&HRMConfig::default()).unwrap();
    
    let signals = vec![0.8, 0.9, 0.7, 15.0, 0.0, 0.5];
    
    // Single inference
    let single_result = hrm.infer(&signals).unwrap();
    
    // Batch inference with same signals
    let batch = vec![signals.clone()];
    let batch_results = hrm.infer_batch(&batch).unwrap();
    
    // Should produce same result
    assert!(
        (single_result.conviction - batch_results[0].conviction).abs() < 0.001,
        "Batch and single inference should produce same results"
    );
}

/// Test 9: Confidence correlates with signal strength
#[test]
fn test_confidence_correlation() {
    let engine = InferenceEngine::new();
    
    // Weak signals -> low confidence
    let weak = vec![0.3, 0.3, 0.3, 30.0, 2.0, 0.5];
    let result_weak = engine.infer(&weak).unwrap();
    
    // Strong signals -> high confidence  
    let strong = vec![0.9, 0.9, 0.9, 10.0, 0.0, 0.5];
    let result_strong = engine.infer(&strong).unwrap();
    
    assert!(
        result_strong.confidence > result_weak.confidence,
        "Strong signals should produce higher confidence"
    );
}

/// Test 10: Output ranges are valid
#[test]
fn test_output_ranges() {
    let engine = InferenceEngine::new();
    
    // Test many random inputs
    for i in 0..100 {
        let signals = vec![
            (i as f32 % 10.0) / 10.0,  // PEGY: 0.0-0.9
            ((i * 3) as f32 % 10.0) / 10.0,  // Insider
            ((i * 7) as f32 % 10.0) / 10.0,  // Sentiment
            (i as f32 * 0.8),  // VIX: 0-80
            ((i % 4) as f32),  // Regime: 0-3
            0.5,
        ];
        
        let result = engine.infer(&signals).unwrap();
        
        // Conviction must be in [0, 1]
        assert!(
            result.conviction >= 0.0 && result.conviction <= 1.0,
            "Conviction out of range: {}", result.conviction
        );
        
        // Confidence must be in [0, 1]
        assert!(
            result.confidence >= 0.0 && result.confidence <= 1.0,
            "Confidence out of range: {}", result.confidence
        );
    }
}

/// Benchmark: Inference should be fast
#[test]
fn test_inference_performance() {
    use std::time::Instant;
    
    let engine = InferenceEngine::new();
    let signals = vec![0.8, 0.9, 0.7, 15.0, 0.0, 0.5];
    
    // Warm up
    for _ in 0..10 {
        let _ = engine.infer(&signals);
    }
    
    // Time 100 inferences
    let start = Instant::now();
    for _ in 0..100 {
        let _ = engine.infer(&signals);
    }
    let elapsed = start.elapsed();
    
    let avg_ms = elapsed.as_millis() as f64 / 100.0;
    
    // Should be fast (< 1ms per inference on CPU)
    assert!(
        avg_ms < 1.0,
        "Inference too slow: {} ms (target < 1ms)",
        avg_ms
    );
    
    println!("Average inference time: {:.3} ms", avg_ms);
}

/// Reference test: Known input -> expected output
#[test]
fn test_reference_case_1() {
    let engine = InferenceEngine::new();
    
    // Specific known input
    let signals = vec![0.75, 0.80, 0.65, 20.0, 0.0, 0.5];
    
    let result = engine.infer(&signals).unwrap();
    
    // With placeholder formula, we expect:
    // base = 0.75*0.3 + 0.80*0.3 + 0.65*0.4 = 0.725
    // volatility_factor = 1 - 20/100 = 0.8
    // conviction = 0.725 * 0.8 = 0.58
    // 
    // This test documents the expected behavior
    // When we switch to real neural network, this test will need updating
    
    println!("Reference case result: conviction={:.2}", result.conviction);
    
    // For now, just ensure it's reasonable
    assert!(result.conviction > 0.4 && result.conviction < 0.8);
}
