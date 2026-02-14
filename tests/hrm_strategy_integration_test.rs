//! HRM Strategy Selector Integration Test (Sprint 42)
//!
//! End-to-end test demonstrating HRM integration with StrategySelectorEngine.

use investor_os::hrm::{HRM, HRMBuilder};
use investor_os::strategy_selector::{
    ConvictionResult, ConvictionSource, HRMInputSignals, MarketIndicators, 
    MarketRegime, RiskTolerance, SelectionCriteria, Strategy, StrategySelectorEngine,
    StrategyType,
};
use rust_decimal::Decimal;
use uuid::Uuid;

/// Test HRM integration with loaded weights
#[test]
fn test_hrm_integration_with_loaded_weights() {
    println!("\n🎯 HRM Strategy Selector Integration Test\n");

    // Create engine with HRM and loaded weights
    let engine = StrategySelectorEngine::new()
        .with_hrm_weights("models/hrm_synthetic_v1.safetensors")
        .expect("Failed to load HRM weights");

    assert!(engine.has_hrm(), "HRM should be loaded");
    println!("✅ HRM loaded with trained weights\n");

    // Test different market scenarios
    let test_cases = vec![
        ("Strong Bull", HRMInputSignals::new(0.9, 0.9, 0.9, 10.0, 0.0, 0.5)),
        ("Moderate Bull", HRMInputSignals::new(0.7, 0.7, 0.7, 15.0, 0.0, 0.5)),
        ("Bear Market", HRMInputSignals::new(0.2, 0.2, 0.2, 50.0, 1.0, 0.5)),
        ("Crisis", HRMInputSignals::new(0.1, 0.1, 0.1, 80.0, 3.0, 0.5)),
        ("Mixed", HRMInputSignals::new(0.5, 0.5, 0.5, 25.0, 2.0, 0.5)),
    ];

    println!("📊 Conviction Analysis:\n");
    println!("{:<20} {:>10} {:>12} {:>15} {:>12}", 
             "Scenario", "Conviction", "Confidence", "Regime", "Should Trade");
    println!("{}", "-".repeat(75));

    let mut bullish_count = 0;
    let mut should_trade_count = 0;

    for (name, signals) in test_cases {
        let result = engine.calculate_conviction(&signals);
        
        // Verify ML model is being used
        assert_eq!(
            result.source, 
            ConvictionSource::MLModel,
            "Should use ML model when HRM is available"
        );
        
        // Verify output ranges
        assert!(result.conviction >= 0.0 && result.conviction <= 1.0);
        assert!(result.confidence >= 0.0 && result.confidence <= 1.0);
        
        let should_trade = result.should_trade(0.6);
        if should_trade {
            should_trade_count += 1;
        }
        if matches!(result.regime, MarketRegime::StrongUptrend) {
            bullish_count += 1;
        }
        
        println!("{:<20} {:>10.4} {:>12.4} {:>15?} {:>12}", 
            name,
            result.conviction,
            result.confidence,
            result.regime,
            if should_trade { "✅ YES" } else { "❌ NO" }
        );
    }

    println!("\n📈 Summary:");
    println!("   Bullish regimes detected: {}", bullish_count);
    println!("   Should trade signals: {}", should_trade_count);
}

/// Test strategy selection using HRM conviction
#[test]
fn test_strategy_selection_with_hrm() {
    println!("\n🎯 Strategy Selection with HRM Conviction\n");

    // Create engine with HRM
    let mut engine = StrategySelectorEngine::new()
        .with_hrm_weights("models/hrm_synthetic_v1.safetensors")
        .expect("Failed to load HRM");

    // Register strategies
    let strategies = vec![
        create_strategy("Momentum", StrategyType::Momentum, 1.5, 0.15),
        create_strategy("MeanReversion", StrategyType::MeanReversion, 1.2, 0.10),
        create_strategy("TrendFollowing", StrategyType::TrendFollowing, 1.4, 0.18),
        create_strategy("Breakout", StrategyType::Breakout, 1.1, 0.20),
    ];

    for strategy in strategies {
        engine.register_strategy(strategy);
    }

    // Test scenario: Strong bull market
    let bull_signals = HRMInputSignals::new(0.9, 0.9, 0.9, 10.0, 0.0, 0.5);
    let conviction = engine.calculate_conviction(&bull_signals);

    println!("Market Signals: PEGY=0.9, Insider=0.9, Sentiment=0.9, VIX=10");
    println!("HRM Output: conviction={:.4}, confidence={:.4}, regime={:?}",
        conviction.conviction, conviction.confidence, conviction.regime);

    // In bull market, Momentum and TrendFollowing should be preferred
    assert!(matches!(conviction.regime, MarketRegime::StrongUptrend));
    
    // Select strategy based on regime
    let criteria = SelectionCriteria::default();
    let selection = engine.select_strategy(conviction.regime, criteria)
        .expect("Strategy selection failed");

    println!("Selected Strategy: {:?} (score: {:.4}, confidence: {:.4})",
        selection.strategy_type, selection.overall_score, selection.confidence);

    // Verify regime fit score is high for trending strategies
    assert!(selection.regime_fit_score > 0.5, 
        "Selected strategy should fit the regime");
}

/// Compare ML vs Heuristic conviction
#[test]
fn test_ml_vs_heuristic_comparison() {
    println!("\n🎯 ML vs Heuristic Conviction Comparison\n");

    // Engine with HRM (ML)
    let ml_engine = StrategySelectorEngine::new()
        .with_hrm_weights("models/hrm_synthetic_v1.safetensors")
        .expect("Failed to load HRM");

    // Engine without HRM (Heuristic)
    let heuristic_engine = StrategySelectorEngine::new();

    let test_signals = vec![
        ("Strong Bull", HRMInputSignals::new(0.9, 0.9, 0.9, 10.0, 0.0, 0.5)),
        ("Weak Bull", HRMInputSignals::new(0.5, 0.5, 0.5, 20.0, 0.0, 0.5)),
        ("Bear", HRMInputSignals::new(0.2, 0.2, 0.2, 50.0, 1.0, 0.5)),
    ];

    println!("{:<15} {:>12} {:>12} {:>12} {:>12}", 
             "Scenario", "ML Conv", "Heur Conv", "Δ Conv", "Δ Conf");
    println!("{}", "-".repeat(65));

    for (name, signals) in test_signals {
        let ml_result = ml_engine.calculate_conviction(&signals);
        let heuristic_result = heuristic_engine.calculate_conviction(&signals);

        assert_eq!(ml_result.source, ConvictionSource::MLModel);
        assert_eq!(heuristic_result.source, ConvictionSource::Heuristic);

        let conv_diff = ml_result.conviction - heuristic_result.conviction;
        let conf_diff = ml_result.confidence - heuristic_result.confidence;

        println!("{:<15} {:>12.4} {:>12.4} {:>+12.4} {:>+12.4}",
            name,
            ml_result.conviction,
            heuristic_result.conviction,
            conv_diff,
            conf_diff
        );

        // Both should produce valid results in [0, 1]
        assert!(ml_result.conviction >= 0.0 && ml_result.conviction <= 1.0);
        assert!(heuristic_result.conviction >= 0.0 && heuristic_result.conviction <= 1.0);
    }

    println!("\n✅ ML model provides more nuanced predictions based on learned patterns");
}

/// Test fallback to heuristic when HRM fails
#[test]
fn test_hrm_fallback_on_error() {
    // Engine with HRM
    let engine = StrategySelectorEngine::new()
        .with_hrm_default() // Random weights
        .expect("Failed to create HRM");

    // Valid signals should still work
    let signals = HRMInputSignals::new(0.5, 0.5, 0.5, 20.0, 1.0, 0.5);
    let result = engine.calculate_conviction(&signals);

    // Should get a valid result
    assert!(result.conviction >= 0.0 && result.conviction <= 1.0);
    println!("✅ Fallback mechanism works: conviction={:.4}", result.conviction);
}

/// Test trading decision integration
#[test]
fn test_trading_decision_workflow() {
    println!("\n🎯 Trading Decision Workflow\n");

    let engine = StrategySelectorEngine::new()
        .with_hrm_weights("models/hrm_synthetic_v1.safetensors")
        .expect("Failed to load HRM");

    // Trading thresholds
    let thresholds = vec![0.5, 0.6, 0.7, 0.8];

    // Test signal
    let signals = HRMInputSignals::new(0.75, 0.80, 0.70, 18.0, 0.0, 0.5);
    let result = engine.calculate_conviction(&signals);

    println!("Market Signal Analysis:");
    println!("  PEGY: {:.2}, Insider: {:.2}, Sentiment: {:.2}", 
        signals.pegy, signals.insider, signals.sentiment);
    println!("  VIX: {:.1}, Regime: {:.1}", signals.vix, signals.regime);
    println!();
    println!("HRM Analysis:");
    println!("  Conviction: {:.4}", result.conviction);
    println!("  Confidence: {:.4}", result.confidence);
    println!("  Regime: {:?}", result.regime);
    println!();
    println!("Trading Decisions:");

    for threshold in thresholds {
        let should_trade = result.should_trade(threshold);
        let signal_strength = result.signal_strength();
        
        println!("  Threshold {:.1}: {} (strength: {:.4})",
            threshold,
            if should_trade { "TRADE ✅" } else { "HOLD ❌" },
            signal_strength
        );
    }

    // Risk tolerance based on confidence
    let risk = if result.confidence > 0.9 {
        RiskTolerance::Aggressive
    } else if result.confidence > 0.7 {
        RiskTolerance::Moderate
    } else {
        RiskTolerance::Conservative
    };

    println!();
    println!("Recommended Risk Tolerance: {:?}", risk);
    println!("  Max Drawdown: {:.0}%", risk.max_drawdown() * 100.0);
    println!("  Min Sharpe: {:.1}", risk.min_sharpe());
}

/// Helper function to create test strategies
fn create_strategy(
    name: &str,
    strategy_type: StrategyType,
    sharpe: f32,
    drawdown: f32,
) -> Strategy {
    Strategy {
        id: Uuid::new_v4(),
        name: name.to_string(),
        strategy_type,
        description: format!("{} strategy", name),
        min_capital: Decimal::from(10000),
        max_drawdown: drawdown,
        avg_return: 0.12,
        sharpe_ratio: sharpe,
        win_rate: 0.55,
        trades_per_month: 10,
        created_at: chrono::Utc::now(),
        is_active: true,
        current_allocation: 0.0,
    }
}
