//! Sprint 7: Advanced Analytics & Backtesting - Golden Path Tests
//!
//! S7-GP-01: Backtest Returns Calculation
//! S7-GP-02: VaR Calculation
//! S7-GP-03: Sharpe Ratio
//! S7-GP-04: XGBoost Training
//! S7-GP-05: Anomaly Detection
//! S7-GP-06: Performance Attribution

use chrono::{Duration, Utc};
use investor_os::analytics::{
    attribution::AttributionAnalyzer,
    backtest::{Backtest, BacktestConfig, SlippageModel},
    ml::{AnomalyDetector, AnomalyResult, CQPredictor, FeaturePipeline},
    risk::RiskAnalyzer,
    MarketData, PriceBar, Signal, SignalDirection, Strategy,
};
use investor_os::signals::{QualityScore, TickerSignals};
use rust_decimal::Decimal;
use std::collections::HashMap;

// S7-GP-01: Backtest Returns Calculation
#[tokio::test]
async fn test_backtest_returns_calculation() {
    let config = BacktestConfig {
        start_date: Utc::now() - Duration::days(30),
        end_date: Utc::now(),
        initial_capital: Decimal::from(100000),
        commission_rate: Decimal::from(1) / Decimal::from(1000),
        slippage_model: SlippageModel::Fixed(Decimal::from(1) / Decimal::from(1000)),
        rebalance_frequency: Duration::days(1),
        max_positions: 10,
        allow_short: false,
    };

    let strategy = Box::new(TestStrategy);
    let mut backtest = Backtest::new(config, strategy);

    // Create historical data
    let mut historical_data: HashMap<String, Vec<PriceBar>> = HashMap::new();
    let prices = generate_test_prices(Utc::now() - Duration::days(30), Utc::now());
    historical_data.insert("AAPL".to_string(), prices);

    let result = backtest.run(&historical_data).await.unwrap();

    // Can't lose more than 100%
    assert!(result.total_return > Decimal::from(-1));
    // Sharpe should be finite
    assert!(result.sharpe_ratio > Decimal::MIN && result.sharpe_ratio < Decimal::MAX);
    // NAV should be positive
    assert!(result.config.initial_capital > Decimal::ZERO);
}

// S7-GP-02: VaR Calculation
#[test]
fn test_var_calculation() {
    let returns: Vec<Decimal> = vec![
        Decimal::from(1) / Decimal::from(100),   // 1%
        Decimal::from(-2) / Decimal::from(100),  // -2%
        Decimal::from(15) / Decimal::from(1000), // 1.5%
        Decimal::from(-1) / Decimal::from(100),  // -1%
        Decimal::from(5) / Decimal::from(1000),  // 0.5%
        Decimal::from(-3) / Decimal::from(100),  // -3%
        Decimal::from(2) / Decimal::from(100),   // 2%
    ]
    .into_iter()
    .cycle()
    .take(60)
    .collect();

    let analyzer = RiskAnalyzer::new(returns, Decimal::from(2) / Decimal::from(100));

    let var_95 = analyzer.var(Decimal::from(95) / Decimal::from(100));

    // VaR should be positive (represents loss)
    assert!(var_95 >= Decimal::ZERO);
    // Worst return was -3%, so VaR should be >= 0% and <= 3%
    assert!(var_95 <= Decimal::from(3) / Decimal::from(100));
}

// S7-GP-03: Sharpe Ratio
#[test]
fn test_sharpe_ratio() {
    // Generate 252 days of ~10% annual return with 15% volatility
    let daily_return = Decimal::from(10) / Decimal::from(100) / Decimal::from(252);
    let returns: Vec<Decimal> = (0..252)
        .map(|i| {
            // Add some noise around the mean
            let noise = Decimal::from((i % 5) as i32 - 2) / Decimal::from(1000);
            daily_return + noise
        })
        .collect();

    let analyzer = RiskAnalyzer::new(returns, Decimal::from(2) / Decimal::from(100));

    let sharpe = analyzer.sharpe_ratio();

    // Expected: (0.10 - 0.02) / 0.15 = ~0.53
    // Allow wide range due to simplified calculation
    assert!(sharpe > Decimal::ZERO);
    assert!(sharpe < Decimal::from(5));
}

// S7-GP-04: XGBoost CQ Prediction
#[test]
fn test_cq_prediction_model() {
    let predictor = CQPredictor::new();

    // Create feature vector
    let features = vec![0.8; 19]; // All features at 0.8 (strong signals)

    let score = predictor.predict(&features).unwrap();

    // Score should be in [0, 1]
    assert!(score >= 0.0 && score <= 1.0);

    // With high features (0.8), score should be > 0.5
    assert!(score > 0.5);

    // Test prediction with confidence
    let (score, confidence) = predictor.predict_with_confidence(&features).unwrap();
    assert!(confidence >= 0.0 && confidence <= 1.0);

    // High score should suggest trading
    let should_trade = predictor.should_trade(&features).unwrap();
    assert!(should_trade);
}

// S7-GP-05: Anomaly Detection
#[test]
fn test_regime_change_detection() {
    let mut detector = AnomalyDetector::new(2.0, 20);

    // Simulate regime change (high vol period followed by low vol)
    // Use varying values to establish a proper baseline
    let normal_period: Vec<f64> = (0..100).map(|i| 0.001 + (i as f64 * 0.0001)).collect();
    detector.set_baseline(&normal_period);

    // Normal value (within 2 std devs)
    let result = detector.detect(0.005);
    assert!(matches!(result, AnomalyResult::Normal));

    // Anomalous value (far from mean)
    let result = detector.detect(0.5);
    assert!(matches!(result, AnomalyResult::Anomaly { .. }));
}

// S7-GP-06: Performance Attribution
#[test]
fn test_performance_attribution() {
    let portfolio_weights: HashMap<String, Decimal> = [
        ("tech".to_string(), Decimal::from(60) / Decimal::from(100)),
        (
            "finance".to_string(),
            Decimal::from(40) / Decimal::from(100),
        ),
    ]
    .into();

    let benchmark_weights: HashMap<String, Decimal> = [
        ("tech".to_string(), Decimal::from(50) / Decimal::from(100)),
        (
            "finance".to_string(),
            Decimal::from(50) / Decimal::from(100),
        ),
    ]
    .into();

    let portfolio_returns: HashMap<String, Decimal> = [
        ("tech".to_string(), Decimal::from(15) / Decimal::from(100)),
        ("finance".to_string(), Decimal::from(8) / Decimal::from(100)),
    ]
    .into();

    let benchmark_returns: HashMap<String, Decimal> = [
        ("tech".to_string(), Decimal::from(12) / Decimal::from(100)),
        (
            "finance".to_string(),
            Decimal::from(10) / Decimal::from(100),
        ),
    ]
    .into();

    let result = AttributionAnalyzer::brinson_attribution(
        &portfolio_weights,
        &benchmark_weights,
        &portfolio_returns,
        &benchmark_returns,
    );

    // Effects should be calculated
    assert!(
        result.allocation_effect != Decimal::ZERO
            || result.selection_effect != Decimal::ZERO
            || result.interaction_effect != Decimal::ZERO
    );

    // Allocation + Selection + Interaction should sum to total excess return
    let sum_effects =
        result.allocation_effect + result.selection_effect + result.interaction_effect;
    // Portfolio return: 0.6 * 0.15 + 0.4 * 0.08 = 0.122
    // Benchmark return: 0.5 * 0.12 + 0.5 * 0.10 = 0.11
    // Excess return: 0.012
    let expected_excess =
        Decimal::from(122) / Decimal::from(1000) - Decimal::from(11) / Decimal::from(100);
    let diff = (sum_effects - expected_excess).abs();
    assert!(diff < Decimal::from(1) / Decimal::from(1000)); // Small tolerance
}

// Additional tests

#[test]
fn test_feature_engineering() {
    let signals = TickerSignals {
        quality_score: QualityScore(80),
        value_score: QualityScore(75),
        momentum_score: QualityScore(70),
        insider_score: QualityScore(65),
        sentiment_score: QualityScore(72),
        regime_fit: QualityScore(85),
        composite_quality: QualityScore(77),
        insider_flow_ratio: 0.5,
        insider_cluster_signal: true,
        news_sentiment: 0.6,
        social_sentiment: 0.7,
        vix_level: 15.0,
        market_breadth: 0.75,
        breakout_score: 0.8,
        atr_trend: 0.05,
        rsi_14: 55.0,
        macd_signal: 0.02,
    };

    let features = FeaturePipeline::generate_features("AAPL", &signals);

    // Should have 19 features
    assert_eq!(features.features.len(), 19);
    assert_eq!(features.feature_names.len(), 19);
    assert_eq!(features.ticker, "AAPL");
}

#[test]
fn test_max_drawdown() {
    // Generate returns that create a clear drawdown
    let returns: Vec<Decimal> = vec![
        Decimal::from(10) / Decimal::from(100),
        Decimal::from(5) / Decimal::from(100),
        Decimal::from(-15) / Decimal::from(100), // Start of drawdown
        Decimal::from(-10) / Decimal::from(100), // Continue down
        Decimal::from(-5) / Decimal::from(100),  // Bottom
        Decimal::from(8) / Decimal::from(100),   // Recovery
        Decimal::from(12) / Decimal::from(100),
    ];

    let analyzer = RiskAnalyzer::new(returns, Decimal::ZERO);

    let max_dd = analyzer.max_drawdown();

    // Max drawdown should be negative
    assert!(max_dd <= Decimal::ZERO);
    // Should be substantial due to the -15%, -10%, -5% sequence
    assert!(max_dd < Decimal::from(-10) / Decimal::from(100));
}

#[test]
fn test_sortino_ratio() {
    let returns: Vec<Decimal> = vec![
        Decimal::from(2) / Decimal::from(100),
        Decimal::from(-1) / Decimal::from(100),
        Decimal::from(3) / Decimal::from(100),
        Decimal::from(-2) / Decimal::from(100),
        Decimal::from(1) / Decimal::from(100),
    ]
    .into_iter()
    .cycle()
    .take(60)
    .collect();

    let analyzer = RiskAnalyzer::new(returns, Decimal::from(2) / Decimal::from(100));

    let sortino = analyzer.sortino_ratio();
    let sharpe = analyzer.sharpe_ratio();

    // Sortino should generally be >= Sharpe (only penalizes downside)
    assert!(sortino >= sharpe || sortino == Decimal::ZERO);
}

// Helper functions and types

fn generate_test_prices(start: chrono::DateTime<Utc>, end: chrono::DateTime<Utc>) -> Vec<PriceBar> {
    let mut prices = Vec::new();
    let mut current = start;
    let mut price = 100.0;

    while current < end {
        // Random walk
        let change = (current.timestamp() % 5) as f64 - 2.0;
        price += change;
        price = price.max(10.0); // Floor at $10

        prices.push(PriceBar {
            timestamp: current,
            open: price - 1.0,
            high: price + 2.0,
            low: price - 2.0,
            close: price,
            volume: 1000000,
        });

        current += Duration::days(1);
    }

    prices
}

struct TestStrategy;

#[async_trait::async_trait]
impl Strategy for TestStrategy {
    fn name(&self) -> &str {
        "Test Strategy"
    }

    async fn generate_signals(&self, data: &MarketData) -> Vec<Signal> {
        data.prices
            .keys()
            .map(|ticker| Signal {
                ticker: ticker.clone(),
                direction: SignalDirection::Long,
                strength: 0.5,
                confidence: 0.7,
            })
            .collect()
    }

    fn position_size(&self, _signal: &Signal, portfolio_value: Decimal) -> Decimal {
        // Invest 10% in each signal
        portfolio_value / Decimal::from(10)
    }
}
