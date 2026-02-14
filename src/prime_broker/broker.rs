//! Prime Broker Definitions

use super::financing::FinancingRates;
use serde::{Deserialize, Serialize};
use rust_decimal::Decimal;

/// Broker identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BrokerId(pub String);

/// Broker tier classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BrokerTier {
    Tier1, // Global banks (Goldman, Morgan Stanley, etc.)
    Tier2, // Specialized prime brokers (IBKR, Schwab)
    Tier3, // DMA brokers (Lightspeed, etc.)
}

/// Prime broker configuration
#[derive(Debug, Clone)]
pub struct PrimeBroker {
    pub id: BrokerId,
    pub name: String,
    pub tier: BrokerTier,
    pub financing_rates: FinancingRates,
    pub commission_structure: CommissionStructure,
    pub margin_requirements: MarginRequirements,
    pub api_latency_ms: u64,
    pub reliability_score: f64, // 0.0 - 1.0
    pub min_account_size: Decimal,
    pub supports_shorting: bool,
    pub supports_margin: bool,
    pub supports_options: bool,
    pub supports_futures: bool,
}

impl PrimeBroker {
    /// Calculate commission for trade
    pub fn calculate_commission(&self, quantity: Decimal, price: Decimal) -> Decimal {
        self.commission_structure.calculate(quantity, price)
    }
    
    /// Check if can trade given quantity
    pub fn can_trade(&self, quantity: Decimal, price: Decimal, account_value: Decimal) -> bool {
        if !self.supports_margin && quantity * price > account_value {
            return false;
        }
        true
    }
    
    // Factory methods for major brokers
    
    pub fn interactive_brokers() -> Self {
        Self {
            id: BrokerId("IBKR".to_string()),
            name: "Interactive Brokers".to_string(),
            tier: BrokerTier::Tier2,
            financing_rates: FinancingRates::ibkr(),
            commission_structure: CommissionStructure::per_share(Decimal::try_from(0.0035).unwrap(), Decimal::from(1)),
            margin_requirements: MarginRequirements::standard(),
            api_latency_ms: 50,
            reliability_score: 0.98,
            min_account_size: Decimal::ZERO,
            supports_shorting: true,
            supports_margin: true,
            supports_options: true,
            supports_futures: true,
        }
    }
    
    pub fn goldman_sachs() -> Self {
        Self {
            id: BrokerId("GS".to_string()),
            name: "Goldman Sachs".to_string(),
            tier: BrokerTier::Tier1,
            financing_rates: FinancingRates::tier1(),
            commission_structure: CommissionStructure::custom(),
            margin_requirements: MarginRequirements::institutional(),
            api_latency_ms: 20,
            reliability_score: 0.995,
            min_account_size: Decimal::from(10_000_000),
            supports_shorting: true,
            supports_margin: true,
            supports_options: true,
            supports_futures: true,
        }
    }
    
    pub fn morgan_stanley() -> Self {
        Self {
            id: BrokerId("MS".to_string()),
            name: "Morgan Stanley".to_string(),
            tier: BrokerTier::Tier1,
            financing_rates: FinancingRates::tier1(),
            commission_structure: CommissionStructure::custom(),
            margin_requirements: MarginRequirements::institutional(),
            api_latency_ms: 25,
            reliability_score: 0.995,
            min_account_size: Decimal::from(10_000_000),
            supports_shorting: true,
            supports_margin: true,
            supports_options: true,
            supports_futures: true,
        }
    }
    
    pub fn schwab() -> Self {
        Self {
            id: BrokerId("SCHWAB".to_string()),
            name: "Charles Schwab".to_string(),
            tier: BrokerTier::Tier2,
            financing_rates: FinancingRates::retail(),
            commission_structure: CommissionStructure::zero_commission(),
            margin_requirements: MarginRequirements::standard(),
            api_latency_ms: 100,
            reliability_score: 0.97,
            min_account_size: Decimal::ZERO,
            supports_shorting: true,
            supports_margin: true,
            supports_options: true,
            supports_futures: false,
        }
    }
    
    pub fn lightspeed() -> Self {
        Self {
            id: BrokerId("LIGHTSPEED".to_string()),
            name: "Lightspeed".to_string(),
            tier: BrokerTier::Tier3,
            financing_rates: FinancingRates::tier3(),
            commission_structure: CommissionStructure::per_share(Decimal::try_from(0.001).unwrap(), Decimal::from(1)),
            margin_requirements: MarginRequirements::day_trading(),
            api_latency_ms: 30,
            reliability_score: 0.95,
            min_account_size: Decimal::from(25_000),
            supports_shorting: true,
            supports_margin: true,
            supports_options: true,
            supports_futures: false,
        }
    }
}

/// Commission structure
#[derive(Debug, Clone)]
pub enum CommissionStructure {
    PerShare { rate: Decimal, min: Decimal, max: Option<Decimal> },
    PerTrade { flat_fee: Decimal },
    Tiered { tiers: Vec<TieredCommission> },
    ZeroCommission,
    Custom, // Negotiated rates
}

#[derive(Debug, Clone)]
pub struct TieredCommission {
    pub volume_threshold: Decimal,
    pub rate: Decimal,
}

impl CommissionStructure {
    pub fn per_share(rate: Decimal, min: Decimal) -> Self {
        Self::PerShare { rate, min, max: None }
    }
    
    pub fn zero_commission() -> Self {
        Self::ZeroCommission
    }
    
    pub fn custom() -> Self {
        Self::Custom
    }
    
    pub fn calculate(&self, quantity: Decimal, _price: Decimal) -> Decimal {
        match self {
            Self::PerShare { rate, min, max } => {
                let commission = quantity * rate;
                let commission = commission.max(*min);
                if let Some(max_comm) = max {
                    commission.min(*max_comm)
                } else {
                    commission
                }
            }
            Self::PerTrade { flat_fee } => *flat_fee,
            Self::ZeroCommission => Decimal::ZERO,
            Self::Custom => Decimal::ZERO, // Would use negotiated rate
            Self::Tiered { tiers } => {
                // Find applicable tier
                let rate = tiers
                    .iter()
                    .find(|t| quantity >= t.volume_threshold)
                    .map(|t| t.rate)
                    .unwrap_or_else(|| tiers.last().map(|t| t.rate).unwrap_or(Decimal::ZERO));
                quantity * rate
            }
        }
    }
}

/// Margin requirements
#[derive(Debug, Clone)]
pub struct MarginRequirements {
    pub initial_margin_pct: Decimal,    // 50% for standard
    pub maintenance_margin_pct: Decimal, // 25% for standard
    pub day_trading_margin_pct: Option<Decimal>, // 25% for day trading
    pub short_margin_pct: Decimal,      // 150% for shorts
    pub portfolio_margin: bool,         // Portfolio margin available
}

impl MarginRequirements {
    pub fn standard() -> Self {
        Self {
            initial_margin_pct: Decimal::from(50),      // 50%
            maintenance_margin_pct: Decimal::from(25),  // 25%
            day_trading_margin_pct: Some(Decimal::from(25)),
            short_margin_pct: Decimal::from(150),       // 150%
            portfolio_margin: false,
        }
    }
    
    pub fn institutional() -> Self {
        Self {
            initial_margin_pct: Decimal::from(50),
            maintenance_margin_pct: Decimal::from(25),
            day_trading_margin_pct: Some(Decimal::from(25)),
            short_margin_pct: Decimal::from(150),
            portfolio_margin: true,
        }
    }
    
    pub fn day_trading() -> Self {
        Self {
            initial_margin_pct: Decimal::from(25),
            maintenance_margin_pct: Decimal::from(25),
            day_trading_margin_pct: Some(Decimal::from(25)),
            short_margin_pct: Decimal::from(150),
            portfolio_margin: false,
        }
    }
    
    /// Calculate required margin for position
    pub fn calculate_margin(&self, market_value: Decimal, is_short: bool) -> Decimal {
        if is_short {
            market_value * self.short_margin_pct / Decimal::from(100)
        } else {
            market_value * self.initial_margin_pct / Decimal::from(100)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ibkr_creation() {
        let ibkr = PrimeBroker::interactive_brokers();
        assert_eq!(ibkr.id.0, "IBKR");
        assert_eq!(ibkr.tier, BrokerTier::Tier2);
        assert!(ibkr.supports_margin);
    }

    #[test]
    fn test_commission_calculation() {
        let comm = CommissionStructure::per_share(Decimal::try_from(0.0035).unwrap(), Decimal::from(1));
        
        // 1000 shares @ $0.0035 = $3.50
        let cost = comm.calculate(Decimal::from(1000), Decimal::from(150));
        assert_eq!(cost, Decimal::try_from(3.5).unwrap());
        
        // 100 shares @ $0.0035 = $0.35, but min is $1
        let cost = comm.calculate(Decimal::from(100), Decimal::from(150));
        assert_eq!(cost, Decimal::from(1));
    }

    #[test]
    fn test_margin_calculation() {
        let margin = MarginRequirements::standard();
        
        // $100K position, 50% initial margin = $50K
        let required = margin.calculate_margin(Decimal::from(100_000), false);
        assert_eq!(required, Decimal::from(50_000));
        
        // Short $100K, 150% margin = $150K
        let required = margin.calculate_margin(Decimal::from(100_000), true);
        assert_eq!(required, Decimal::from(150_000));
    }

    #[test]
    fn test_zero_commission() {
        let comm = CommissionStructure::zero_commission();
        let cost = comm.calculate(Decimal::from(1000), Decimal::from(150));
        assert_eq!(cost, Decimal::ZERO);
    }
}
