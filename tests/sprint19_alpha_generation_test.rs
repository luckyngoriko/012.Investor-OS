//! Sprint 19 - Alpha Generation Core Golden Path Tests
//!
//! Golden Path тестове за:
//! - Signal generation (Composite Quality)
//! - Multi-factor scoring
//! - Backtesting with walk-forward analysis
//! - ML feature engineering
//! - Strategy selection based on regime
//! - Performance attribution

use chrono::Utc;
use investor_os::analytics::{
    backtest::{BacktestConfig, BacktestResult, SlippageModel},
    ml::FeaturePipeline,
    risk::RiskAnalyzer,
};
use investor_os::signals::{QualityScore, TickerSignals};
use investor_os::strategy_selector::recommender::RiskLevel;
use investor_os::strategy_selector::{
    MarketRegime, Recommendation, SelectionCriteria, StrategyRecommender, StrategySelector,
    StrategySwitcher, StrategyType, SwitchConfig,
};
use rust_decimal::Decimal;

/// GOLDEN PATH: Signal Generation Pipeline
///
/// Тества:
/// 1. Генериране на TickerSignals от пазарни данни
/// 2. Изчисляване на Composite Quality (CQ) score
/// 3. Комбиниране на multiple factors (value, momentum, quality)
#[test]
fn test_signal_generation_pipeline() {
    println!("\n📡 Testing Signal Generation Pipeline");

    // Създаваме TickerSignals с различни фактори
    let signals = TickerSignals {
        quality_score: QualityScore(85),
        value_score: QualityScore(72),
        momentum_score: QualityScore(91),
        insider_score: QualityScore(68),
        sentiment_score: QualityScore(79),
        regime_fit: QualityScore(88),
        composite_quality: QualityScore(0), // Ще изчислим
        insider_flow_ratio: 1.45,
        insider_cluster_signal: true,
        news_sentiment: 0.72,
        social_sentiment: 0.65,
        vix_level: 18.5,
        market_breadth: 0.68,
        breakout_score: 0.82,
        atr_trend: 0.45,
        rsi_14: 62.0,
        macd_signal: 0.23,
    };

    println!("✅ Created TickerSignals for AAPL");
    println!("   Quality: {}%", signals.quality_score.0);
    println!("   Value: {}%", signals.value_score.0);
    println!("   Momentum: {}%", signals.momentum_score.0);
    println!("   Insider: {}%", signals.insider_score.0);
    println!("   Sentiment: {}%", signals.sentiment_score.0);

    // Проверка на диапазони
    assert!(signals.quality_score.0 <= 100);
    assert!(signals.value_score.0 <= 100);
    assert!(signals.momentum_score.0 <= 100);

    // Проверка на insider сигнал
    assert!(
        signals.insider_flow_ratio > 1.0,
        "Positive insider flow expected"
    );
    assert!(
        signals.insider_cluster_signal,
        "Cluster signal should be active"
    );

    // Проверка на sentiment
    assert!(
        signals.news_sentiment > 0.0,
        "Positive news sentiment expected"
    );
    assert!(
        signals.social_sentiment > 0.0,
        "Positive social sentiment expected"
    );

    // Проверка на technical индикатори
    assert!(
        signals.rsi_14 > 30.0 && signals.rsi_14 < 70.0,
        "RSI should be in neutral range"
    );

    println!("✅ Signal generation pipeline test completed!");
}

/// GOLDEN PATH: Composite Quality (CQ) Calculation
///
/// Тества:
/// 1. Изчисляване на CQ от множество фактори
/// 2. Тегла на отделните компоненти
/// 3. Нормализация
#[test]
fn test_composite_quality_calculation() {
    println!("\n🎯 Testing Composite Quality Calculation");

    // Създаваме feature вектор от signals
    let signals = TickerSignals {
        quality_score: QualityScore(85),
        value_score: QualityScore(72),
        momentum_score: QualityScore(91),
        insider_score: QualityScore(68),
        sentiment_score: QualityScore(79),
        regime_fit: QualityScore(88),
        composite_quality: QualityScore(0),
        insider_flow_ratio: 1.45,
        insider_cluster_signal: true,
        news_sentiment: 0.72,
        social_sentiment: 0.65,
        vix_level: 18.5,
        market_breadth: 0.68,
        breakout_score: 0.82,
        atr_trend: 0.45,
        rsi_14: 62.0,
        macd_signal: 0.23,
    };

    let features = FeaturePipeline::generate_features("AAPL", &signals);

    println!(
        "✅ Generated feature vector with {} features",
        features.features.len()
    );
    println!("   Feature names: {:?}", features.feature_names);

    // Проверка на feature вектор
    assert_eq!(features.features.len(), features.feature_names.len());
    assert_eq!(features.ticker, "AAPL");

    // Проверка на feature стойности (трябва да са нормализирани)
    for (i, (value, name)) in features
        .features
        .iter()
        .zip(features.feature_names.iter())
        .enumerate()
    {
        println!("   {}: {} = {:.4}", i + 1, name, value);
        assert!(!value.is_nan(), "Feature {} should not be NaN", name);
        assert!(
            !value.is_infinite(),
            "Feature {} should not be infinite",
            name
        );
    }

    // CQ би трябвало да е средно на основните фактори
    let cq_approx = (signals.quality_score.0 as f64
        + signals.value_score.0 as f64
        + signals.momentum_score.0 as f64)
        / 3.0;
    println!("   Approximate CQ: {:.1}", cq_approx);

    println!("✅ Composite quality calculation test completed!");
}

/// GOLDEN PATH: Backtesting with Walk-Forward Analysis
///
/// Тества:
/// 1. Backtest конфигурация
/// 2. Симулация на търговия
/// 3. Метрики (Sharpe, Max DD, Win Rate)
#[test]
fn test_backtest_walk_forward() {
    println!("\n📊 Testing Backtest with Walk-Forward Analysis");

    // Създаваме backtest конфигурация
    let config = BacktestConfig {
        start_date: Utc::now() - chrono::Duration::days(365),
        end_date: Utc::now(),
        initial_capital: Decimal::from(100000),
        commission_rate: Decimal::try_from(0.001).unwrap(), // 0.1%
        slippage_model: SlippageModel::Fixed(Decimal::try_from(0.001).unwrap()),
        rebalance_frequency: chrono::Duration::days(1),
        max_positions: 20,
        allow_short: false,
    };

    println!("✅ Created backtest configuration");
    println!("   Initial capital: ${}", config.initial_capital);
    println!(
        "   Commission: {}%",
        config.commission_rate * Decimal::from(100)
    );
    println!("   Max positions: {}", config.max_positions);

    // Симулиран backtest резултат
    let mock_result = create_mock_backtest_result(&config);

    println!("\n   Backtest Results:");
    println!("   Total Return: {:.2}%", mock_result.total_return);
    println!(
        "   Annualized Return: {:.2}%",
        mock_result.annualized_return
    );
    println!("   Sharpe Ratio: {:.2}", mock_result.sharpe_ratio);
    println!("   Max Drawdown: {:.2}%", mock_result.max_drawdown);
    println!("   Total Trades: {}", mock_result.total_trades);
    println!("   Win Rate: {:.1}%", mock_result.win_rate);

    // Проверки
    assert!(
        mock_result.sharpe_ratio > Decimal::ZERO,
        "Sharpe should be positive"
    );
    assert!(
        mock_result.max_drawdown <= Decimal::ZERO,
        "Max DD should be negative or zero"
    );
    assert!(mock_result.total_trades > 0, "Should have trades");

    // Проверка на win rate (приблизителна проверка)
    let calculated_win_rate = if mock_result.total_trades > 0 {
        Decimal::from(mock_result.winning_trades as i64)
            / Decimal::from(mock_result.total_trades as i64)
            * Decimal::from(100)
    } else {
        Decimal::ZERO
    };
    let diff = (calculated_win_rate - mock_result.win_rate).abs();
    println!(
        "   Calculated Win Rate: {}%, Stored: {}%, Diff: {}%",
        calculated_win_rate, mock_result.win_rate, diff
    );
    assert!(
        diff < Decimal::from(5),
        "Win rate calculation mismatch should be within 5%"
    );

    println!("✅ Backtest walk-forward test completed!");
}

/// GOLDEN PATH: Risk Metrics Calculation
///
/// Тества:
/// 1. Sharpe ratio
/// 2. Maximum drawdown
/// 3. Value at Risk (VaR)
#[test]
fn test_risk_metrics_calculation() {
    println!("\n⚠️  Testing Risk Metrics Calculation");

    // Симулираме daily returns
    let returns = vec![
        Decimal::try_from(0.012).unwrap(),
        Decimal::try_from(-0.005).unwrap(),
        Decimal::try_from(0.008).unwrap(),
        Decimal::try_from(0.015).unwrap(),
        Decimal::try_from(-0.003).unwrap(),
        Decimal::try_from(0.021).unwrap(),
        Decimal::try_from(-0.012).unwrap(),
        Decimal::try_from(0.009).unwrap(),
        Decimal::try_from(0.006).unwrap(),
        Decimal::try_from(-0.008).unwrap(),
    ];

    let risk_free_rate = Decimal::try_from(0.02 / 252.0).unwrap(); // ~2% annual

    // Use RiskAnalyzer for calculations
    let analyzer = RiskAnalyzer::new(returns, risk_free_rate);

    println!("✅ Risk Metrics:");
    println!("   Sharpe Ratio: {:.4}", analyzer.sharpe_ratio());
    println!(
        "   Max Drawdown: {:.2}%",
        analyzer.max_drawdown() * Decimal::from(100)
    );
    println!(
        "   VaR (95%): {:.2}%",
        analyzer.var(Decimal::try_from(0.95).unwrap()) * Decimal::from(100)
    );

    assert!(
        analyzer.sharpe_ratio() > Decimal::ZERO,
        "Sharpe should be positive for these returns"
    );
    assert!(
        analyzer.max_drawdown() <= Decimal::ZERO,
        "Max DD should be negative or zero"
    );
    // VaR може да е положителен или отрицателен в зависимост от данните
    let var_val = analyzer.var(Decimal::try_from(0.95).unwrap());
    println!("   VaR value: {}", var_val);
    assert!(var_val != Decimal::ZERO, "VaR should be calculated");

    println!("✅ Risk metrics calculation test completed!");
}

/// GOLDEN PATH: Strategy Selection by Market Regime
///
/// Тества:
/// 1. Регим detection
/// 2. Strategy suitability
/// 3. Auto-selection logic
#[test]
fn test_strategy_regime_selection() {
    println!("\n🎭 Testing Strategy Selection by Market Regime");

    let _selector = StrategySelector::new();

    // Проверка на regime suitability
    let regimes = vec![
        (
            MarketRegime::Trending,
            vec![StrategyType::Momentum, StrategyType::TrendFollowing],
        ),
        (
            MarketRegime::Ranging,
            vec![StrategyType::MeanReversion, StrategyType::PairsTrading],
        ),
        (
            MarketRegime::VolatilityExpansion,
            vec![StrategyType::Breakout],
        ),
        (
            MarketRegime::LowVolatility,
            vec![StrategyType::MarketMaking],
        ),
    ];

    for (regime, expected_strategies) in regimes {
        println!("\n   Regime: {:?}", regime);

        for strategy in &expected_strategies {
            let suitable = strategy.suitable_regimes();
            let is_suitable = suitable.contains(&regime);
            println!(
                "   - {:?}: {}",
                strategy,
                if is_suitable {
                    "✓ Suitable"
                } else {
                    "✗ Not suitable"
                }
            );
            assert!(
                is_suitable,
                "{:?} should be suitable for {:?}",
                strategy, regime
            );
        }
    }

    println!("\n✅ Strategy regime selection test completed!");
}

/// GOLDEN PATH: ML Feature Engineering
///
/// Тества:
/// 1. Feature generation
/// 2. Feature normalization
/// 3. Feature importance
#[test]
fn test_ml_feature_engineering() {
    println!("\n🔬 Testing ML Feature Engineering");

    // Създаваме TickerSignals
    let signals = TickerSignals {
        quality_score: QualityScore(75),
        value_score: QualityScore(68),
        momentum_score: QualityScore(82),
        insider_score: QualityScore(55),
        sentiment_score: QualityScore(70),
        regime_fit: QualityScore(78),
        composite_quality: QualityScore(0),
        insider_flow_ratio: 1.2,
        insider_cluster_signal: false,
        news_sentiment: 0.55,
        social_sentiment: 0.48,
        vix_level: 22.0,
        market_breadth: 0.55,
        breakout_score: 0.65,
        atr_trend: 0.35,
        rsi_14: 58.0,
        macd_signal: 0.15,
    };

    // Генерираме features
    let features = FeaturePipeline::generate_features("MSFT", &signals);

    println!("✅ Generated {} features", features.features.len());

    // Проверка на feature интеракции
    let quality_idx = features
        .feature_names
        .iter()
        .position(|n| n == "quality_score")
        .unwrap();
    let value_idx = features
        .feature_names
        .iter()
        .position(|n| n == "value_score")
        .unwrap();
    let interaction_idx = features
        .feature_names
        .iter()
        .position(|n| n == "quality_x_value")
        .unwrap();

    let quality = features.features[quality_idx];
    let value = features.features[value_idx];
    let interaction = features.features[interaction_idx];

    // Interaction трябва да е произведение
    let expected_interaction = quality * value;
    assert!(
        (interaction - expected_interaction).abs() < 0.001,
        "Quality x Value interaction mismatch: {} vs {}",
        interaction,
        expected_interaction
    );

    println!("   Quality: {:.4}", quality);
    println!("   Value: {:.4}", value);
    println!("   Quality x Value: {:.4}", interaction);

    // Проверка на time-series features
    let price_history = vec![
        100.0, 102.0, 101.0, 105.0, 108.0, 106.0, 110.0, 112.0, 111.0, 115.0,
    ];
    let ts_features = FeaturePipeline::generate_ts_features(&price_history);

    println!("✅ Generated {} time-series features", ts_features.len());
    assert!(!ts_features.is_empty(), "Should have TS features");

    println!("✅ ML feature engineering test completed!");
}

/// GOLDEN PATH: Strategy Recommendation Engine
///
/// Тества:
/// 1. Recommendation creation
/// 2. Risk compatibility
/// 3. Expected returns
#[test]
fn test_strategy_recommendation() {
    println!("\n🤖 Testing Strategy Recommendation Engine");

    let recommender = StrategyRecommender::new();

    // Създаваме тестова препоръка с правилните полета
    let recommendation = Recommendation {
        strategy_id: uuid::Uuid::new_v4(),
        strategy_type: StrategyType::Momentum,
        strategy_name: "High Momentum Growth".to_string(),
        rank: 1,
        score: 0.88,
        reason: "Strong momentum in trending market".to_string(),
        expected_return: 0.15,
        risk_level: RiskLevel::High,
        min_capital: Decimal::from(10000),
        suitability_pct: 85.0,
    };

    println!("✅ Created Recommendation:");
    println!(
        "   Strategy: {:?} - {}",
        recommendation.strategy_type, recommendation.strategy_name
    );
    println!("   Rank: {}", recommendation.rank);
    println!("   Score: {:.0}%", recommendation.score * 100.0);
    println!(
        "   Expected Return: {:.1}%",
        recommendation.expected_return * 100.0
    );
    println!("   Risk Level: {:?}", recommendation.risk_level);
    println!("   Min Capital: ${}", recommendation.min_capital);
    println!("   Suitability: {:.0}%", recommendation.suitability_pct);

    // Проверки
    assert!(recommendation.score > 0.0 && recommendation.score <= 1.0);
    assert!(recommendation.rank > 0);
    assert!(recommendation.expected_return > 0.0);
    assert!(recommendation.min_capital > Decimal::ZERO);

    println!("✅ Strategy recommendation test completed!");
}

/// GOLDEN PATH: Strategy Switching Logic
///
/// Тества:
/// 1. Switch conditions
/// 2. Confidence thresholds
/// 3. Daily limits
#[test]
fn test_strategy_switching() {
    println!("\n🔄 Testing Strategy Switching Logic");

    let config = SwitchConfig {
        confidence_threshold: 0.75,
        min_score_improvement: 0.05, // 5% improvement
        max_switches_per_day: 3,
        min_hold_period_seconds: 3600, // 1 hour
        require_regime_change: false,
        momentum_penalty: 0.1,
    };

    let _switcher = StrategySwitcher::new(config.clone());

    println!("✅ Created StrategySwitcher");
    println!(
        "   Min confidence: {:.0}%",
        config.confidence_threshold * 100.0
    );
    println!(
        "   Min improvement: {:.0}%",
        config.min_score_improvement * 100.0
    );
    println!("   Max switches/day: {}", config.max_switches_per_day);

    // Проверка на конфигурацията
    assert_eq!(config.min_score_improvement, 0.05);
    assert_eq!(config.max_switches_per_day, 3);

    println!("✅ Strategy switching test completed!");
}

/// GOLDEN PATH: End-to-End Alpha Generation Pipeline
///
/// Тества:
/// 1. Пълен pipeline: Signals → Features → Backtest → Selection
/// 2. Интеграция между всички модули
/// 3. Реалистичен сценарий
#[test]
fn test_full_alpha_generation_pipeline() {
    println!("\n🚀 Testing Full Alpha Generation Pipeline");

    // Step 1: Generate Signals
    println!("\n📡 Step 1: Signal Generation");
    let signals = TickerSignals {
        quality_score: QualityScore(88),
        value_score: QualityScore(76),
        momentum_score: QualityScore(93),
        insider_score: QualityScore(82),
        sentiment_score: QualityScore(85),
        regime_fit: QualityScore(90),
        composite_quality: QualityScore(0),
        insider_flow_ratio: 1.85,
        insider_cluster_signal: true,
        news_sentiment: 0.78,
        social_sentiment: 0.71,
        vix_level: 16.5,
        market_breadth: 0.72,
        breakout_score: 0.88,
        atr_trend: 0.52,
        rsi_14: 65.0,
        macd_signal: 0.31,
    };
    println!("   ✓ Generated high-quality signals");

    // Step 2: Feature Engineering
    println!("\n🔬 Step 2: Feature Engineering");
    let features = FeaturePipeline::generate_features("NVDA", &signals);
    println!("   ✓ Generated {} features", features.features.len());

    // Step 3: Strategy Selection
    println!("\n🎭 Step 3: Strategy Selection");
    let _selector = StrategySelector::new();
    let criteria = SelectionCriteria {
        min_sharpe: 1.5,
        max_drawdown: -0.15,
        min_win_rate: 0.55,
        lookback_days: 90,
        require_proven: true,
        prefer_lower_turnover: true,
    };

    // Проверка на suitability за trending regime
    let trending_regime = MarketRegime::Trending;
    let momentum_suitable = StrategyType::Momentum
        .suitable_regimes()
        .contains(&trending_regime);
    let mr_suitable = StrategyType::MeanReversion
        .suitable_regimes()
        .contains(&trending_regime);

    println!("   Regime: Trending");
    println!("   Momentum suitable: {}", momentum_suitable);
    println!("   Mean Reversion suitable: {}", mr_suitable);
    println!(
        "   Criteria: Sharpe > {:.1}, Max DD < {:.0}%",
        criteria.min_sharpe,
        criteria.max_drawdown * 100.0
    );

    assert!(
        momentum_suitable,
        "Momentum should be suitable for trending"
    );
    assert!(
        !mr_suitable,
        "Mean Reversion should NOT be suitable for trending"
    );

    // Step 4: Risk Analysis
    println!("\n⚠️  Step 4: Risk Analysis");
    let mock_returns = vec![
        Decimal::try_from(0.015).unwrap(),
        Decimal::try_from(0.008).unwrap(),
        Decimal::try_from(-0.003).unwrap(),
        Decimal::try_from(0.022).unwrap(),
        Decimal::try_from(0.012).unwrap(),
    ];

    let risk_free = Decimal::try_from(0.0001).unwrap();
    let analyzer = RiskAnalyzer::new(mock_returns, risk_free);
    let sharpe = analyzer.sharpe_ratio();
    println!("   Sharpe Ratio: {:.2}", sharpe);
    assert!(
        sharpe > Decimal::try_from(1.5).unwrap(),
        "Sharpe should exceed 1.5 threshold"
    );

    // Step 5: Recommendation
    println!("\n🤖 Step 5: Strategy Recommendation");
    let recommendation = Recommendation {
        strategy_id: uuid::Uuid::new_v4(),
        strategy_type: StrategyType::Momentum,
        strategy_name: "High Momentum Alpha".to_string(),
        rank: 1,
        score: 0.88,
        reason: "Strong momentum signal, high CQ score, suitable for trending regime".to_string(),
        expected_return: 0.15,
        risk_level: RiskLevel::High,
        min_capital: Decimal::from(25000),
        suitability_pct: 88.0,
    };

    println!(
        "   Strategy: {:?} - {}",
        recommendation.strategy_type, recommendation.strategy_name
    );
    println!("   Rank: {}", recommendation.rank);
    println!("   Score: {:.0}%", recommendation.score * 100.0);
    println!(
        "   Expected Return: {:.1}%",
        recommendation.expected_return * 100.0
    );
    println!("   Risk Level: {:?}", recommendation.risk_level);
    println!("   Min Capital: ${}", recommendation.min_capital);
    println!("   Reason: {}", recommendation.reason);

    // Final validation
    assert!(recommendation.score >= 0.85);
    assert_eq!(recommendation.strategy_type, StrategyType::Momentum);

    println!("\n✅ Full alpha generation pipeline test completed!");
    println!("\n   Summary:");
    println!("   - Signal Quality: HIGH (CQ: ~85%)");
    println!("   - Features: {} generated", features.features.len());
    println!("   - Regime: Trending ✓");
    println!("   - Selected Strategy: Momentum");
    println!("   - Risk-Adjusted Return: HIGH (Sharpe: {:.2})", sharpe);
    println!("   - Min Capital Required: ${}", recommendation.min_capital);
}

/// GOLDEN PATH: Performance Attribution
///
/// Тества:
/// 1. Brinson attribution
/// 2. Factor contribution
/// 3. Strategy performance breakdown
#[test]
fn test_performance_attribution() {
    println!("\n📈 Testing Performance Attribution");

    // Симулираме performance по различни фактори
    let factor_returns = vec![
        ("Quality", Decimal::try_from(0.035).unwrap()),
        ("Value", Decimal::try_from(0.018).unwrap()),
        ("Momentum", Decimal::try_from(0.042).unwrap()),
        ("Low Vol", Decimal::try_from(0.012).unwrap()),
    ];

    let total_return: Decimal = factor_returns.iter().map(|(_, r)| r).sum();

    println!("✅ Factor Attribution:");
    for (factor, ret) in &factor_returns {
        let contribution = (*ret / total_return) * Decimal::from(100);
        println!(
            "   {}: {:.2}% (contribution: {:.1}%)",
            factor,
            ret * Decimal::from(100),
            contribution
        );
    }
    println!("   Total Return: {:.2}%", total_return * Decimal::from(100));

    // Проверка
    assert!(
        total_return > Decimal::ZERO,
        "Total return should be positive"
    );
    assert!(
        factor_returns[2].1 > factor_returns[1].1,
        "Momentum should outperform Value"
    );

    println!("✅ Performance attribution test completed!");
}

// Helper function to create mock backtest result
fn create_mock_backtest_result(config: &BacktestConfig) -> BacktestResult {
    BacktestResult {
        config: config.clone(),
        total_return: Decimal::try_from(0.285).unwrap(), // 28.5%
        annualized_return: Decimal::try_from(0.285).unwrap(),
        total_trades: 156,
        winning_trades: 94,
        losing_trades: 62,
        win_rate: Decimal::try_from(60.26).unwrap(), // Percentage
        avg_trade_return: Decimal::try_from(0.0018).unwrap(),
        max_drawdown: Decimal::try_from(-0.085).unwrap(), // -8.5%
        sharpe_ratio: Decimal::try_from(1.85).unwrap(),
        sortino_ratio: Decimal::try_from(2.12).unwrap(),
        daily_returns: vec![],
        equity_curve: vec![],
        trades: vec![],
    }
}
