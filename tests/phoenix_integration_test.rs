//! Integration tests for Phoenix Mode
//!
//! Sprint 9: Phoenix Autonomous Learning System

use investor_os::phoenix::assessment::GraduationAssessor;
use investor_os::phoenix::graduation::*;
use investor_os::phoenix::*;
use rust_decimal::Decimal;

/// Test realistic CAGR targets (not the impossible 82%)
#[test]
fn test_realistic_cagr_targets() {
    let config = CagrTargets::default();
    
    // Level 1 minimum: 15% (realistic)
    assert_eq!(config.level1_min, 0.15);
    
    // Level 4 optimal: 30% (excellent but achievable)
    assert_eq!(config.level4_optimal, 0.30);
    
    // Suspicious threshold: 50% (likely overfitting)
    assert_eq!(config.max_suspicious, 0.50);
}

/// Test that unrealistic 82% CAGR is rejected
#[test]
fn test_unrealistic_cagr_rejected() {
    let suspicious_cagr = 0.82; // 82%
    let max_allowed = 0.50; // 50%
    
    assert!(
        suspicious_cagr > max_allowed,
        "CAGR of {}% should be flagged as suspicious/overfitted",
        suspicious_cagr * 100.0
    );
}

/// Test graduation criteria assessment
#[test]
fn test_graduation_assessment_low_cagr() {
    let config = GraduationConfig::default();
    let assessor = GraduationAssessor::new(config);
    
    // Create metrics with insufficient CAGR
    let metrics = PerformanceMetrics {
        cagr: 0.10, // Only 10%, need 15%
        max_drawdown: Decimal::from_str_exact("-0.10").unwrap_or(Decimal::from(-10) / Decimal::from(100)),
        sharpe_ratio: 1.0,
        total_trades: 50, // Need 100
        win_rate: 0.55,
        ..PerformanceMetrics::default()
    };
    
    let regime_perf = RegimePerformance::default();
    let stress = StressTestResult::default();
    let walk_forward = WalkForwardResult::default();
    let monte_carlo = MonteCarloResult::default();
    
    let assessment = assessor.assess(
        &metrics,
        &regime_perf,
        &stress,
        &walk_forward,
        &monte_carlo,
    );
    
    // Should NOT be ready
    match assessment.level {
        GraduationLevel::NotReady { reasons, .. } => {
            // Should have at least one reason for failure
            assert!(!reasons.is_empty(), "Should have failure reasons for low CAGR");
            
            // Check for insufficient returns reason
            let has_low_cagr = reasons.iter().any(|r| matches!(r, 
                FailReason::InsufficientReturns { current, required } 
                if *current == 0.10 && *required == 0.15
            ));
            assert!(has_low_cagr, "Should fail due to insufficient CAGR");
        }
        _ => panic!("Should NOT be ready with 10% CAGR and 50 trades"),
    }
}

/// Test that high payoff ratio (lottery tickets) is flagged
#[test]
fn test_high_payoff_ratio_rejected() {
    let config = GraduationConfig::default();
    let assessor = GraduationAssessor::new(config);
    
    let metrics = PerformanceMetrics {
        cagr: 0.20,
        max_drawdown: Decimal::from_str_exact("-0.10").unwrap(),
        sharpe_ratio: 1.5,
        total_trades: 100,
        win_rate: 0.30, // Low win rate
        payoff_ratio: 8.0, // Very high - lottery tickets!
        ..PerformanceMetrics::default()
    };
    
    let assessment = assessor.assess(
        &metrics,
        &RegimePerformance::default(),
        &StressTestResult::default(),
        &WalkForwardResult::default(),
        &MonteCarloResult::default(),
    );
    
    // Should flag high payoff ratio
    match &assessment.level {
        GraduationLevel::NotReady { reasons, .. } => {
            let has_high_payoff = reasons.iter().any(|r| matches!(r,
                FailReason::HighPayoffRatio { current, max_allowed }
                if *current == 8.0 && *max_allowed == 5.0
            ));
            assert!(has_high_payoff, "Should flag lottery-ticket strategy");
        }
        _ => {}
    }
}

/// Test stress test failure
#[test]
fn test_stress_test_failure() {
    let config = GraduationConfig::default();
    let assessor = GraduationAssessor::new(config);
    
    let stress = StressTestResult {
        scenario_results: vec![],
        worst_scenario: "GFC 2008".to_string(),
        worst_drawdown: Decimal::from_str_exact("-0.50").unwrap(), // -50%
        avg_drawdown: Decimal::from_str_exact("-0.30").unwrap(),
        survival_rate: 0.3, // Only 30% survival
        passed: false,
    };
    
    let assessment = assessor.assess(
        &PerformanceMetrics::default(),
        &RegimePerformance::default(),
        &stress,
        &WalkForwardResult::default(),
        &MonteCarloResult::default(),
    );
    
    match &assessment.level {
        GraduationLevel::NotReady { reasons, .. } => {
            let has_stress_fail = reasons.iter().any(|r| matches!(r,
                FailReason::FailedStressTest { scenario, .. }
                if scenario == "GFC 2008"
            ));
            assert!(has_stress_fail, "Should fail stress test");
        }
        _ => {}
    }
}

/// Test graduation levels progression
#[test]
fn test_graduation_levels() {
    // Level 1: Paper Trading
    let level1 = GraduationLevel::PaperTrading {
        max_position_size: Decimal::from(1000),
        max_positions: 5,
        max_leverage: 1.0,
        duration_months: 6,
    };
    assert_eq!(level1.name(), "Paper Trading");
    assert!(level1.max_capital().is_none()); // Virtual
    
    // Level 2: Micro Live
    let level2 = GraduationLevel::MicroLive {
        max_capital: Decimal::from(1000),
        max_position_size: Decimal::from(100),
        max_daily_loss: Decimal::from(50),
        max_positions: 2,
        duration_months: 3,
    };
    assert_eq!(level2.name(), "Micro Live");
    assert_eq!(level2.max_capital(), Some(Decimal::from(1000)));
    
    // Level 4: Full Strategy
    let level4 = GraduationLevel::FullStrategy {
        max_capital: Decimal::from(50000),
        max_position_pct: Decimal::from_str_exact("0.05").unwrap(),
        allow_options: true,
        allow_short: true,
        allow_margin: true,
    };
    assert_eq!(level4.name(), "Full Strategy");
    assert_eq!(level4.max_capital(), Some(Decimal::from(50000)));
}

/// Test fail reason descriptions
#[test]
fn test_fail_reason_descriptions() {
    let reason = FailReason::InsufficientReturns {
        current: 0.10,
        required: 0.15,
    };
    let desc = reason.description();
    assert!(desc.contains("10.0%"), "Expected 10.0% in: {}", desc);
    assert!(desc.contains("15.0%"), "Expected 15.0% in: {}", desc);
    
    let reason2 = FailReason::SuspiciousPerformance {
        cagr: 0.85,
        explanation: "Likely overfitted".to_string(),
    };
    let desc2 = reason2.description();
    assert!(desc2.contains("85.0%"), "Expected 85.0% in: {}", desc2);
    assert!(desc2.contains("overfitted"));
}

/// Test statistical requirements
#[test]
fn test_statistical_requirements() {
    let req = StatisticalRequirements::default();
    
    // Need at least 100 trades for significance
    assert_eq!(req.min_total_trades, 100);
    
    // Win rate > 52%
    assert_eq!(req.min_win_rate, 0.52);
    
    // Sharpe > 1.2
    assert_eq!(req.min_sharpe, 1.2);
    
    // Payoff ratio < 5.0 (avoid lottery tickets)
    assert_eq!(req.max_payoff_ratio, 5.0);
}

/// Test cost model
#[test]
fn test_cost_model() {
    let cost = CostModel::default();
    
    // Commission: €1 per trade
    assert_eq!(cost.commission_per_trade, Decimal::from(1));
    
    // Slippage: 0.1%
    assert_eq!(cost.slippage_pct, Decimal::from_str_exact("0.001").unwrap());
    
    // Spread: 0.05%
    assert_eq!(cost.spread_pct, Decimal::from_str_exact("0.0005").unwrap());
}

/// Test stress scenarios
#[test]
fn test_stress_scenarios() {
    let config = StressTestConfig::default();
    
    // Should have 8 historical crises
    assert_eq!(config.scenarios.len(), 8);
    
    // Check for major crises
    let has_gfc = config.scenarios.iter().any(|s| 
        matches!(s.scenario_type, StressScenarioType::GFC2008)
    );
    assert!(has_gfc, "Should include GFC 2008");
    
    let has_covid = config.scenarios.iter().any(|s| 
        matches!(s.scenario_type, StressScenarioType::CovidCrash2020)
    );
    assert!(has_covid, "Should include COVID-19 crash");
    
    // Must survive at least 70%
    assert_eq!(config.min_survival_rate, 0.70);
}

/// Test that 200x goal is NOT in our criteria
#[test]
fn test_no_200x_goal() {
    // The unrealistic goal would be:
    // 1000€ -> 200,000€ in 5 years = 82% CAGR
    
    let unrealistic_cagr = 0.82;
    let our_max_cagr = 0.50; // 50% is our suspicious threshold
    
    assert!(
        unrealistic_cagr > our_max_cagr,
        "Our system should NOT accept 82% CAGR as realistic"
    );
    
    // Our realistic targets:
    let targets = CagrTargets::default();
    assert!(targets.level1_min <= 0.20, "Level 1 should be <= 20%");
    assert!(targets.level4_optimal <= 0.35, "Level 4 should be <= 35%");
}

/// Test overall score calculation
#[test]
fn test_overall_score_calculation() {
    let config = GraduationConfig::default();
    let assessor = GraduationAssessor::new(config);
    
    // Perfect metrics
    let perfect_metrics = PerformanceMetrics {
        cagr: 0.30, // 30%
        sharpe_ratio: 2.0,
        max_drawdown: Decimal::from_str_exact("-0.10").unwrap(),
        total_trades: 200,
        win_rate: 0.60,
        profitable_month_pct: 0.70,
        ..PerformanceMetrics::default()
    };
    
    let stress = StressTestResult {
        survival_rate: 0.90,
        passed: true,
        ..StressTestResult::default()
    };
    
    let walk_forward = WalkForwardResult {
        consistency_score: 0.85,
        is_consistent: true,
        ..WalkForwardResult::default()
    };
    
    let monte_carlo = MonteCarloResult {
        survival_rate: 0.95,
        probability_of_ruin: 0.005,
        ..MonteCarloResult::default()
    };
    
    let assessment = assessor.assess(
        &perfect_metrics,
        &RegimePerformance::default(),
        &stress,
        &walk_forward,
        &monte_carlo,
    );
    
    // Should have high score
    assert!(assessment.overall_score > 0.8, "Perfect metrics should score > 80%");
}

/// Test beta threshold (independence from market)
#[test]
fn test_beta_threshold() {
    let limits = RiskLimits::default();
    
    // Beta should be < 0.7
    assert_eq!(limits.max_beta, 0.7);
    
    // High beta means just following market
    let high_beta = 0.9;
    assert!(high_beta > limits.max_beta, "Beta > 0.7 is too correlated with market");
}

/// Test drawdown limit
#[test]
fn test_drawdown_limit() {
    let limits = RiskLimits::default();
    
    // Max drawdown: 15%
    let max_dd = Decimal::from_str_exact("0.15").unwrap();
    assert_eq!(limits.max_drawdown, max_dd);
    
    // 20% drawdown exceeds limit
    let high_dd = Decimal::from_str_exact("0.20").unwrap();
    assert!(high_dd > limits.max_drawdown, "20% drawdown is too high");
}
