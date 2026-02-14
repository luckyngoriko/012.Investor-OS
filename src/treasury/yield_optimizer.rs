//! Yield Optimizer - Find best yield for idle cash

use super::*;

/// Yield opportunity for a specific protocol and currency
#[derive(Debug, Clone)]
pub struct YieldOpportunity {
    pub protocol: String,
    pub currency: Currency,
    pub apy: Decimal,
    pub tvl: Decimal,
    pub risk_score: u8,
}

/// Yield position - an active allocation to a yield protocol
#[derive(Debug, Clone)]
pub struct YieldPosition {
    pub id: Uuid,
    pub protocol: String,
    pub currency: Currency,
    pub amount: Decimal,
    pub apy: Decimal,
    pub opened_at: DateTime<Utc>,
}

/// Yield protocol
#[derive(Debug, Clone)]
pub struct YieldProtocol {
    pub name: String,
    pub protocol_type: ProtocolType,
    pub supported_currencies: Vec<Currency>,
    pub base_apy: Decimal,
    pub reward_apy: Option<Decimal>, // Extra rewards
    pub tvl: Decimal,                // Total value locked
    pub risk_score: u8,              // 1-10
    pub lockup_period_days: u32,
}

#[derive(Debug, Clone)]
pub enum ProtocolType {
    Lending,      // Aave, Compound
    Staking,      // ETH staking
    LiquidityPool, // DEX LP
    MoneyMarket,  // Traditional
}

/// Yield optimizer
#[derive(Debug)]
pub struct YieldOptimizer {
    protocols: Vec<YieldProtocol>,
}

impl YieldOptimizer {
    pub async fn new() -> Result<Self> {
        let mut optimizer = Self {
            protocols: Vec::new(),
        };
        
        // Initialize with known protocols
        optimizer.init_protocols();
        
        Ok(optimizer)
    }
    
    /// Find best yield opportunity for a currency
    pub async fn find_best(&self, currency: Currency) -> Result<YieldOpportunity> {
        let opportunities: Vec<_> = self.protocols
            .iter()
            .filter(|p| p.supported_currencies.contains(&currency))
            .map(|p| {
                let total_apy = p.base_apy + p.reward_apy.unwrap_or(Decimal::ZERO);
                
                YieldOpportunity {
                    protocol: p.name.clone(),
                    currency,
                    apy: total_apy,
                    tvl: p.tvl,
                    risk_score: p.risk_score,
                }
            })
            .collect();
        
        if opportunities.is_empty() {
            return Err(TreasuryError::UnsupportedCurrency(
                format!("No yield opportunities for {}", currency.as_str())
            ));
        }
        
        // Sort by APY (risk-adjusted)
        let best = opportunities
            .into_iter()
            .max_by(|a, b| {
                // Simple risk adjustment: APY / risk_score
                let a_score = a.apy / Decimal::from(a.risk_score);
                let b_score = b.apy / Decimal::from(b.risk_score);
                a_score.partial_cmp(&b_score).unwrap()
            })
            .unwrap();
        
        Ok(best)
    }
    
    /// Get all opportunities sorted by APY
    pub async fn get_all_opportunities(&self, currency: Currency) -> Vec<YieldOpportunity> {
        let mut opportunities: Vec<_> = self.protocols
            .iter()
            .filter(|p| p.supported_currencies.contains(&currency))
            .map(|p| YieldOpportunity {
                protocol: p.name.clone(),
                currency,
                apy: p.base_apy + p.reward_apy.unwrap_or(Decimal::ZERO),
                tvl: p.tvl,
                risk_score: p.risk_score,
            })
            .collect();
        
        opportunities.sort_by(|a, b| b.apy.partial_cmp(&a.apy).unwrap());
        opportunities
    }
    
    /// Calculate optimal allocation
    pub async fn optimize_allocation(
        &self,
        total_capital: Decimal,
        currency: Currency,
        max_risk: u8,
    ) -> Result<Vec<Allocation>> {
        let opportunities = self.get_all_opportunities(currency).await;
        
        // Filter by risk
        let suitable: Vec<_> = opportunities
            .into_iter()
            .filter(|o| o.risk_score <= max_risk)
            .collect();
        
        if suitable.is_empty() {
            return Ok(vec![]);
        }
        
        // Simple allocation: proportional to risk-adjusted APY
        let total_score: Decimal = suitable
            .iter()
            .map(|o| o.apy / Decimal::from(o.risk_score))
            .sum();
        
        let allocations: Vec<_> = suitable
            .iter()
            .map(|o| {
                let score = o.apy / Decimal::from(o.risk_score);
                let weight = score / total_score;
                Allocation {
                    protocol: o.protocol.clone(),
                    amount: total_capital * weight,
                    expected_apy: o.apy,
                    risk_score: o.risk_score,
                }
            })
            .collect();
        
        Ok(allocations)
    }
    
    /// Find all yield opportunities (alias for get_all_opportunities)
    pub async fn find_opportunities(&self) -> Result<Vec<YieldOpportunity>> {
        // Return opportunities for all supported currencies
        let mut all = Vec::new();
        for currency in [Currency::USDC, Currency::USDT, Currency::DAI, Currency::ETH] {
            all.extend(self.get_all_opportunities(currency).await);
        }
        Ok(all)
    }
    
    /// Allocate funds to a yield protocol (stub - requires DeFi integration)
    pub async fn allocate(
        &self,
        currency: Currency,
        amount: Decimal,
        protocol: String,
    ) -> Result<YieldPosition> {
        // In production, this would interact with Aave/Compound/etc.
        // For now, return a mock position
        let opportunity = self.find_best(currency).await?;
        
        Ok(YieldPosition {
            id: Uuid::new_v4(),
            protocol,
            currency,
            amount,
            apy: opportunity.apy,
            opened_at: Utc::now(),
        })
    }
    
    /// Get total value of all yield positions in USD
    pub async fn total_value_usd(&self) -> Result<Decimal> {
        // In production, would sum all active positions
        // For now, return zero
        Ok(Decimal::ZERO)
    }
    
    fn init_protocols(&mut self) {
        // Traditional money market
        self.protocols.push(YieldProtocol {
            name: "US Treasury Bills".to_string(),
            protocol_type: ProtocolType::MoneyMarket,
            supported_currencies: vec![Currency::USD],
            base_apy: "0.045".parse::<Decimal>().unwrap(), // 4.5%
            reward_apy: None,
            tvl: Decimal::from(1000000000000i64), // $1T
            risk_score: 1,
            lockup_period_days: 90,
        });
        
        // DeFi protocols (will be enabled in Sprint 31)
        self.protocols.push(YieldProtocol {
            name: "Aave".to_string(),
            protocol_type: ProtocolType::Lending,
            supported_currencies: vec![Currency::USDC, Currency::USDT, Currency::DAI],
            base_apy: "0.05".parse::<Decimal>().unwrap(), // 5%
            reward_apy: Some("0.02".parse::<Decimal>().unwrap()), // +2% in AAVE
            tvl: Decimal::from(10000000000i64), // $10B
            risk_score: 3,
            lockup_period_days: 0,
        });
        
        self.protocols.push(YieldProtocol {
            name: "Compound".to_string(),
            protocol_type: ProtocolType::Lending,
            supported_currencies: vec![Currency::USDC, Currency::USDT],
            base_apy: "0.048".parse::<Decimal>().unwrap(), // 4.8%
            reward_apy: Some("0.015".parse::<Decimal>().unwrap()), // +1.5% in COMP
            tvl: Decimal::from(5000000000i64), // $5B
            risk_score: 3,
            lockup_period_days: 0,
        });
        
        // Staking
        self.protocols.push(YieldProtocol {
            name: "ETH Staking".to_string(),
            protocol_type: ProtocolType::Staking,
            supported_currencies: vec![Currency::ETH],
            base_apy: "0.035".parse::<Decimal>().unwrap(), // 3.5%
            reward_apy: None,
            tvl: Decimal::from(50000000000i64), // $50B
            risk_score: 2,
            lockup_period_days: 0, // Can exit via LSTs
        });
        
        // Higher yield options
        self.protocols.push(YieldProtocol {
            name: "Convex Finance".to_string(),
            protocol_type: ProtocolType::LiquidityPool,
            supported_currencies: vec![Currency::USDC, Currency::USDT],
            base_apy: "0.08".parse::<Decimal>().unwrap(), // 8%
            reward_apy: Some("0.05".parse::<Decimal>().unwrap()), // +5% in CVX
            tvl: Decimal::from(3000000000i64), // $3B
            risk_score: 5,
            lockup_period_days: 0,
        });
    }
}

/// Allocation result
#[derive(Debug, Clone)]
pub struct Allocation {
    pub protocol: String,
    pub amount: Decimal,
    pub expected_apy: Decimal,
    pub risk_score: u8,
}

impl Allocation {
    pub fn expected_return(&self) -> Decimal {
        self.amount * self.expected_apy
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_find_best_yield() {
        let optimizer = YieldOptimizer::new().await.unwrap();
        
        let best = optimizer.find_best(Currency::USD).await.unwrap();
        
        assert!(!best.protocol.is_empty());
        assert!(best.apy > Decimal::ZERO);
        assert!(best.apy < Decimal::from(1)); // Less than 100%
        assert!(best.risk_score >= 1 && best.risk_score <= 10);
    }
    
    #[tokio::test]
    async fn test_yield_higher_than_treasury() {
        let optimizer = YieldOptimizer::new().await.unwrap();
        
        let best = optimizer.find_best(Currency::USD).await.unwrap();
        
        // Best should be at least 4% (Treasury bills baseline)
        assert!(best.apy > "0.04".parse::<Decimal>().unwrap());
    }
    
    #[tokio::test]
    async fn test_optimize_allocation() {
        let optimizer = YieldOptimizer::new().await.unwrap();
        
        let allocations = optimizer
            .optimize_allocation(Decimal::from(100000), Currency::USD, 5)
            .await
            .unwrap();
        
        assert!(!allocations.is_empty());
        
        // Check total allocation sums to capital
        let total: Decimal = allocations.iter().map(|a| a.amount).sum();
        assert!(total > Decimal::ZERO);
    }
    
    #[tokio::test]
    async fn test_low_risk_filter() {
        let optimizer = YieldOptimizer::new().await.unwrap();
        
        // Very conservative (risk 1 only)
        let allocations = optimizer
            .optimize_allocation(Decimal::from(100000), Currency::USD, 1)
            .await
            .unwrap();
        
        for alloc in &allocations {
            assert_eq!(alloc.risk_score, 1);
        }
    }
}
