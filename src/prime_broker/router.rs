//! Prime Broker Router

use super::{PrimeBroker, BrokerId, OrderSide};
use rust_decimal::Decimal;

/// Prime broker router
pub struct PrimeBrokerRouter {
    brokers: Vec<PrimeBroker>,
}

impl PrimeBrokerRouter {
    pub fn new() -> Self {
        Self {
            brokers: Vec::new(),
        }
    }
    
    pub fn add_broker(&mut self, broker: PrimeBroker) {
        self.brokers.push(broker);
    }
    
    /// Select best broker based on criteria
    pub fn select_best_broker(
        &self,
        symbol: &str,
        quantity: Decimal,
        side: OrderSide,
        criteria: RoutingCriteria,
    ) -> Option<BrokerScore> {
        let mut scores: Vec<BrokerScore> = self.brokers.iter()
            .filter(|b| self.broker_supports_symbol(b, symbol))
            .map(|broker| self.score_broker(broker, symbol, quantity, side, &criteria))
            .collect();
        
        if scores.is_empty() {
            return None;
        }
        
        // Sort by total score (highest first)
        scores.sort_by(|a, b| b.total_score.partial_cmp(&a.total_score).unwrap());
        
        scores.into_iter().next()
    }
    
    /// Score a broker for this order
    fn score_broker(
        &self,
        broker: &PrimeBroker,
        _symbol: &str,
        quantity: Decimal,
        side: OrderSide,
        criteria: &RoutingCriteria,
    ) -> BrokerScore {
        let market_price = Decimal::from(100); // Would get actual price
        let market_value = quantity * market_price;
        
        // Cost score (lower is better, so we invert)
        let commission = broker.calculate_commission(quantity, market_price);
        let financing_rate = match side {
            OrderSide::Buy => broker.financing_rates.long_rate(),
            OrderSide::Sell => broker.financing_rates.short_rate(),
        };
        let _estimated_daily_financing = market_value * (financing_rate / Decimal::from(100)) / Decimal::from(360);
        
        let cost_score = if criteria.max_commission.is_some() && commission > criteria.max_commission.unwrap() {
            0.0
        } else {
            // Normalize cost score (lower commission = higher score)
            let comm_f64: f64 = commission.try_into().unwrap_or(0.0);
            let value_f64: f64 = market_value.try_into().unwrap_or(1.0);
            let comm_pct = comm_f64 / value_f64;
            (1.0 - comm_pct * 100.0).max(0.0) * criteria.cost_weight
        };
        
        // Latency score
        let latency_score = if broker.api_latency_ms <= criteria.max_latency_ms {
            let latency_factor = 1.0 - (broker.api_latency_ms as f64 / criteria.max_latency_ms as f64);
            latency_factor * criteria.latency_weight
        } else {
            0.0
        };
        
        // Reliability score
        let reliability_score = broker.reliability_score * criteria.reliability_weight;
        
        // Financing score (lower rate = better)
        let rate_f64: f64 = financing_rate.try_into().unwrap_or(10.0);
        let financing_score = ((15.0 - rate_f64) / 15.0).max(0.0) * criteria.financing_weight;
        
        let total_score = cost_score + latency_score + reliability_score + financing_score;
        
        BrokerScore {
            broker_id: broker.id.clone(),
            broker_name: broker.name.clone(),
            total_score,
            cost_score,
            latency_score,
            reliability_score,
            financing_score,
            estimated_commission: commission,
            estimated_financing_rate: financing_rate,
        }
    }
    
    /// Check if broker supports symbol
    fn broker_supports_symbol(&self, broker: &PrimeBroker, symbol: &str) -> bool {
        // Simplified - would check actual symbol availability
        !symbol.is_empty() && broker.supports_margin
    }
    
    /// Get cheapest broker for financing
    pub fn cheapest_for_financing(&self, side: OrderSide) -> Option<&PrimeBroker> {
        self.brokers.iter()
            .min_by_key(|b| {
                let rate = match side {
                    OrderSide::Buy => b.financing_rates.long_rate(),
                    OrderSide::Sell => b.financing_rates.short_rate(),
                };
                let rate_f64: f64 = rate.try_into().unwrap_or(999.0);
                (rate_f64 * 1000.0) as i64
            })
    }
    
    /// Get fastest broker
    pub fn fastest_broker(&self) -> Option<&PrimeBroker> {
        self.brokers.iter()
            .min_by_key(|b| b.api_latency_ms)
    }
    
    /// Get most reliable broker
    pub fn most_reliable(&self) -> Option<&PrimeBroker> {
        self.brokers.iter()
            .max_by(|a, b| a.reliability_score.partial_cmp(&b.reliability_score).unwrap())
    }
    
    /// Rank all brokers by criteria
    pub fn rank_brokers(&self, criteria: RoutingCriteria) -> Vec<BrokerScore> {
        let mut scores: Vec<_> = self.brokers.iter()
            .map(|broker| self.score_broker(broker, "", Decimal::ONE, OrderSide::Buy, &criteria))
            .collect();
        
        scores.sort_by(|a, b| b.total_score.partial_cmp(&a.total_score).unwrap());
        scores
    }
}

impl Default for PrimeBrokerRouter {
    fn default() -> Self {
        Self::new()
    }
}

/// Routing criteria
#[derive(Debug, Clone)]
pub struct RoutingCriteria {
    pub cost_weight: f64,
    pub latency_weight: f64,
    pub reliability_weight: f64,
    pub financing_weight: f64,
    pub max_latency_ms: u64,
    pub max_commission: Option<Decimal>,
    pub prefer_short_locate: bool,
}

impl RoutingCriteria {
    /// Optimize for lowest cost
    pub fn lowest_cost() -> Self {
        Self {
            cost_weight: 0.5,
            latency_weight: 0.1,
            reliability_weight: 0.2,
            financing_weight: 0.2,
            max_latency_ms: 500,
            max_commission: None,
            prefer_short_locate: false,
        }
    }
    
    /// Optimize for lowest latency
    pub fn lowest_latency() -> Self {
        Self {
            cost_weight: 0.1,
            latency_weight: 0.5,
            reliability_weight: 0.2,
            financing_weight: 0.2,
            max_latency_ms: 100,
            max_commission: None,
            prefer_short_locate: false,
        }
    }
    
    /// Optimize for overnight holds (financing cost matters)
    pub fn overnight_hold() -> Self {
        Self {
            cost_weight: 0.2,
            latency_weight: 0.1,
            reliability_weight: 0.2,
            financing_weight: 0.5,
            max_latency_ms: 500,
            max_commission: None,
            prefer_short_locate: false,
        }
    }
    
    /// Balanced criteria
    pub fn balanced() -> Self {
        Self {
            cost_weight: 0.25,
            latency_weight: 0.25,
            reliability_weight: 0.25,
            financing_weight: 0.25,
            max_latency_ms: 200,
            max_commission: None,
            prefer_short_locate: false,
        }
    }
}

/// Broker score
#[derive(Debug, Clone)]
pub struct BrokerScore {
    pub broker_id: BrokerId,
    pub broker_name: String,
    pub total_score: f64,
    pub cost_score: f64,
    pub latency_score: f64,
    pub reliability_score: f64,
    pub financing_score: f64,
    pub estimated_commission: Decimal,
    pub estimated_financing_rate: Decimal,
}

/// Smart order router with ML
pub struct SmartOrderRouter {
    base_router: PrimeBrokerRouter,
    ml_model: Option<MLRoutingModel>,
    historical_data: Vec<RoutingDecision>,
}

#[derive(Debug, Clone)]
struct MLRoutingModel;

#[derive(Debug, Clone)]
pub struct RoutingDecision {
    pub broker_id: BrokerId,
    pub symbol: String,
    pub quantity: Decimal,
    pub actual_slippage: Decimal,
    pub actual_latency_ms: u64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl SmartOrderRouter {
    pub fn new(router: PrimeBrokerRouter) -> Self {
        Self {
            base_router: router,
            ml_model: None,
            historical_data: Vec::new(),
        }
    }
    
    /// Route with ML enhancement
    pub fn smart_route(
        &self,
        symbol: &str,
        quantity: Decimal,
        side: OrderSide,
        urgency: OrderUrgency,
    ) -> Option<BrokerScore> {
        // Adjust criteria based on urgency
        let criteria = match urgency {
            OrderUrgency::High => RoutingCriteria::lowest_latency(),
            OrderUrgency::Low => RoutingCriteria::overnight_hold(),
            OrderUrgency::Medium => RoutingCriteria::balanced(),
        };
        
        // Could use ML model here to predict best broker
        self.base_router.select_best_broker(symbol, quantity, side, criteria)
    }
}

/// Order urgency
#[derive(Debug, Clone, Copy)]
pub enum OrderUrgency {
    Low,     // Can wait for best price
    Medium,  // Normal execution
    High,    // Immediate execution needed
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::{FinancingRates, CommissionStructure, MarginRequirements};

    fn create_test_broker(id: &str, latency: u64, reliability: f64) -> PrimeBroker {
        PrimeBroker {
            id: BrokerId(id.to_string()),
            name: id.to_string(),
            tier: super::super::BrokerTier::Tier2,
            financing_rates: FinancingRates::ibkr(),
            commission_structure: CommissionStructure::per_share(Decimal::try_from(0.005).unwrap(), Decimal::ONE),
            margin_requirements: MarginRequirements::standard(),
            api_latency_ms: latency,
            reliability_score: reliability,
            min_account_size: Decimal::ZERO,
            supports_shorting: true,
            supports_margin: true,
            supports_options: true,
            supports_futures: false,
        }
    }

    #[test]
    fn test_broker_selection() {
        let mut router = PrimeBrokerRouter::new();
        
        router.add_broker(create_test_broker("FAST", 20, 0.95));
        router.add_broker(create_test_broker("CHEAP", 100, 0.90));
        router.add_broker(create_test_broker("RELIABLE", 50, 0.99));
        
        let criteria = RoutingCriteria::lowest_latency();
        let best = router.select_best_broker("AAPL", Decimal::from(100), OrderSide::Buy, criteria);
        
        assert!(best.is_some());
        // Should pick FAST for lowest latency
        assert_eq!(best.unwrap().broker_id.0, "FAST");
    }

    #[test]
    fn test_cheapest_financing() {
        let mut router = PrimeBrokerRouter::new();
        
        let mut cheap = create_test_broker("CHEAP", 100, 0.90);
        cheap.financing_rates = FinancingRates::tier1(); // Lower rates
        
        let mut expensive = create_test_broker("EXPENSIVE", 50, 0.95);
        expensive.financing_rates = FinancingRates::retail(); // Higher rates
        
        router.add_broker(cheap);
        router.add_broker(expensive);
        
        let cheapest = router.cheapest_for_financing(OrderSide::Buy);
        
        assert!(cheapest.is_some());
        assert_eq!(cheapest.unwrap().id.0, "CHEAP");
    }

    #[test]
    fn test_broker_ranking() {
        let mut router = PrimeBrokerRouter::new();
        
        router.add_broker(create_test_broker("A", 20, 0.95));
        router.add_broker(create_test_broker("B", 100, 0.90));
        router.add_broker(create_test_broker("C", 50, 0.99));
        
        let rankings = router.rank_brokers(RoutingCriteria::balanced());
        
        assert_eq!(rankings.len(), 3);
        // Highest score first
        assert!(rankings[0].total_score >= rankings[1].total_score);
    }
}
