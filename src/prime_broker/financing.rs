//! Financing Rate Calculation

use super::Position;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Financing rates configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinancingRates {
    pub base_rate: Decimal,        // Benchmark rate (SOFR, etc.)
    pub long_spread: Decimal,      // Spread for long positions
    pub short_spread: Decimal,     // Spread for short positions
    pub margin_spread: Decimal,    // Spread for margin loans
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TieredRate {
    pub threshold: Decimal,        // Volume threshold
    pub discount_bps: i32,         // Discount in basis points
}

impl FinancingRates {
    /// IBKR rates (competitive retail)
    pub fn ibkr() -> Self {
        Self {
            base_rate: Decimal::try_from(5.33).unwrap(), // SOFR
            long_spread: Decimal::try_from(1.5).unwrap(),
            short_spread: Decimal::try_from(1.5).unwrap(),
            margin_spread: Decimal::try_from(1.5).unwrap(),
        }
    }
    
    /// Tier 1 rates (institutional)
    pub fn tier1() -> Self {
        Self {
            base_rate: Decimal::try_from(5.33).unwrap(),
            long_spread: Decimal::try_from(0.5).unwrap(),
            short_spread: Decimal::try_from(0.5).unwrap(),
            margin_spread: Decimal::try_from(0.5).unwrap(),
        }
    }
    
    /// Retail rates
    pub fn retail() -> Self {
        Self {
            base_rate: Decimal::try_from(5.33).unwrap(),
            long_spread: Decimal::try_from(2.0).unwrap(),
            short_spread: Decimal::try_from(2.0).unwrap(),
            margin_spread: Decimal::try_from(2.0).unwrap(),
        }
    }
    
    /// Tier 3 rates (higher for smaller brokers)
    pub fn tier3() -> Self {
        Self {
            base_rate: Decimal::try_from(5.33).unwrap(),
            long_spread: Decimal::try_from(2.5).unwrap(),
            short_spread: Decimal::try_from(2.5).unwrap(),
            margin_spread: Decimal::try_from(2.5).unwrap(),
        }
    }
    
    /// Get effective long rate
    pub fn long_rate(&self) -> Decimal {
        self.base_rate + self.long_spread
    }
    
    /// Get effective short rate
    pub fn short_rate(&self) -> Decimal {
        self.base_rate + self.short_spread
    }
    
    /// Get effective margin rate
    pub fn margin_rate(&self) -> Decimal {
        self.base_rate + self.margin_spread
    }
}

/// Financing calculator
pub struct FinancingCalculator<'a> {
    rates: &'a FinancingRates,
}

impl<'a> FinancingCalculator<'a> {
    pub fn new(rates: &'a FinancingRates) -> Self {
        Self { rates }
    }
    
    /// Calculate overnight financing cost for position
    pub fn calculate_overnight_cost(&self, position: &Position) -> OvernightCost {
        let is_short = position.quantity < Decimal::ZERO;
        let market_value = position.market_value.abs();
        
        // Annual rate
        let annual_rate = if is_short {
            self.rates.short_rate()
        } else {
            self.rates.long_rate()
        };
        
        // Daily rate (annual / 360 for money market convention)
        let daily_rate = annual_rate / Decimal::from(360);
        let daily_rate_pct = daily_rate / Decimal::from(100);
        
        // Daily cost
        let daily_cost = market_value * daily_rate_pct;
        
        // Annual projection
        let annual_cost = market_value * (annual_rate / Decimal::from(100));
        
        OvernightCost {
            symbol: position.symbol.clone(),
            market_value,
            is_short,
            annual_rate,
            daily_rate,
            daily_cost,
            annual_cost,
            days_held: 1,
            total_cost: daily_cost,
        }
    }
    
    /// Calculate financing for multiple days
    pub fn calculate_multi_day_cost(
        &self,
        position: &Position,
        days: u32,
    ) -> OvernightCost {
        let mut cost = self.calculate_overnight_cost(position);
        cost.days_held = days;
        cost.total_cost = cost.daily_cost * Decimal::from(days);
        cost
    }
    
    /// Calculate blended rate for portfolio
    pub fn calculate_portfolio_rate(
        &self,
        long_exposure: Decimal,
        short_exposure: Decimal,
    ) -> PortfolioFinancingRate {
        let total_exposure = long_exposure + short_exposure;
        
        if total_exposure.is_zero() {
            return PortfolioFinancingRate::default();
        }
        
        let long_weight: f64 = (long_exposure / total_exposure).try_into().unwrap_or(0.0);
        let short_weight: f64 = (short_exposure / total_exposure).try_into().unwrap_or(0.0);
        
        let long_rate_f64: f64 = self.rates.long_rate().try_into().unwrap_or(0.0);
        let short_rate_f64: f64 = self.rates.short_rate().try_into().unwrap_or(0.0);
        
        let blended_rate = long_weight * long_rate_f64 + short_weight * short_rate_f64;
        
        PortfolioFinancingRate {
            long_rate: self.rates.long_rate(),
            short_rate: self.rates.short_rate(),
            blended_rate: Decimal::try_from(blended_rate).unwrap_or(Decimal::ZERO),
            long_exposure,
            short_exposure,
            net_exposure: long_exposure - short_exposure,
        }
    }
    
    /// Compare with another financing rate
    pub fn compare_rates(&self, other: &FinancingRates) -> RateComparison {
        let self_long: f64 = self.rates.long_rate().try_into().unwrap_or(0.0);
        let other_long: f64 = other.long_rate().try_into().unwrap_or(0.0);
        
        RateComparison {
            long_diff_bps: ((self_long - other_long) * 100.0) as i32,
            short_diff_bps: ((self.rates.short_rate().try_into().unwrap_or(0.0) 
                - other.short_rate().try_into().unwrap_or(0.0)) * 100.0) as i32,
            cheaper_for_long: if self_long < other_long { "self" } else { "other" },
        }
    }
}

/// Overnight financing cost
#[derive(Debug, Clone)]
pub struct OvernightCost {
    pub symbol: String,
    pub market_value: Decimal,
    pub is_short: bool,
    pub annual_rate: Decimal,
    pub daily_rate: Decimal,
    pub daily_cost: Decimal,
    pub annual_cost: Decimal,
    pub days_held: u32,
    pub total_cost: Decimal,
}

impl OvernightCost {
    /// Project cost for additional days
    pub fn project_days(&self, additional_days: u32) -> Decimal {
        self.daily_cost * Decimal::from(additional_days)
    }
    
    /// Annualized cost as percentage of position
    pub fn annualized_pct(&self) -> f64 {
        if self.market_value.is_zero() {
            return 0.0;
        }
        let annual: f64 = self.annual_cost.try_into().unwrap_or(0.0);
        let value: f64 = self.market_value.try_into().unwrap_or(1.0);
        (annual / value) * 100.0
    }
}

/// Portfolio financing rate summary
#[derive(Debug, Clone, Default)]
pub struct PortfolioFinancingRate {
    pub long_rate: Decimal,
    pub short_rate: Decimal,
    pub blended_rate: Decimal,
    pub long_exposure: Decimal,
    pub short_exposure: Decimal,
    pub net_exposure: Decimal,
}

/// Rate comparison between two brokers
#[derive(Debug, Clone)]
pub struct RateComparison {
    pub long_diff_bps: i32,
    pub short_diff_bps: i32,
    pub cheaper_for_long: &'static str,
}

/// Financing cost tracker for historical analysis
pub struct FinancingCostTracker {
    daily_costs: Vec<DailyFinancingRecord>,
}

#[derive(Debug, Clone)]
pub struct DailyFinancingRecord {
    pub date: DateTime<Utc>,
    pub total_cost: Decimal,
    pub long_cost: Decimal,
    pub short_cost: Decimal,
    pub positions_count: usize,
}

impl FinancingCostTracker {
    pub fn new() -> Self {
        Self {
            daily_costs: Vec::new(),
        }
    }
    
    pub fn record_day(&mut self, record: DailyFinancingRecord) {
        self.daily_costs.push(record);
    }
    
    /// Get total financing costs for period
    pub fn get_period_costs(&self, days: usize) -> Decimal {
        self.daily_costs
            .iter()
            .rev()
            .take(days)
            .map(|r| r.total_cost)
            .sum()
    }
    
    /// Calculate average daily cost
    pub fn average_daily_cost(&self) -> Decimal {
        if self.daily_costs.is_empty() {
            return Decimal::ZERO;
        }
        let total: Decimal = self.daily_costs.iter().map(|r| r.total_cost).sum();
        total / Decimal::from(self.daily_costs.len() as i64)
    }
    
    /// Annual projection based on historical data
    pub fn annual_projection(&self) -> Decimal {
        self.average_daily_cost() * Decimal::from(365)
    }
}

impl Default for FinancingCostTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn test_overnight_cost_calculation() {
        let rates = FinancingRates::ibkr();
        let calculator = FinancingCalculator::new(&rates);
        
        let position = Position {
            symbol: "AAPL".to_string(),
            quantity: Decimal::from(100),
            avg_price: Decimal::from(150),
            market_price: Decimal::from(150),
            market_value: Decimal::from(15_000),
        };
        
        let cost = calculator.calculate_overnight_cost(&position);
        
        assert!(cost.daily_cost > Decimal::ZERO);
        assert_eq!(cost.market_value, Decimal::from(15_000));
        assert!(!cost.is_short);
        
        // Should be around $1-2 for $15K at ~6.83% annually
        let daily_f64: f64 = cost.daily_cost.try_into().unwrap();
        assert!(daily_f64 > 0.5 && daily_f64 < 5.0);
    }

    #[test]
    fn test_short_financing() {
        let rates = FinancingRates::ibkr();
        let calculator = FinancingCalculator::new(&rates);
        
        let position = Position {
            symbol: "AAPL".to_string(),
            quantity: Decimal::from(-100), // Short
            avg_price: Decimal::from(150),
            market_price: Decimal::from(150),
            market_value: Decimal::from(-15_000),
        };
        
        let cost = calculator.calculate_overnight_cost(&position);
        
        assert!(cost.is_short);
        assert!(cost.daily_cost > Decimal::ZERO);
    }

    #[test]
    fn test_rate_comparison() {
        let ibkr = FinancingRates::ibkr();
        let retail = FinancingRates::retail();
        
        let calculator = FinancingCalculator::new(&ibkr);
        let comparison = calculator.compare_rates(&retail);
        
        // IBKR should be cheaper
        assert!(comparison.long_diff_bps < 0);
    }

    #[test]
    fn test_cost_tracker() {
        let mut tracker = FinancingCostTracker::new();
        
        for i in 0..10 {
            tracker.record_day(DailyFinancingRecord {
                date: Utc::now() - Duration::days(i as i64),
                total_cost: Decimal::from(10),
                long_cost: Decimal::from(5),
                short_cost: Decimal::from(5),
                positions_count: 5,
            });
        }
        
        assert_eq!(tracker.average_daily_cost(), Decimal::from(10));
        assert_eq!(tracker.get_period_costs(5), Decimal::from(50));
    }
}
