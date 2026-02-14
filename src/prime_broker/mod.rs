//! Multi-Prime Brokerage Module - Sprint 28
//!
//! Provides institutional-grade multi-broker trading capabilities:
//! - Prime broker selection based on cost and quality
//! - Cross-margining across brokers
//! - Financing rate optimization
//! - Execution quality tracking

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use std::collections::HashMap;
use thiserror::Error;

pub mod broker;
pub mod financing;
pub mod cross_margin;
pub mod router;
pub mod performance;

pub use broker::{PrimeBroker, BrokerId, BrokerTier, CommissionStructure, MarginRequirements};
pub use financing::{FinancingRates, FinancingCalculator, OvernightCost};
pub use cross_margin::{CrossMarginingEngine, MarginSummary, RebalanceSuggestion};
pub use router::{PrimeBrokerRouter, RoutingCriteria, BrokerScore};
pub use performance::{ExecutionQualityMetrics, PerformanceTracker, BrokerRanking};

/// Prime broker error
#[derive(Error, Debug, Clone)]
pub enum PrimeBrokerError {
    #[error("Broker not found: {0:?}")]
    BrokerNotFound(BrokerId),
    
    #[error("Insufficient margin: required {required}, available {available}")]
    InsufficientMargin { broker: BrokerId, required: Decimal, available: Decimal },
    
    #[error("Financing calculation failed: {0}")]
    FinancingError(String),
    
    #[error("Cross-margining error: {0}")]
    CrossMarginError(String),
    
    #[error("Routing failed: {0}")]
    RoutingError(String),
}

/// Multi-prime brokerage manager
pub struct MultiPrimeManager {
    brokers: HashMap<BrokerId, PrimeBroker>,
    router: PrimeBrokerRouter,
    margin_engine: CrossMarginingEngine,
    performance_tracker: PerformanceTracker,
}

impl MultiPrimeManager {
    pub fn new() -> Self {
        let mut manager = Self {
            brokers: HashMap::new(),
            router: PrimeBrokerRouter::new(),
            margin_engine: CrossMarginingEngine::new(),
            performance_tracker: PerformanceTracker::new(),
        };
        manager.initialize_default_brokers();
        manager
    }
    
    /// Initialize default prime brokers
    fn initialize_default_brokers(&mut self) {
        // Tier 1: Global Banks
        self.register_broker(PrimeBroker::interactive_brokers());
        self.register_broker(PrimeBroker::goldman_sachs());
        self.register_broker(PrimeBroker::morgan_stanley());
        
        // Tier 2: Specialized
        self.register_broker(PrimeBroker::schwab());
        self.register_broker(PrimeBroker::lightspeed());
    }
    
    /// Register a prime broker
    pub fn register_broker(&mut self, broker: PrimeBroker) {
        let id = broker.id.clone();
        self.brokers.insert(id.clone(), broker.clone());
        self.router.add_broker(broker);
    }
    
    /// Get best broker for order based on criteria
    pub fn select_broker(
        &self,
        symbol: &str,
        quantity: Decimal,
        side: OrderSide,
        criteria: RoutingCriteria,
    ) -> Option<BrokerScore> {
        self.router.select_best_broker(symbol, quantity, side, criteria)
    }
    
    /// Calculate overnight financing for position
    pub fn calculate_overnight_cost(
        &self,
        broker_id: &BrokerId,
        position: &Position,
    ) -> Result<OvernightCost, PrimeBrokerError> {
        let broker = self.brokers.get(broker_id)
            .ok_or_else(|| PrimeBrokerError::BrokerNotFound(broker_id.clone()))?;
        
        let calculator = FinancingCalculator::new(&broker.financing_rates);
        let cost = calculator.calculate_overnight_cost(position);
        
        Ok(cost)
    }
    
    /// Calculate total margin across all brokers
    pub fn calculate_total_margin(&self) -> MarginSummary {
        self.margin_engine.calculate_total_margin(&self.brokers)
    }
    
    /// Find cross-margining optimization opportunities
    pub fn find_optimization_opportunities(&self) -> Vec<RebalanceSuggestion> {
        self.margin_engine.find_optimization_opportunities(&self.brokers)
    }
    
    /// Get broker by ID
    pub fn get_broker(&self, id: &BrokerId) -> Option<&PrimeBroker> {
        self.brokers.get(id)
    }
    
    /// Get all brokers
    pub fn get_all_brokers(&self) -> Vec<&PrimeBroker> {
        self.brokers.values().collect()
    }
    
    /// Get brokers by tier
    pub fn get_brokers_by_tier(&self, tier: BrokerTier) -> Vec<&PrimeBroker> {
        self.brokers
            .values()
            .filter(|b| b.tier == tier)
            .collect()
    }
    
    /// Compare financing rates across brokers
    pub fn compare_financing_rates(&self, side: OrderSide) -> Vec<BrokerRateComparison> {
        let mut comparisons: Vec<_> = self.brokers
            .values()
            .map(|broker| {
                let rate = match side {
                    OrderSide::Buy => broker.financing_rates.long_rate(),
                    OrderSide::Sell => broker.financing_rates.short_rate(),
                };
                BrokerRateComparison {
                    broker_id: broker.id.clone(),
                    broker_name: broker.name.clone(),
                    rate,
                    tier: broker.tier,
                }
            })
            .collect();
        
        // Sort by rate (lowest first)
        comparisons.sort_by(|a, b| a.rate.cmp(&b.rate));
        comparisons
    }
    
    /// Get broker performance ranking
    pub fn get_broker_rankings(&self) -> Vec<BrokerRanking> {
        self.performance_tracker.get_rankings(&self.brokers)
    }
    
    /// Track execution for performance monitoring
    pub fn track_execution(
        &mut self,
        broker_id: &BrokerId,
        execution: ExecutionRecord,
    ) {
        self.performance_tracker.record_execution(broker_id, execution);
    }
    
    /// Calculate total financing costs across all brokers
    pub fn calculate_total_financing_costs(&self, positions: &BrokerPositions) -> TotalFinancingCost {
        let mut total_long_cost = Decimal::ZERO;
        let mut total_short_cost = Decimal::ZERO;
        let mut by_broker = HashMap::new();
        
        for (broker_id, broker_positions) in &positions.0 {
            if let Some(broker) = self.brokers.get(broker_id) {
                let calculator = FinancingCalculator::new(&broker.financing_rates);
                let mut broker_long = Decimal::ZERO;
                let mut broker_short = Decimal::ZERO;
                
                for position in broker_positions {
                    let cost = calculator.calculate_overnight_cost(position);
                    if position.quantity > Decimal::ZERO {
                        broker_long += cost.total_cost;
                    } else {
                        broker_short += cost.total_cost;
                    }
                }
                
                total_long_cost += broker_long;
                total_short_cost += broker_short;
                
                by_broker.insert(broker_id.clone(), BrokerFinancingCost {
                    long_cost: broker_long,
                    short_cost: broker_short,
                    total: broker_long + broker_short,
                });
            }
        }
        
        TotalFinancingCost {
            long_cost: total_long_cost,
            short_cost: total_short_cost,
            total: total_long_cost + total_short_cost,
            by_broker,
        }
    }
    
    /// Get broker count
    pub fn broker_count(&self) -> usize {
        self.brokers.len()
    }
}

impl Default for MultiPrimeManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Broker rate comparison
#[derive(Debug, Clone)]
pub struct BrokerRateComparison {
    pub broker_id: BrokerId,
    pub broker_name: String,
    pub rate: Decimal,
    pub tier: BrokerTier,
}

/// Order side
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderSide {
    Buy,
    Sell,
}

/// Position representation
#[derive(Debug, Clone)]
pub struct Position {
    pub symbol: String,
    pub quantity: Decimal,
    pub avg_price: Decimal,
    pub market_price: Decimal,
    pub market_value: Decimal,
}

/// Execution record
#[derive(Debug, Clone)]
pub struct ExecutionRecord {
    pub symbol: String,
    pub quantity: Decimal,
    pub price: Decimal,
    pub side: OrderSide,
    pub timestamp: DateTime<Utc>,
    pub slippage_bps: i32,
    pub latency_ms: u64,
}

/// Positions grouped by broker
#[derive(Debug, Clone, Default)]
pub struct BrokerPositions(pub HashMap<BrokerId, Vec<Position>>);

/// Total financing cost summary
#[derive(Debug, Clone)]
pub struct TotalFinancingCost {
    pub long_cost: Decimal,
    pub short_cost: Decimal,
    pub total: Decimal,
    pub by_broker: HashMap<BrokerId, BrokerFinancingCost>,
}

/// Financing cost per broker
#[derive(Debug, Clone)]
pub struct BrokerFinancingCost {
    pub long_cost: Decimal,
    pub short_cost: Decimal,
    pub total: Decimal,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manager_creation() {
        let manager = MultiPrimeManager::new();
        assert!(manager.broker_count() >= 3);
    }

    #[test]
    fn test_broker_lookup() {
        let manager = MultiPrimeManager::new();
        
        let ibkr = manager.get_broker(&BrokerId("IBKR".to_string()));
        assert!(ibkr.is_some());
    }

    #[test]
    fn test_financing_rate_comparison() {
        let manager = MultiPrimeManager::new();
        
        let comparisons = manager.compare_financing_rates(OrderSide::Buy);
        assert!(!comparisons.is_empty());
        
        // Should be sorted by rate
        for i in 1..comparisons.len() {
            assert!(comparisons[i].rate >= comparisons[i-1].rate);
        }
    }

    #[test]
    fn test_broker_by_tier() {
        let manager = MultiPrimeManager::new();
        
        let tier1 = manager.get_brokers_by_tier(BrokerTier::Tier1);
        let tier2 = manager.get_brokers_by_tier(BrokerTier::Tier2);
        
        // Should have brokers in different tiers
        assert!(!tier1.is_empty() || !tier2.is_empty());
    }
}
