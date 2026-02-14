//! Cross-Margining Engine

use super::{BrokerId, PrimeBroker, Position};
use rust_decimal::Decimal;
use std::collections::HashMap;

/// Cross-margining engine
pub struct CrossMarginingEngine {
    hedging_efficiency_threshold: Decimal, // Minimum % to consider hedge
}

impl CrossMarginingEngine {
    pub fn new() -> Self {
        Self {
            hedging_efficiency_threshold: Decimal::try_from(0.95).unwrap(), // 95%
        }
    }
    
    /// Calculate total margin across all brokers
    pub fn calculate_total_margin(&self, _brokers: &HashMap<BrokerId, PrimeBroker>) -> MarginSummary {
        // This would aggregate actual positions
        // For now, return a summary structure
        MarginSummary {
            total_long_exposure: Decimal::ZERO,
            total_short_exposure: Decimal::ZERO,
            net_exposure: Decimal::ZERO,
            gross_exposure: Decimal::ZERO,
            required_margin: Decimal::ZERO,
            available_margin: Decimal::ZERO,
            excess_margin: Decimal::ZERO,
            hedge_ratio: 0.0,
            cross_margin_benefit: Decimal::ZERO,
            by_broker: HashMap::new(),
        }
    }
    
    /// Calculate with actual positions
    pub fn calculate_with_positions(
        &self,
        brokers: &HashMap<BrokerId, PrimeBroker>,
        positions: &HashMap<BrokerId, Vec<Position>>,
    ) -> MarginSummary {
        let mut total_long = Decimal::ZERO;
        let mut total_short = Decimal::ZERO;
        let mut by_broker: HashMap<BrokerId, BrokerMargin> = HashMap::new();
        
        for (broker_id, broker_positions) in positions {
            let mut broker_long = Decimal::ZERO;
            let mut broker_short = Decimal::ZERO;
            
            for pos in broker_positions {
                if pos.quantity > Decimal::ZERO {
                    broker_long += pos.market_value;
                } else {
                    broker_short += pos.market_value.abs();
                }
            }
            
            // Calculate margin for this broker
            let net = broker_long - broker_short;
            let _gross = broker_long + broker_short;
            
            if let Some(broker) = brokers.get(broker_id) {
                let required = if net >= Decimal::ZERO {
                    net * broker.margin_requirements.maintenance_margin_pct / Decimal::from(100)
                } else {
                    net.abs() * broker.margin_requirements.short_margin_pct / Decimal::from(100)
                };
                
                by_broker.insert(broker_id.clone(), BrokerMargin {
                    long_exposure: broker_long,
                    short_exposure: broker_short,
                    net_exposure: net,
                    required_margin: required,
                });
            }
            
            total_long += broker_long;
            total_short += broker_short;
        }
        
        let net_exposure = total_long - total_short;
        let gross_exposure = total_long + total_short;
        
        // Calculate hedge ratio
        let hedge_ratio: f64 = if gross_exposure.is_zero() {
            0.0
        } else {
            let net_f64: f64 = net_exposure.abs().try_into().unwrap_or(0.0);
            let gross_f64: f64 = gross_exposure.try_into().unwrap_or(1.0);
            1.0 - (net_f64 / gross_f64)
        };
        
        // Calculate cross-margin benefit
        // If perfectly hedged, margin requirement is much lower
        let required_margin = if hedge_ratio > 0.95 {
            // Well hedged - margin on net only
            net_exposure.abs() * Decimal::try_from(0.25).unwrap()
        } else {
            // Poorly hedged - margin on gross
            gross_exposure * Decimal::try_from(0.50).unwrap()
        };
        
        // Calculate benefit vs no cross-margining
        let no_cross_margin_required: Decimal = by_broker.values()
            .map(|m| m.required_margin)
            .sum();
        let cross_margin_benefit = no_cross_margin_required - required_margin;
        
        MarginSummary {
            total_long_exposure: total_long,
            total_short_exposure: total_short,
            net_exposure,
            gross_exposure,
            required_margin,
            available_margin: Decimal::ZERO, // Would come from account data
            excess_margin: Decimal::ZERO - required_margin,
            hedge_ratio,
            cross_margin_benefit,
            by_broker,
        }
    }
    
    /// Find optimization opportunities
    pub fn find_optimization_opportunities(
        &self,
        _brokers: &HashMap<BrokerId, PrimeBroker>,
    ) -> Vec<RebalanceSuggestion> {
        
        
        // Example: Find offsetting positions across brokers
        // Long AAPL at Broker A, Short AAPL at Broker B
        // Suggestion: Consolidate to one broker for cross-margin benefit
        
        // This is a simplified version - real implementation would:
        // 1. Aggregate all positions across brokers
        // 2. Find offsetting positions
        // 3. Suggest consolidations
        // 4. Calculate margin savings
        
        Vec::new()
    }
    
    /// Find offsetting positions across brokers
    pub fn find_offsetting_positions(
        &self,
        positions: &HashMap<BrokerId, Vec<Position>>,
    ) -> Vec<OffsettingPosition> {
        let mut offsets = Vec::new();
        let mut position_map: HashMap<String, Vec<(BrokerId, Decimal)>> = HashMap::new();
        
        // Group positions by symbol
        for (broker_id, broker_positions) in positions {
            for pos in broker_positions {
                position_map
                    .entry(pos.symbol.clone())
                    .or_default()
                    .push((broker_id.clone(), pos.quantity));
            }
        }
        
        // Find offsetting positions
        for (symbol, positions_list) in position_map {
            let total: Decimal = positions_list.iter().map(|(_, qty)| qty).sum();
            
            if total.is_zero() {
                // Perfect hedge across brokers
                for (broker_id, qty) in positions_list {
                    offsets.push(OffsettingPosition {
                        symbol: symbol.clone(),
                        broker_id,
                        quantity: qty,
                        is_hedge: true,
                    });
                }
            }
        }
        
        offsets
    }
    
    /// Calculate optimal position distribution
    pub fn optimize_distribution(
        &self,
        positions: &[Position],
        brokers: &[PrimeBroker],
    ) -> OptimalDistribution {
        // Simplified: Distribute to broker with lowest margin requirements
        let mut distribution: HashMap<BrokerId, Vec<Position>> = HashMap::new();
        
        // Find best broker for each position
        for pos in positions {
            let best_broker = brokers.iter()
                .min_by_key(|b| b.margin_requirements.maintenance_margin_pct)
                .map(|b| b.id.clone());
            
            if let Some(broker_id) = best_broker {
                distribution.entry(broker_id).or_default().push(pos.clone());
            }
        }
        
        OptimalDistribution {
            distribution,
            estimated_margin_savings: Decimal::ZERO, // Would calculate actual savings
        }
    }
}

impl Default for CrossMarginingEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Margin summary across all brokers
#[derive(Debug, Clone)]
pub struct MarginSummary {
    pub total_long_exposure: Decimal,
    pub total_short_exposure: Decimal,
    pub net_exposure: Decimal,
    pub gross_exposure: Decimal,
    pub required_margin: Decimal,
    pub available_margin: Decimal,
    pub excess_margin: Decimal,
    pub hedge_ratio: f64,
    pub cross_margin_benefit: Decimal,
    pub by_broker: HashMap<BrokerId, BrokerMargin>,
}

impl MarginSummary {
    /// Check if margin call
    pub fn is_margin_call(&self) -> bool {
        self.excess_margin < Decimal::ZERO
    }
    
    /// Margin utilization percentage
    pub fn utilization_pct(&self) -> f64 {
        if self.available_margin.is_zero() {
            return 0.0;
        }
        let req: f64 = self.required_margin.try_into().unwrap_or(0.0);
        let avail: f64 = self.available_margin.try_into().unwrap_or(1.0);
        (req / avail) * 100.0
    }
}

/// Margin per broker
#[derive(Debug, Clone, Default)]
pub struct BrokerMargin {
    pub long_exposure: Decimal,
    pub short_exposure: Decimal,
    pub net_exposure: Decimal,
    pub required_margin: Decimal,
}

/// Rebalance suggestion
#[derive(Debug, Clone)]
pub struct RebalanceSuggestion {
    pub from_broker: BrokerId,
    pub to_broker: BrokerId,
    pub symbol: String,
    pub quantity: Decimal,
    pub reason: String,
    pub margin_savings: Decimal,
}

/// Offsetting position
#[derive(Debug, Clone)]
pub struct OffsettingPosition {
    pub symbol: String,
    pub broker_id: BrokerId,
    pub quantity: Decimal,
    pub is_hedge: bool,
}

/// Optimal position distribution
#[derive(Debug, Clone)]
pub struct OptimalDistribution {
    pub distribution: HashMap<BrokerId, Vec<Position>>,
    pub estimated_margin_savings: Decimal,
}

/// Cross-margining benefits calculator
pub struct CrossMarginBenefitCalculator;

impl CrossMarginBenefitCalculator {
    /// Calculate savings from cross-margining vs separate margin
    pub fn calculate_savings(
        gross_exposure: Decimal,
        net_exposure: Decimal,
        margin_rate: Decimal,
    ) -> Decimal {
        // Without cross-margining: pay margin on gross
        let without_cross = gross_exposure * margin_rate;
        
        // With cross-margining: pay margin on net
        let with_cross = net_exposure.abs() * margin_rate;
        
        without_cross - with_cross
    }
    
    /// Calculate hedge efficiency
    pub fn hedge_efficiency(long: Decimal, short: Decimal) -> f64 {
        let gross = long + short;
        let net = (long - short).abs();
        
        if gross.is_zero() {
            return 0.0;
        }
        
        let gross_f64: f64 = gross.try_into().unwrap_or(0.0);
        let net_f64: f64 = net.try_into().unwrap_or(0.0);
        
        1.0 - (net_f64 / gross_f64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_margin_summary() {
        let summary = MarginSummary {
            total_long_exposure: Decimal::from(100_000),
            total_short_exposure: Decimal::from(80_000),
            net_exposure: Decimal::from(20_000),
            gross_exposure: Decimal::from(180_000),
            required_margin: Decimal::from(5_000),
            available_margin: Decimal::from(50_000),
            excess_margin: Decimal::from(45_000),
            hedge_ratio: 0.89, // 89% hedged
            cross_margin_benefit: Decimal::from(10_000),
            by_broker: HashMap::new(),
        };
        
        assert!(!summary.is_margin_call());
        assert!(summary.utilization_pct() > 0.0);
        assert!(summary.hedge_ratio > 0.8);
    }

    #[test]
    fn test_hedge_efficiency() {
        // Perfect hedge
        let eff = CrossMarginBenefitCalculator::hedge_efficiency(
            Decimal::from(100_000),
            Decimal::from(100_000),
        );
        assert_eq!(eff, 1.0);
        
        // No hedge
        let eff = CrossMarginBenefitCalculator::hedge_efficiency(
            Decimal::from(100_000),
            Decimal::ZERO,
        );
        assert_eq!(eff, 0.0);
        
        // Partial hedge
        let eff = CrossMarginBenefitCalculator::hedge_efficiency(
            Decimal::from(100_000),
            Decimal::from(50_000),
        );
        assert!(eff > 0.0 && eff < 1.0);
    }

    #[test]
    fn test_cross_margin_savings() {
        let savings = CrossMarginBenefitCalculator::calculate_savings(
            Decimal::from(200_000), // Gross
            Decimal::from(20_000),  // Net
            Decimal::try_from(0.25).unwrap(), // 25% margin
        );
        
        // Savings = (200K - 20K) * 0.25 = 45K
        let savings_f64: f64 = savings.try_into().unwrap();
        assert!(savings_f64 > 40_000.0 && savings_f64 < 50_000.0);
    }

    #[test]
    fn test_find_offsetting_positions() {
        let engine = CrossMarginingEngine::new();
        
        let mut positions: HashMap<BrokerId, Vec<Position>> = HashMap::new();
        
        positions.insert(BrokerId("A".to_string()), vec![
            Position {
                symbol: "AAPL".to_string(),
                quantity: Decimal::from(100),
                avg_price: Decimal::from(150),
                market_price: Decimal::from(150),
                market_value: Decimal::from(15_000),
            },
        ]);
        
        positions.insert(BrokerId("B".to_string()), vec![
            Position {
                symbol: "AAPL".to_string(),
                quantity: Decimal::from(-100), // Short
                avg_price: Decimal::from(150),
                market_price: Decimal::from(150),
                market_value: Decimal::from(-15_000),
            },
        ]);
        
        let offsets = engine.find_offsetting_positions(&positions);
        
        assert_eq!(offsets.len(), 2);
        assert!(offsets.iter().all(|o| o.is_hedge));
    }
}
