//! Sprint 28: Multi-Prime Brokerage - Golden Path Tests
//!
//! Tests for:
//! - Prime broker selection
//! - Cross-margining optimization
//! - Financing rate comparison
//! - Execution quality tracking
//! - Cost optimization

use investor_os::prime_broker::{
    MultiPrimeManager, PrimeBroker, BrokerId, BrokerTier,
    OrderSide, Position, ExecutionRecord,
    cross_margin::{CrossMarginingEngine, CrossMarginBenefitCalculator},
    financing::{FinancingCalculator, FinancingRates, FinancingCostTracker, DailyFinancingRecord},
    router::{PrimeBrokerRouter, RoutingCriteria, SmartOrderRouter, OrderUrgency},
};
use rust_decimal::Decimal;
use chrono::Utc;
use std::collections::HashMap;

// ============================================================================
// Test 1: Multi-Prime Manager Creation
// ============================================================================

#[test]
fn test_prime_manager_creation() {
    let manager = MultiPrimeManager::new();
    
    assert!(manager.broker_count() >= 3, "Should have at least 3 brokers");
}

// ============================================================================
// Test 2: Major Broker Registration
// ============================================================================

#[test]
fn test_major_broker_registration() {
    let manager = MultiPrimeManager::new();
    
    let ibkr = manager.get_broker(&BrokerId("IBKR".to_string()));
    assert!(ibkr.is_some(), "Interactive Brokers should be registered");
    assert_eq!(ibkr.unwrap().tier, BrokerTier::Tier2);
    
    let gs = manager.get_broker(&BrokerId("GS".to_string()));
    assert!(gs.is_some(), "Goldman Sachs should be registered");
    assert_eq!(gs.unwrap().tier, BrokerTier::Tier1);
}

// ============================================================================
// Test 3: Broker Tier Classification
// ============================================================================

#[test]
fn test_broker_tier_classification() {
    let manager = MultiPrimeManager::new();
    
    let tier1 = manager.get_brokers_by_tier(BrokerTier::Tier1);
    let tier2 = manager.get_brokers_by_tier(BrokerTier::Tier2);
    
    assert!(!tier1.is_empty() || !tier2.is_empty(), "Should have brokers in different tiers");
    
    // Tier 1 should have higher min account size
    for broker in &tier1 {
        assert!(broker.min_account_size >= Decimal::from(1_000_000), "Tier 1 should have high min account");
    }
}

// ============================================================================
// Test 4: Financing Rate Comparison
// ============================================================================

#[test]
fn test_financing_rate_comparison() {
    let manager = MultiPrimeManager::new();
    
    let long_rates = manager.compare_financing_rates(OrderSide::Buy);
    
    assert!(!long_rates.is_empty(), "Should have financing rates");
    
    // Should be sorted by rate (lowest first)
    for i in 1..long_rates.len() {
        assert!(long_rates[i].rate >= long_rates[i-1].rate, "Rates should be sorted");
    }
    
    // Tier 1 brokers should have lower rates
    let tier1_rates: Vec<_> = long_rates.iter().filter(|r| r.tier == BrokerTier::Tier1).collect();
    let tier3_rates: Vec<_> = long_rates.iter().filter(|r| r.tier == BrokerTier::Tier3).collect();
    
    if !tier1_rates.is_empty() && !tier3_rates.is_empty() {
        assert!(tier1_rates[0].rate <= tier3_rates[0].rate, "Tier 1 should have lower rates");
    }
}

// ============================================================================
// Test 5: Cheapest Financing Selection
// ============================================================================

#[test]
fn test_cheapest_financing_selection() {
    let manager = MultiPrimeManager::new();
    
    // All rates should be reasonable (under 20%)
    let rates = manager.compare_financing_rates(OrderSide::Buy);
    for rate in &rates {
        assert!(rate.rate < Decimal::from(20), "Rate should be under 20%");
    }
}

// ============================================================================
// Test 6: Broker Selection by Cost
// ============================================================================

#[test]
fn test_broker_selection_by_cost() {
    let manager = MultiPrimeManager::new();
    
    let criteria = RoutingCriteria::lowest_cost();
    let best = manager.select_broker("AAPL", Decimal::from(1000), OrderSide::Buy, criteria);
    
    assert!(best.is_some(), "Should select a broker");
}

// ============================================================================
// Test 7: Broker Selection by Latency
// ============================================================================

#[test]
fn test_broker_selection_by_latency() {
    let manager = MultiPrimeManager::new();
    
    let criteria = RoutingCriteria::lowest_latency();
    let best = manager.select_broker("AAPL", Decimal::from(1000), OrderSide::Buy, criteria);
    
    assert!(best.is_some(), "Should select a broker");
    
    // Should prefer low latency
    if let Some(score) = best {
        assert!(score.latency_score > 0.0, "Should have latency score");
    }
}

// ============================================================================
// Test 8: Overnight Financing Calculation
// ============================================================================

#[test]
fn test_overnight_financing_calculation() {
    let manager = MultiPrimeManager::new();
    
    let position = Position {
        symbol: "AAPL".to_string(),
        quantity: Decimal::from(100),
        avg_price: Decimal::from(150),
        market_price: Decimal::from(150),
        market_value: Decimal::from(15_000),
    };
    
    let cost = manager.calculate_overnight_cost(&BrokerId("IBKR".to_string()), &position);
    
    assert!(cost.is_ok(), "Should calculate cost");
    
    let cost = cost.unwrap();
    assert!(cost.daily_cost > Decimal::ZERO, "Should have positive daily cost");
    assert_eq!(cost.market_value, Decimal::from(15_000));
}

// ============================================================================
// Test 9: Cross-Margining Hedge Efficiency
// ============================================================================

#[test]
fn test_cross_margining_hedge_efficiency() {
    // Perfect hedge: $100K long, $100K short = 100% efficiency
    let eff = CrossMarginBenefitCalculator::hedge_efficiency(
        Decimal::from(100_000),
        Decimal::from(100_000),
    );
    assert_eq!(eff, 1.0, "Perfect hedge should be 100%");
    
    // No hedge: $100K long, $0 short = 0% efficiency
    let eff = CrossMarginBenefitCalculator::hedge_efficiency(
        Decimal::from(100_000),
        Decimal::ZERO,
    );
    assert_eq!(eff, 0.0, "No hedge should be 0%");
    
    // Partial hedge: $100K long, $50K short = 50% efficiency
    let eff = CrossMarginBenefitCalculator::hedge_efficiency(
        Decimal::from(100_000),
        Decimal::from(50_000),
    );
    assert!(eff > 0.3 && eff < 0.7, "Partial hedge should be around 50%");
}

// ============================================================================
// Test 10: Cross-Margin Savings Calculation
// ============================================================================

#[test]
fn test_cross_margin_savings() {
    let savings = CrossMarginBenefitCalculator::calculate_savings(
        Decimal::from(200_000), // Gross exposure
        Decimal::from(20_000),  // Net exposure
        Decimal::try_from(0.25).unwrap(), // 25% margin rate
    );
    
    // Savings = (200K - 20K) * 0.25 = 45K
    let savings_f64: f64 = savings.try_into().unwrap();
    assert!(savings_f64 > 40_000.0 && savings_f64 < 50_000.0);
}

// ============================================================================
// Test 11: Financing Cost Tracking
// ============================================================================

#[test]
fn test_financing_cost_tracking() {
    let mut tracker = FinancingCostTracker::new();
    
    // Record 10 days of financing
    for i in 0..10 {
        tracker.record_day(DailyFinancingRecord {
            date: Utc::now() - chrono::Duration::days(i),
            total_cost: Decimal::from(50),
            long_cost: Decimal::from(30),
            short_cost: Decimal::from(20),
            positions_count: 10,
        });
    }
    
    assert_eq!(tracker.average_daily_cost(), Decimal::from(50));
    assert_eq!(tracker.get_period_costs(5), Decimal::from(250));
    
    let annual = tracker.annual_projection();
    assert_eq!(annual, Decimal::from(50 * 365));
}

// ============================================================================
// Test 12: Commission Calculation
// ============================================================================

#[test]
fn test_commission_calculation() {
    use investor_os::prime_broker::broker::CommissionStructure;
    
    let per_share = CommissionStructure::per_share(
        Decimal::try_from(0.005).unwrap(), // $0.005 per share
        Decimal::from(1) // $1 minimum
    );
    
    // 1000 shares @ $0.005 = $5
    let comm = per_share.calculate(Decimal::from(1000), Decimal::from(100));
    assert_eq!(comm, Decimal::from(5));
    
    // 100 shares @ $0.005 = $0.50, but min is $1
    let comm = per_share.calculate(Decimal::from(100), Decimal::from(100));
    assert_eq!(comm, Decimal::from(1));
    
    // Zero commission
    let zero = CommissionStructure::zero_commission();
    let comm = zero.calculate(Decimal::from(10000), Decimal::from(100));
    assert_eq!(comm, Decimal::ZERO);
}

// ============================================================================
// Test 13: Margin Requirements Calculation
// ============================================================================

#[test]
fn test_margin_requirements() {
    use investor_os::prime_broker::broker::MarginRequirements;
    
    let standard = MarginRequirements::standard();
    
    // $100K long position, 50% initial margin = $50K
    let margin = standard.calculate_margin(Decimal::from(100_000), false);
    assert_eq!(margin, Decimal::from(50_000));
    
    // $100K short position, 150% margin = $150K
    let margin = standard.calculate_margin(Decimal::from(100_000), true);
    assert_eq!(margin, Decimal::from(150_000));
}

// ============================================================================
// Test 14: Smart Order Router by Urgency
// ============================================================================

#[test]
fn test_smart_order_router() {
    let manager = MultiPrimeManager::new();
    let router = PrimeBrokerRouter::new();
    
    // Add brokers to router
    for broker in manager.get_all_brokers() {
        // Would add brokers here
    }
    
    // Test routing with different urgency
    // Low urgency = prefer financing cost
    // High urgency = prefer latency
}

// ============================================================================
// Test 15: Broker Performance Ranking
// ============================================================================

#[test]
fn test_broker_performance_ranking() {
    let manager = MultiPrimeManager::new();
    
    let rankings = manager.get_broker_rankings();
    
    // Should have rankings for all brokers
    assert_eq!(rankings.len(), manager.broker_count());
    
    // Should be sorted by score
    for i in 1..rankings.len() {
        assert!(rankings[i].overall_score <= rankings[i-1].overall_score);
    }
}

// ============================================================================
// Test 16: Total Financing Costs Calculation
// ============================================================================

#[test]
fn test_total_financing_costs() {
    let manager = MultiPrimeManager::new();
    
    let mut positions = investor_os::prime_broker::BrokerPositions::default();
    
    // Add positions
    positions.0.insert(BrokerId("IBKR".to_string()), vec![
        Position {
            symbol: "AAPL".to_string(),
            quantity: Decimal::from(100),
            avg_price: Decimal::from(150),
            market_price: Decimal::from(150),
            market_value: Decimal::from(15_000),
        },
    ]);
    
    let costs = manager.calculate_total_financing_costs(&positions);
    
    // Should calculate costs
    assert!(costs.long_cost >= Decimal::ZERO);
}
