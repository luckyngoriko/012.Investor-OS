//! Integration tests for Sprint 13: Advanced Risk Management

use investor_os::broker::multi_asset::{AssetClass, MultiAssetPosition};
use investor_os::risk::{AdvancedRiskEngine, PortfolioGreeks, StressTestResults, VaRResult};
use rust_decimal::Decimal;

/// Test risk engine creation
#[test]
fn test_risk_engine_creation() {
    let engine = AdvancedRiskEngine::new();
    assert_eq!(engine.mc_simulations, 100_000);
}

/// Test Monte Carlo VaR calculation
#[test]
fn test_var_calculation() {
    let engine = AdvancedRiskEngine::new();
    let positions = create_test_positions();

    let var_95 = engine.calculate_var_mc(&positions, 0.95, 1);

    assert_eq!(var_95.confidence, 0.95);
    assert_eq!(var_95.time_horizon_days, 1);
    assert_eq!(var_95.simulations, 100_000);
    assert!(var_95.var_amount > Decimal::ZERO);
}

/// Test VaR at different confidence levels
#[test]
fn test_var_confidence_levels() {
    let engine = AdvancedRiskEngine::new();
    let positions = create_test_positions();

    let var_95 = engine.calculate_var_mc(&positions, 0.95, 1);
    let var_99 = engine.calculate_var_mc(&positions, 0.99, 10);

    // 99% VaR should be higher than 95%
    assert!(var_99.var_amount >= var_95.var_amount);

    // 10-day should be higher than 1-day
    assert_eq!(var_99.time_horizon_days, 10);
}

/// Test stress testing
#[test]
fn test_stress_test() {
    let engine = AdvancedRiskEngine::new();
    let positions = create_test_positions();

    let results = engine.stress_test(&positions);

    // Should have scenarios
    assert!(!results.scenarios.is_empty());

    // Survival rate between 0 and 1
    assert!(results.survival_rate >= 0.0);
    assert!(results.survival_rate <= 1.0);
}

/// Test stress scenario survival
#[test]
fn test_stress_scenario_survival() {
    let engine = AdvancedRiskEngine::new();
    let positions = create_test_positions();

    let results = engine.stress_test(&positions);

    // Count survived
    let survived = results.scenarios.iter().filter(|s| s.survived).count();
    let total = results.scenarios.len();

    assert!(survived <= total);
    assert_eq!(results.survival_rate, survived as f64 / total as f64);
}

/// Test portfolio Greeks calculation
#[test]
fn test_portfolio_greeks() {
    let engine = AdvancedRiskEngine::new();
    let positions = create_test_positions();

    let greeks = engine.calculate_greeks(&positions);

    // Delta should be positive (long positions)
    assert!(greeks.delta > Decimal::ZERO);

    // Other Greeks should exist
    assert!(greeks.gamma >= Decimal::ZERO);
    assert!(greeks.vega >= Decimal::ZERO);
    assert!(greeks.theta >= Decimal::ZERO);
}

/// Test Greeks with empty portfolio
#[test]
fn test_greeks_empty_portfolio() {
    let engine = AdvancedRiskEngine::new();
    let positions: Vec<MultiAssetPosition> = vec![];

    let greeks = engine.calculate_greeks(&positions);

    // All Greeks should be zero
    assert_eq!(greeks.delta, Decimal::ZERO);
    assert_eq!(greeks.gamma, Decimal::ZERO);
    assert_eq!(greeks.vega, Decimal::ZERO);
    assert_eq!(greeks.theta, Decimal::ZERO);
}

/// Test correlation matrix
#[test]
fn test_correlation_matrix() {
    let engine = AdvancedRiskEngine::new();
    let positions = create_test_positions();

    let matrix = engine.correlation_matrix(&positions);

    // Should have entries for each position pair
    assert!(!matrix.is_empty());

    // Self-correlation should be 1.0
    if let Some(&corr) = matrix.get(&("AAPL".to_string(), "AAPL".to_string())) {
        assert_eq!(corr, 1.0);
    }
}

/// Test correlation bounds
#[test]
fn test_correlation_bounds() {
    let engine = AdvancedRiskEngine::new();
    let positions = create_test_positions();

    let matrix = engine.correlation_matrix(&positions);

    // All correlations should be between -1 and 1
    for ((_sym1, _sym2), corr) in matrix {
        assert!(corr >= -1.0);
        assert!(corr <= 1.0);
    }
}

/// Test VaR result structure
#[test]
fn test_var_result() {
    let var = VaRResult {
        confidence: 0.95,
        time_horizon_days: 1,
        var_amount: Decimal::from(1000),
        var_pct: 0.05,
        simulations: 100_000,
    };

    assert_eq!(var.confidence, 0.95);
    assert_eq!(var.time_horizon_days, 1);
    assert_eq!(var.var_amount, Decimal::from(1000));
    assert_eq!(var.var_pct, 0.05);
}

/// Test stress test results
#[test]
fn test_stress_results_passed() {
    let results = StressTestResults {
        scenarios: vec![],
        survival_rate: 0.75,
        worst_scenario_loss: Decimal::from_str_exact("0.20").unwrap(),
        passed: true,
    };

    assert!(results.passed);
    assert_eq!(results.survival_rate, 0.75);
    assert!(results.worst_scenario_loss > Decimal::ZERO);
}

/// Test Portfolio Greeks default
#[test]
fn test_greeks_default() {
    let greeks = PortfolioGreeks {
        delta: Decimal::ZERO,
        gamma: Decimal::ZERO,
        vega: Decimal::ZERO,
        theta: Decimal::ZERO,
    };

    assert_eq!(greeks.delta, Decimal::ZERO);
}

/// Helper function to create test positions
fn create_test_positions() -> Vec<MultiAssetPosition> {
    vec![
        MultiAssetPosition {
            symbol: "AAPL".to_string(),
            asset_class: AssetClass::Equity,
            quantity: Decimal::from(100),
            avg_cost: Decimal::from(150),
            current_price: Decimal::from(175),
            market_value: Decimal::from(17500),
            unrealized_pnl: Decimal::from(2500),
            currency: "USD".to_string(),
        },
        MultiAssetPosition {
            symbol: "MSFT".to_string(),
            asset_class: AssetClass::Equity,
            quantity: Decimal::from(50),
            avg_cost: Decimal::from(300),
            current_price: Decimal::from(350),
            market_value: Decimal::from(17500),
            unrealized_pnl: Decimal::from(2500),
            currency: "USD".to_string(),
        },
        MultiAssetPosition {
            symbol: "BTC".to_string(),
            asset_class: AssetClass::Crypto,
            quantity: Decimal::from(1),
            avg_cost: Decimal::from(40000),
            current_price: Decimal::from(50000),
            market_value: Decimal::from(50000),
            unrealized_pnl: Decimal::from(10000),
            currency: "USDT".to_string(),
        },
    ]
}
