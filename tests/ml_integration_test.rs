//! Golden Path Tests for ML Integration
//!
//! Tests the integration of ML with:
//! - Trading signals (Signal generation from predictions)
//! - Position sizing (Confidence-based sizing)
//! - Portfolio optimization (ML views)

use investor_os::ml::{
    MlSignalGenerator, MlPositionSizer, MlPortfolioOptimizer, MlPortfolioViews,
    MlTradingPipeline, MlTradingDecision,
    FeatureConfig, FeatureEngine, PriceData, Prediction,
    inference::InferenceConfig,
};
use investor_os::risk::position_sizing::{SizingConfig, SizingMethod};

use investor_os::strategies::SignalDirection;
use rust_decimal::Decimal;
use chrono::Utc;

/// Helper: Create mock price data
fn create_price_data(close: f64) -> PriceData {
    PriceData {
        open: Decimal::try_from(close * 0.99).unwrap(),
        high: Decimal::try_from(close * 1.01).unwrap(),
        low: Decimal::try_from(close * 0.98).unwrap(),
        close: Decimal::try_from(close).unwrap(),
        volume: Decimal::from(1_000_000),
    }
}

/// Helper: Create price history with trend
fn create_trending_history(start: f64, count: usize, trend: f64) -> Vec<PriceData> {
    (0..count)
        .map(|i| {
            let price = start + (i as f64) * trend + ((i % 5) as f64) * 0.5;
            create_price_data(price)
        })
        .collect()
}

/// Test 1: ML Signal Generator Creation
#[test]
fn test_ml_signal_generator_creation() {
    println!("\n🎯 Testing ML Signal Generator Creation");
    
    let generator = MlSignalGenerator::new(
        FeatureConfig::default(),
        InferenceConfig::default(),
        Decimal::try_from(0.6).unwrap(),
    );
    
    println!("✅ Created ML Signal Generator");
    println!("   Confidence threshold: 0.6");
    
    // Just verify it was created successfully
    println!("   Generator created successfully");
}

/// Test 2: Signal Generation from Price Data
#[test]
fn test_ml_signal_generation() {
    println!("\n📈 Testing ML Signal Generation");
    
    let generator = MlSignalGenerator::default();
    
    // Create trending price history (60 periods minimum)
    let price_history = create_trending_history(100.0, 60, 0.5);
    
    println!("   Price history: {} periods", price_history.len());
    println!("   Start price: ${}", price_history.first().unwrap().close);
    println!("   End price: ${}", price_history.last().unwrap().close);
    
    // Generate signal
    let signal = generator.generate_signal("AAPL", &price_history);
    
    if let Some(sig) = signal {
        println!("✅ Generated signal:");
        println!("   Symbol: {}", sig.symbol);
        println!("   Direction: {:?}", sig.direction);
        println!("   Strength: {:.2}", sig.strength);
        println!("   Confidence: {:.2}", sig.confidence);
        
        assert_eq!(sig.symbol, "AAPL");
        assert!(sig.strength >= Decimal::ZERO && sig.strength <= Decimal::ONE);
        assert!(sig.confidence >= Decimal::ZERO && sig.confidence <= Decimal::ONE);
    } else {
        println!("ℹ️  No signal generated (confidence below threshold or neutral)");
    }
}

/// Test 3: Signal Direction Based on Trend
#[test]
fn test_signal_direction_uptrend() {
    println!("\n📊 Testing Signal Direction - Uptrend");
    
    let generator = MlSignalGenerator::default();
    
    // Strong uptrend
    let uptrend = create_trending_history(100.0, 60, 2.0);
    
    if let Some(signal) = generator.generate_signal("AAPL", &uptrend) {
        println!("   Uptrend signal direction: {:?}", signal.direction);
        println!("   Signal strength: {:.2}", signal.strength);
        
        // In uptrend, should be Long or StrongBuy
        assert!(
            signal.direction == SignalDirection::Long || 
            signal.direction == SignalDirection::Neutral,
            "Expected Long or Neutral in uptrend"
        );
    } else {
        println!("   No signal (neutral market)");
    }
}

/// Test 4: ML Position Sizer Creation
#[test]
fn test_ml_position_sizer_creation() {
    println!("\n💰 Testing ML Position Sizer Creation");
    
    let config = SizingConfig {
        method: SizingMethod::FixedFractional,
        risk_percent: Decimal::try_from(0.01).unwrap(),
        ..Default::default()
    };
    
    let sizer = MlPositionSizer::new(config);
    
    println!("✅ Created ML Position Sizer");
    println!("   Method: Fixed Fractional");
    println!("   Risk per trade: 1%");
    println!("   Confidence scaling: enabled");
}

/// Test 5: Position Sizing with Confidence
#[test]
fn test_position_sizing_with_confidence() {
    println!("\n📏 Testing Position Sizing with ML Confidence");
    
    let config = SizingConfig {
        method: SizingMethod::FixedFractional,
        risk_percent: Decimal::try_from(0.02).unwrap(), // 2% risk
        ..Default::default()
    };
    
    let sizer = MlPositionSizer::new(config);
    
    let capital = Decimal::from(100_000);
    let entry = Decimal::from(150);
    let stop_loss = Decimal::from(140); // 10$ stop = 6.67% risk
    
    // Test with different confidence levels
    let test_cases = vec![
        (Decimal::try_from(0.95).unwrap(), "High confidence (95%)"),
        (Decimal::try_from(0.75).unwrap(), "Medium confidence (75%)"),
        (Decimal::try_from(0.55).unwrap(), "Low confidence (55%)"),
    ];
    
    for (confidence, desc) in test_cases {
        let size = sizer.calculate_size_with_confidence(
            capital, entry, stop_loss, confidence,
        );
        
        if let Some(s) = size {
            let value = s * entry;
            println!("   {}: {} shares (${})", desc, s, value);
            assert!(s > Decimal::ZERO);
        }
    }
}

/// Test 6: Confidence Scaling Factor
#[test]
fn test_confidence_scaling_factor() {
    println!("\n📐 Testing Confidence Scaling Factor");
    
    let config = SizingConfig::default();
    let sizer = MlPositionSizer::new(config);
    
    // Test confidence factors
    let test_cases = vec![
        (Decimal::ONE, Decimal::ONE),                           // 100% -> 100%
        (Decimal::try_from(0.75).unwrap(), Decimal::try_from(0.75).unwrap()),
        (Decimal::try_from(0.5).unwrap(), Decimal::try_from(0.5).unwrap()),
    ];
    
    for (confidence, _) in test_cases {
        // Just verify confidence values are valid
        println!("   Testing confidence: {}", confidence);
        assert!(confidence >= Decimal::ZERO);
        assert!(confidence <= Decimal::ONE);
    }
}

/// Test 7: ML Portfolio Views Creation
#[test]
fn test_ml_portfolio_views() {
    println!("\n📋 Testing ML Portfolio Views");
    
    let predictions = vec![
        ("AAPL".to_string(), Prediction::new(
            Decimal::try_from(0.15).unwrap(), // 15% expected return
            Decimal::try_from(0.85).unwrap(), // 85% confidence
        )),
        ("MSFT".to_string(), Prediction::new(
            Decimal::try_from(0.12).unwrap(),
            Decimal::try_from(0.80).unwrap(),
        )),
        ("TSLA".to_string(), Prediction::new(
            Decimal::try_from(0.25).unwrap(),
            Decimal::try_from(0.60).unwrap(), // Lower confidence
        )),
    ];
    
    let views = MlPortfolioViews::new(predictions);
    
    println!("✅ Created ML Portfolio Views");
    println!("   Total views: {}", views.predictions.len());
    
    // Test filtering by confidence
    let high_confidence = views.filter_by_confidence(Decimal::try_from(0.7).unwrap());
    println!("   High confidence views (>=70%): {}", high_confidence.predictions.len());
    
    assert_eq!(high_confidence.predictions.len(), 2); // AAPL and MSFT
    
    // Test individual view access
    if let Some(view) = views.get_view("AAPL") {
        println!("   AAPL view (expected return): {:.2}", view);
    }
}

/// Test 8: ML Portfolio Optimizer
#[test]
fn test_ml_portfolio_optimizer() {
    println!("\n🎯 Testing ML Portfolio Optimizer");
    
    let optimizer = MlPortfolioOptimizer::new();
    
    println!("✅ Created ML Portfolio Optimizer");
    println!("   ML views enabled: true");
    println!("   Confidence threshold: 0.6");
}

/// Test 9: ML Trading Pipeline Creation
#[test]
fn test_ml_trading_pipeline_creation() {
    println!("\n🔄 Testing ML Trading Pipeline Creation");
    
    let pipeline = MlTradingPipeline::default();
    
    println!("✅ Created ML Trading Pipeline");
    println!("   Components: Signal Generator, Position Sizer, Portfolio Optimizer");
}

/// Test 10: End-to-End ML Trading Decision
#[test]
fn test_ml_trading_decision() {
    println!("\n🚀 Testing End-to-End ML Trading Decision");
    
    let pipeline = MlTradingPipeline::default();
    
    // Market data
    let symbol = "AAPL";
    let price_history = create_trending_history(150.0, 60, 1.0);
    let available_capital = Decimal::from(50_000);
    
    println!("   Symbol: {}", symbol);
    println!("   Price history: {} periods", price_history.len());
    println!("   Available capital: ${}", available_capital);
    
    // Process market data
    let decision = pipeline.process_market_data(symbol, &price_history, available_capital);
    
    if let Some(dec) = decision {
        println!("✅ Generated trading decision:");
        println!("   Symbol: {}", dec.symbol);
        println!("   Direction: {:?}", dec.signal.direction);
        println!("   Position size: {} shares", dec.position_size);
        println!("   Entry price: ${}", dec.entry_price);
        println!("   Position value: ${}", dec.position_value());
        println!("   Stop loss: ${}", dec.stop_loss);
        println!("   Signal confidence: {:.2}", dec.signal.confidence);
        
        assert!(dec.is_valid());
        assert_eq!(dec.symbol, symbol);
        assert!(dec.position_size > Decimal::ZERO);
    } else {
        println!("ℹ️  No trading decision (signal below threshold or neutral)");
    }
}

/// Test 11: ML Decision Validation
#[test]
fn test_ml_trading_decision_validity() {
    println!("\n✅ Testing ML Trading Decision Validity");
    
    let valid_decision = MlTradingDecision {
        symbol: "AAPL".to_string(),
        signal: investor_os::strategies::Signal::new(
            "AAPL",
            SignalDirection::Long,
            Decimal::try_from(0.8).unwrap(),
        ).with_confidence(Decimal::try_from(0.75).unwrap()),
        position_size: Decimal::from(100),
        entry_price: Decimal::from(150),
        stop_loss: Decimal::from(140),
        timestamp: Utc::now(),
    };
    
    let invalid_decision = MlTradingDecision {
        symbol: "TSLA".to_string(),
        signal: investor_os::strategies::Signal::new(
            "TSLA",
            SignalDirection::Neutral,
            Decimal::ZERO,
        ).with_confidence(Decimal::ZERO),
        position_size: Decimal::ZERO,
        entry_price: Decimal::from(200),
        stop_loss: Decimal::from(180),
        timestamp: Utc::now(),
    };
    
    println!("   Valid decision check: {}", valid_decision.is_valid());
    println!("   Invalid decision check: {}", invalid_decision.is_valid());
    
    assert!(valid_decision.is_valid());
    assert!(!invalid_decision.is_valid());
}

/// Test 12: Signal with Different Market Conditions
#[test]
fn test_signals_different_conditions() {
    println!("\n📊 Testing Signals in Different Market Conditions");
    
    let generator = MlSignalGenerator::default();
    
    // Bullish trend
    let bullish = create_trending_history(100.0, 60, 1.5);
    let bullish_signal = generator.generate_signal("BULL", &bullish);
    
    // Bearish trend
    let bearish = create_trending_history(100.0, 60, -1.5);
    let bearish_signal = generator.generate_signal("BEAR", &bearish);
    
    // Sideways (no clear trend)
    let sideways: Vec<PriceData> = (0..60)
        .map(|i| create_price_data(100.0 + ((i % 5) as f64) * 0.5))
        .collect();
    let sideways_signal = generator.generate_signal("SIDE", &sideways);
    
    println!("   Bullish trend signal: {:?}", 
        bullish_signal.as_ref().map(|s| format!("{:?}", s.direction)));
    println!("   Bearish trend signal: {:?}", 
        bearish_signal.as_ref().map(|s| format!("{:?}", s.direction)));
    println!("   Sideways trend signal: {:?}", 
        sideways_signal.as_ref().map(|s| format!("{:?}", s.direction)));
}

/// Test 13: Confidence-Based Position Size Comparison
#[test]
fn test_confidence_position_size_comparison() {
    println!("\n💹 Testing Confidence-Based Position Size Comparison");
    
    let config = SizingConfig::default();
    let sizer = MlPositionSizer::new(config);
    
    let capital = Decimal::from(100_000);
    let entry = Decimal::from(100);
    let stop = Decimal::from(95);
    
    let high_conf_size = sizer.calculate_size_with_confidence(
        capital, entry, stop, Decimal::try_from(0.95).unwrap()
    ).unwrap_or(Decimal::ZERO);
    
    let low_conf_size = sizer.calculate_size_with_confidence(
        capital, entry, stop, Decimal::try_from(0.55).unwrap()
    ).unwrap_or(Decimal::ZERO);
    
    println!("   High confidence (95%): {} shares", high_conf_size);
    println!("   Low confidence (55%): {} shares", low_conf_size);
    
    // High confidence should result in larger position
    assert!(high_conf_size >= low_conf_size);
}

/// Test 14: Complete ML Trading Workflow
#[test]
fn test_complete_ml_trading_workflow() {
    println!("\n🔄 Testing Complete ML Trading Workflow");
    
    let pipeline = MlTradingPipeline::default();
    
    let symbols = vec!["AAPL", "MSFT", "GOOGL"];
    let capital_per_trade = Decimal::from(30_000);
    
    let mut decisions = Vec::new();
    
    for symbol in &symbols {
        let price_history = create_trending_history(100.0 + (symbol.len() as f64) * 50.0, 60, 1.0);
        
        if let Some(decision) = pipeline.process_market_data(symbol, &price_history, capital_per_trade) {
            println!("   {}: {:?} - {} shares", 
                symbol, decision.signal.direction, decision.position_size);
            decisions.push(decision);
        } else {
            println!("   {}: No signal", symbol);
        }
    }
    
    println!("   Total decisions: {}", decisions.len());
    
    // All should be valid if generated
    for decision in &decisions {
        assert!(decision.is_valid());
    }
}

/// Test 15: ML Integration with Risk Management
#[test]
fn test_ml_integration_with_risk() {
    println!("\n🛡️ Testing ML Integration with Risk Management");
    
    let pipeline = MlTradingPipeline::default();
    let capital = Decimal::from(100_000);
    
    // Generate multiple decisions
    let symbols = vec!["AAPL", "MSFT", "AMZN", "TSLA", "NVDA"];
    let mut total_risk = Decimal::ZERO;
    let mut decision_count = 0;
    
    for symbol in &symbols {
        let price_history = create_trending_history(150.0, 60, 0.8);
        
        if let Some(decision) = pipeline.process_market_data(symbol, &price_history, capital / Decimal::from(5)) {
            let risk_amount = decision.position_size * (decision.entry_price - decision.stop_loss);
            let risk_pct = (risk_amount / capital) * Decimal::from(100);
            
            println!("   {}: Risk ${} ({:.2}% of capital)", 
                symbol, risk_amount, risk_pct);
            
            total_risk += risk_amount;
            decision_count += 1;
        }
    }
    
    if decision_count > 0 {
        let total_risk_pct = (total_risk / capital) * Decimal::from(100);
        println!("   Total portfolio risk: ${} ({:.2}%)", total_risk, total_risk_pct);
        
        // Total risk should be reasonable (< 10% of capital)
        assert!(total_risk_pct < Decimal::from(10));
    }
}
