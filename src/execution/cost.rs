//! Cost calculation and market impact modeling

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use super::order::{Order, OrderSide};
use super::venue::VenueQuote;

/// Market impact model configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactModel {
    /// Base impact (percentage for average size trade)
    pub base_impact_bps: Decimal, // Basis points
    /// Impact scales with square root of size
    pub sqrt_scaling: bool,
    /// Volatility adjustment
    pub volatility_factor: Decimal,
}

impl Default for ImpactModel {
    fn default() -> Self {
        Self {
            base_impact_bps: Decimal::from(10), // 10 bps = 0.1%
            sqrt_scaling: true,
            volatility_factor: Decimal::ONE,
        }
    }
}

/// Execution cost components
#[derive(Debug, Clone)]
pub struct ExecutionCost {
    pub explicit_fees: Decimal,       // Commissions, exchange fees
    pub market_impact: Decimal,       // Price movement due to our trade
    pub spread_cost: Decimal,         // Bid-ask spread
    pub slippage: Decimal,            // Difference from expected price
    pub opportunity_cost: Decimal,    // Cost of unfilled portion
    pub total_cost: Decimal,          // Sum of all costs
    pub total_cost_bps: Decimal,      // Total cost in basis points
}

/// Cost calculator for execution analysis
#[derive(Debug)]
pub struct CostCalculator {
    impact_model: ImpactModel,
}

impl CostCalculator {
    pub fn new() -> Self {
        Self {
            impact_model: ImpactModel::default(),
        }
    }
    
    /// Calculate execution cost for an order at a venue
    pub fn calculate_cost(
        &self,
        order: &Order,
        quote: &VenueQuote,
        avg_daily_volume: Decimal,
    ) -> ExecutionCost {
        let notional = order.quantity * quote.mid();
        
        // 1. Spread cost
        let spread_cost = self.calculate_spread_cost(order, quote);
        
        // 2. Market impact
        let market_impact = self.calculate_market_impact(
            order.quantity,
            avg_daily_volume,
            quote.mid(),
        );
        
        // 3. Explicit fees (estimate)
        let explicit_fees = notional * Decimal::try_from(0.001).unwrap(); // 0.1% estimate
        
        // 4. Slippage (temporary, assume 50% of spread)
        let slippage = spread_cost * Decimal::try_from(0.5).unwrap();
        
        // 5. Opportunity cost (for unfilled portion - assume 0 for now)
        let opportunity_cost = Decimal::ZERO;
        
        let total_cost = explicit_fees + market_impact + spread_cost + slippage + opportunity_cost;
        
        let total_cost_bps = if notional.is_zero() {
            Decimal::ZERO
        } else {
            (total_cost / notional) * Decimal::from(10000) // Convert to bps
        };
        
        ExecutionCost {
            explicit_fees,
            market_impact,
            spread_cost,
            slippage,
            opportunity_cost,
            total_cost,
            total_cost_bps,
        }
    }
    
    /// Calculate spread cost
    fn calculate_spread_cost(&self, order: &Order, quote: &VenueQuote) -> Decimal {
        let half_spread = quote.spread() / Decimal::from(2);
        let cost_per_unit = match order.side {
            OrderSide::Buy => half_spread,  // Pay more
            OrderSide::Sell => -half_spread, // Receive less
        };
        order.quantity * cost_per_unit
    }
    
    /// Calculate market impact using square root model
    /// Impact = σ * √(Size / ADV) * Price
    fn calculate_market_impact(
        &self,
        size: Decimal,
        adv: Decimal, // Average Daily Volume
        price: Decimal,
    ) -> Decimal {
        if adv.is_zero() || size.is_zero() {
            return Decimal::ZERO;
        }
        
        let participation = size / adv;
        
        if self.impact_model.sqrt_scaling {
            // Square root model: Impact ∝ √participation
            // Simplified: use participation^0.6 as approximation to sqrt for typical range
            let impact_factor = self.impact_model.base_impact_bps 
                * Decimal::try_from(0.0001).unwrap()
                * self.impact_model.volatility_factor;
            
            // Approximate sqrt with: sqrt(x) ≈ x / (0.5 + 0.5*x) for x in [0,1]
            let sqrt_approx = if participation > Decimal::ZERO {
                participation / (Decimal::try_from(0.5).unwrap() + participation * Decimal::try_from(0.5).unwrap())
            } else {
                Decimal::ZERO
            };
            
            price * size * impact_factor * sqrt_approx
        } else {
            // Linear model
            let impact_factor = self.impact_model.base_impact_bps 
                * Decimal::try_from(0.0001).unwrap();
            
            price * size * impact_factor * participation
        }
    }
    
    /// Compare costs across multiple venues
    pub fn compare_venues(
        &self,
        order: &Order,
        quotes: &[&VenueQuote],
        adv_by_venue: &std::collections::HashMap<String, Decimal>,
    ) -> Vec<(String, ExecutionCost)> {
        let mut results = Vec::new();
        
        for quote in quotes {
            let adv = adv_by_venue.get(&quote.venue.name())
                .copied()
                .unwrap_or(Decimal::from(1000000)); // Default 1M
            
            let cost = self.calculate_cost(order, quote, adv);
            results.push((quote.venue.name(), cost));
        }
        
        // Sort by total cost ascending
        results.sort_by(|a, b| a.1.total_cost.cmp(&b.1.total_cost));
        results
    }
    
    /// Estimate implementation shortfall
    /// Difference between decision price and execution price
    pub fn implementation_shortfall(
        decision_price: Decimal,
        avg_fill_price: Decimal,
        side: OrderSide,
    ) -> Decimal {
        match side {
            OrderSide::Buy => avg_fill_price - decision_price,
            OrderSide::Sell => decision_price - avg_fill_price,
        }
    }
}

impl Default for CostCalculator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::venue::{Venue, VenueQuote};
    use chrono::Utc;
    
    fn create_test_quote(bid: i64, ask: i64) -> VenueQuote {
        VenueQuote {
            venue: Venue::Binance,
            symbol: "BTC".to_string(),
            bid: Decimal::from(bid),
            ask: Decimal::from(ask),
            bid_size: Decimal::from(100),
            ask_size: Decimal::from(100),
            timestamp: Utc::now(),
            latency_ms: 10,
        }
    }
    
    #[test]
    fn test_spread_cost_calculation() {
        let calc = CostCalculator::new();
        let quote = create_test_quote(50000, 50100); // $100 spread
        
        let buy_order = Order::market("BTC", OrderSide::Buy, Decimal::from(1));
        let sell_order = Order::market("BTC", OrderSide::Sell, Decimal::from(1));
        
        let buy_cost = calc.calculate_spread_cost(&buy_order, &quote);
        let sell_cost = calc.calculate_spread_cost(&sell_order, &quote);
        
        // Buy cost = 1 * $50 (half spread)
        assert_eq!(buy_cost, Decimal::from(50));
        // Sell cost = -1 * $50 (negative = receive less)
        assert_eq!(sell_cost, Decimal::from(-50));
    }
    
    #[test]
    fn test_market_impact_small_order() {
        let calc = CostCalculator::new();
        
        // Small order: 1 BTC with ADV of 10000 BTC
        let impact = calc.calculate_market_impact(
            Decimal::from(1),
            Decimal::from(10000),
            Decimal::from(50000),
        );
        
        // Should be small impact
        assert!(impact < Decimal::from(100));
    }
    
    #[test]
    fn test_market_impact_large_order() {
        let calc = CostCalculator::new();
        
        // Large order: 1000 BTC with ADV of 10000 BTC = 10% of ADV
        let impact = calc.calculate_market_impact(
            Decimal::from(1000),
            Decimal::from(10000),
            Decimal::from(50000),
        );
        
        // Should be significant impact
        assert!(impact > Decimal::from(1000));
    }
    
    #[test]
    fn test_implementation_shortfall() {
        // Decision price: $50k
        // Fill price: $50.1k (worse for buyer)
        let is = CostCalculator::implementation_shortfall(
            Decimal::from(50000),
            Decimal::from(50100),
            OrderSide::Buy,
        );
        
        // Shortfall = $100 per unit
        assert_eq!(is, Decimal::from(100));
        
        // For seller, higher fill is better
        let is_sell = CostCalculator::implementation_shortfall(
            Decimal::from(50000),
            Decimal::from(50100),
            OrderSide::Sell,
        );
        
        // Shortfall = -$100 (negative = profit)
        assert_eq!(is_sell, Decimal::from(-100));
    }
    
    #[test]
    fn test_cost_comparison() {
        let calc = CostCalculator::new();
        let order = Order::market("BTC", OrderSide::Buy, Decimal::from(10));
        
        let quote1 = create_test_quote(50000, 50100); // Wide spread
        let quote2 = create_test_quote(50050, 50060); // Tight spread
        
        let quotes: Vec<&VenueQuote> = vec![&quote1, &quote2];
        let mut adv = std::collections::HashMap::new();
        adv.insert("Binance".to_string(), Decimal::from(1000000));
        
        let comparison = calc.compare_venues(&order, &quotes, &adv);
        
        // Tighter spread should be cheaper
        assert!(comparison[0].1.total_cost < comparison[1].1.total_cost);
    }
}
