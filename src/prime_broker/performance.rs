//! Broker Performance Tracking

use super::{BrokerId, PrimeBroker, ExecutionRecord};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use std::collections::HashMap;

/// Performance tracker
pub struct PerformanceTracker {
    executions: HashMap<BrokerId, Vec<ExecutionRecord>>,
}

impl PerformanceTracker {
    pub fn new() -> Self {
        Self {
            executions: HashMap::new(),
        }
    }
    
    /// Record execution
    pub fn record_execution(&mut self, broker_id: &BrokerId, execution: ExecutionRecord) {
        self.executions
            .entry(broker_id.clone())
            .or_default()
            .push(execution);
    }
    
    /// Calculate execution quality metrics
    pub fn calculate_metrics(&self, broker_id: &BrokerId) -> ExecutionQualityMetrics {
        let execs = match self.executions.get(broker_id) {
            Some(e) if !e.is_empty() => e,
            _ => return ExecutionQualityMetrics::default(),
        };
        
        let total = execs.len() as f64;
        
        // Average slippage
        let avg_slippage: f64 = execs.iter()
            .map(|e| e.slippage_bps as f64)
            .sum::<f64>() / total;
        
        // Average latency
        let avg_latency: f64 = execs.iter()
            .map(|e| e.latency_ms as f64)
            .sum::<f64>() / total;
        
        // Fill rate (assume all fills for now)
        let fill_rate = 1.0;
        
        // Success rate (orders with slippage < 10 bps)
        let success_count = execs.iter()
            .filter(|e| e.slippage_bps.abs() < 10)
            .count() as f64;
        let success_rate = success_count / total;
        
        ExecutionQualityMetrics {
            avg_slippage_bps: avg_slippage as i32,
            avg_latency_ms: avg_latency as u64,
            fill_rate,
            success_rate,
            total_executions: execs.len(),
            last_updated: Utc::now(),
        }
    }
    
    /// Get broker rankings
    pub fn get_rankings(&self, brokers: &HashMap<BrokerId, PrimeBroker>) -> Vec<BrokerRanking> {
        let mut rankings: Vec<_> = brokers.keys()
            .map(|id| {
                let metrics = self.calculate_metrics(id);
                let broker = brokers.get(id).unwrap();
                
                BrokerRanking {
                    broker_id: id.clone(),
                    broker_name: broker.name.clone(),
                    overall_score: metrics.overall_score(),
                    quality_metrics: metrics,
                    reliability: broker.reliability_score,
                }
            })
            .collect();
        
        // Sort by overall score
        rankings.sort_by(|a, b| b.overall_score.partial_cmp(&a.overall_score).unwrap());
        
        rankings
    }
    
    /// Get best performing broker
    pub fn get_best_performer(&self, brokers: &HashMap<BrokerId, PrimeBroker>) -> Option<BrokerRanking> {
        self.get_rankings(brokers).into_iter().next()
    }
    
    /// Compare two brokers
    pub fn compare_brokers(&self, broker_a: &BrokerId, broker_b: &BrokerId) -> BrokerComparison {
        let metrics_a = self.calculate_metrics(broker_a);
        let metrics_b = self.calculate_metrics(broker_b);
        
        let better_slippage = if metrics_a.avg_slippage_bps < metrics_b.avg_slippage_bps { broker_a.clone() } else { broker_b.clone() };
        let better_latency = if metrics_a.avg_latency_ms < metrics_b.avg_latency_ms { broker_a.clone() } else { broker_b.clone() };
        
        BrokerComparison {
            broker_a: broker_a.clone(),
            broker_b: broker_b.clone(),
            metrics_a,
            metrics_b,
            better_slippage,
            better_latency,
        }
    }
}

impl Default for PerformanceTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Execution quality metrics
#[derive(Debug, Clone, Default)]
pub struct ExecutionQualityMetrics {
    pub avg_slippage_bps: i32,
    pub avg_latency_ms: u64,
    pub fill_rate: f64,
    pub success_rate: f64,
    pub total_executions: usize,
    pub last_updated: DateTime<Utc>,
}

impl ExecutionQualityMetrics {
    /// Calculate overall score (0-100)
    pub fn overall_score(&self) -> f64 {
        if self.total_executions == 0 {
            return 0.0;
        }
        
        // Slippage score (lower is better)
        let slippage_score = (100.0 - (self.avg_slippage_bps.abs() as f64)).max(0.0) * 0.3;
        
        // Latency score (lower is better)
        let latency_score = (100.0 - (self.avg_latency_ms as f64 / 10.0)).max(0.0) * 0.3;
        
        // Fill rate score
        let fill_score = self.fill_rate * 100.0 * 0.2;
        
        // Success rate score
        let success_score = self.success_rate * 100.0 * 0.2;
        
        slippage_score + latency_score + fill_score + success_score
    }
}

/// Broker ranking
#[derive(Debug, Clone)]
pub struct BrokerRanking {
    pub broker_id: BrokerId,
    pub broker_name: String,
    pub overall_score: f64,
    pub quality_metrics: ExecutionQualityMetrics,
    pub reliability: f64,
}

/// Broker comparison
#[derive(Debug, Clone)]
pub struct BrokerComparison {
    pub broker_a: BrokerId,
    pub broker_b: BrokerId,
    pub metrics_a: ExecutionQualityMetrics,
    pub metrics_b: ExecutionQualityMetrics,
    pub better_slippage: BrokerId,
    pub better_latency: BrokerId,
}

/// Cost analysis
pub struct CostAnalyzer;

impl CostAnalyzer {
    /// Calculate all-in cost for trade
    pub fn calculate_all_in_cost(
        commission: Decimal,
        slippage_bps: i32,
        market_value: Decimal,
        financing_cost: Decimal,
    ) -> AllInCost {
        // Slippage cost
        let slippage_pct = Decimal::from(slippage_bps.abs()) / Decimal::from(10000);
        let slippage_cost = market_value * slippage_pct;
        
        let total = commission + slippage_cost + financing_cost;
        
        AllInCost {
            commission,
            slippage_cost,
            financing_cost,
            total,
            as_pct: if market_value.is_zero() { Decimal::ZERO } else { total / market_value * Decimal::from(100) },
        }
    }
    
    /// Compare costs between brokers
    pub fn compare_costs(
        broker_a_cost: &AllInCost,
        broker_b_cost: &AllInCost,
    ) -> CostComparison {
        let diff = broker_a_cost.total - broker_b_cost.total;
        let cheaper = if broker_a_cost.total < broker_b_cost.total { "A" } else { "B" };
        
        CostComparison {
            cost_a: broker_a_cost.clone(),
            cost_b: broker_b_cost.clone(),
            difference: diff.abs(),
            cheaper_broker: cheaper,
            savings_pct: if broker_a_cost.total.is_zero() { Decimal::ZERO } else { 
                (diff.abs() / broker_a_cost.total) * Decimal::from(100) 
            },
        }
    }
}

/// All-in cost breakdown
#[derive(Debug, Clone)]
pub struct AllInCost {
    pub commission: Decimal,
    pub slippage_cost: Decimal,
    pub financing_cost: Decimal,
    pub total: Decimal,
    pub as_pct: Decimal,
}

/// Cost comparison
#[derive(Debug, Clone)]
pub struct CostComparison {
    pub cost_a: AllInCost,
    pub cost_b: AllInCost,
    pub difference: Decimal,
    pub cheaper_broker: &'static str,
    pub savings_pct: Decimal,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_execution(slippage: i32, latency: u64) -> ExecutionRecord {
        ExecutionRecord {
            symbol: "AAPL".to_string(),
            quantity: Decimal::from(100),
            price: Decimal::from(150),
            side: super::super::OrderSide::Buy,
            timestamp: Utc::now(),
            slippage_bps: slippage,
            latency_ms: latency,
        }
    }

    #[test]
    fn test_metrics_calculation() {
        let mut tracker = PerformanceTracker::new();
        let broker_id = BrokerId("TEST".to_string());
        
        // Add executions
        for _ in 0..10 {
            tracker.record_execution(&broker_id, create_test_execution(5, 50));
        }
        
        let metrics = tracker.calculate_metrics(&broker_id);
        
        assert_eq!(metrics.total_executions, 10);
        assert_eq!(metrics.avg_slippage_bps, 5);
        assert_eq!(metrics.avg_latency_ms, 50);
    }

    #[test]
    fn test_overall_score() {
        let metrics = ExecutionQualityMetrics {
            avg_slippage_bps: 2,
            avg_latency_ms: 30,
            fill_rate: 1.0,
            success_rate: 0.95,
            total_executions: 100,
            last_updated: Utc::now(),
        };
        
        let score = metrics.overall_score();
        assert!(score > 80.0); // Should be high for good metrics
    }

    #[test]
    fn test_all_in_cost() {
        let cost = CostAnalyzer::calculate_all_in_cost(
            Decimal::from(5),      // $5 commission
            5,                      // 5 bps slippage
            Decimal::from(15_000), // $15K position
            Decimal::from(2),      // $2 financing
        );
        
        assert!(cost.commission > Decimal::ZERO);
        assert!(cost.slippage_cost > Decimal::ZERO);
        assert_eq!(cost.total, cost.commission + cost.slippage_cost + cost.financing_cost);
    }

    #[test]
    fn test_cost_comparison() {
        let cost_a = CostAnalyzer::calculate_all_in_cost(
            Decimal::from(10), 5, Decimal::from(15000), Decimal::from(2)
        );
        let cost_b = CostAnalyzer::calculate_all_in_cost(
            Decimal::from(5), 3, Decimal::from(15000), Decimal::from(1)
        );
        
        let comparison = CostAnalyzer::compare_costs(&cost_a, &cost_b);
        
        assert_eq!(comparison.cheaper_broker, "B");
        assert!(comparison.difference > Decimal::ZERO);
    }
}
