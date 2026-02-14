//! Futures Roll Manager

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use super::SchedulerError;

/// Futures roll manager
#[derive(Debug)]
pub struct FuturesRollManager {
    contracts: Vec<FuturesContract>,
}

impl FuturesRollManager {
    pub fn new() -> Self {
        Self {
            contracts: Vec::new(),
        }
    }
    
    /// Detect expiring contracts
    pub fn detect_expiring_contracts(&self) -> Vec<ContractExpiration> {
        let mut expiring = Vec::new();
        let now = Utc::now();
        
        for contract in &self.contracts {
            let days_to_expiry = (contract.expiry - now).num_days();
            
            if days_to_expiry <= contract.roll_days_before as i64 {
                expiring.push(ContractExpiration {
                    symbol: contract.symbol.clone(),
                    current_contract: contract.contract_code.clone(),
                    next_contract: contract.next_contract.clone(),
                    expiry_date: contract.expiry,
                    days_remaining: days_to_expiry as i32,
                    urgency: if days_to_expiry <= 2 {
                        RollUrgency::High
                    } else {
                        RollUrgency::Medium
                    },
                });
            }
        }
        
        expiring
    }
    
    /// Calculate optimal roll plan
    pub fn calculate_optimal_roll(&self, contract: &FuturesContract) -> RollPlan {
        RollPlan {
            from_contract: contract.contract_code.clone(),
            to_contract: contract.next_contract.clone(),
            quantity: Decimal::ONE, // Would be actual position size
            target_spread: Decimal::try_from(0.02).unwrap(), // 2 ticks
            max_slippage: Decimal::try_from(0.05).unwrap(),  // 5 ticks
        }
    }
    
    /// Execute roll
    pub async fn execute_roll(&self, plan: RollPlan) -> Result<RollResult, SchedulerError> {
        // Would execute actual roll trade
        Ok(RollResult {
            success: true,
            executed_quantity: plan.quantity,
            avg_spread: plan.target_spread,
            cost: Decimal::ZERO, // Would calculate actual cost
        })
    }
    
    /// Register contract
    pub fn register_contract(&mut self, contract: FuturesContract) {
        self.contracts.push(contract);
    }
}

impl Default for FuturesRollManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Futures contract
#[derive(Debug, Clone)]
pub struct FuturesContract {
    pub symbol: String,
    pub contract_code: String,
    pub next_contract: String,
    pub expiry: DateTime<Utc>,
    pub roll_days_before: u32,
    pub contract_size: Decimal,
}

/// Contract expiration info
#[derive(Debug, Clone)]
pub struct ContractExpiration {
    pub symbol: String,
    pub current_contract: String,
    pub next_contract: String,
    pub expiry_date: DateTime<Utc>,
    pub days_remaining: i32,
    pub urgency: RollUrgency,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RollUrgency {
    Low,    // > 5 days
    Medium, // 3-5 days
    High,   // < 3 days
}

/// Roll plan
#[derive(Debug, Clone)]
pub struct RollPlan {
    pub from_contract: String,
    pub to_contract: String,
    pub quantity: Decimal,
    pub target_spread: Decimal,
    pub max_slippage: Decimal,
}

/// Roll result
#[derive(Debug, Clone)]
pub struct RollResult {
    pub success: bool,
    pub executed_quantity: Decimal,
    pub avg_spread: Decimal,
    pub cost: Decimal,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn test_roll_detection() {
        let mut manager = FuturesRollManager::new();
        
        // Add contract expiring in 2 days
        manager.register_contract(FuturesContract {
            symbol: "ES".to_string(),
            contract_code: "ESZ24".to_string(),
            next_contract: "ESH25".to_string(),
            expiry: Utc::now() + Duration::days(2),
            roll_days_before: 5,
            contract_size: Decimal::from(50),
        });
        
        let expiring = manager.detect_expiring_contracts();
        assert_eq!(expiring.len(), 1);
        assert_eq!(expiring[0].urgency, RollUrgency::High);
    }

    #[test]
    fn test_no_roll_needed() {
        let mut manager = FuturesRollManager::new();
        
        // Add contract expiring in 30 days
        manager.register_contract(FuturesContract {
            symbol: "ES".to_string(),
            contract_code: "ESZ24".to_string(),
            next_contract: "ESH25".to_string(),
            expiry: Utc::now() + Duration::days(30),
            roll_days_before: 5,
            contract_size: Decimal::from(50),
        });
        
        let expiring = manager.detect_expiring_contracts();
        assert!(expiring.is_empty());
    }
}
