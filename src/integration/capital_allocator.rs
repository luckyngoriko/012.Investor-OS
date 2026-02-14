//! Capital allocation between Treasury and Margin accounts

use rust_decimal::Decimal;
use tracing::info;
use std::collections::HashMap;

use crate::treasury::{Currency, Treasury};
use crate::margin::MarginManager;
use super::error::{IntegrationError, Result};

/// Capital allocation configuration
#[derive(Debug, Clone)]
pub struct CapitalAllocationConfig {
    /// Maximum percentage of treasury that can be allocated to margin (0-1)
    pub max_margin_allocation_percent: Decimal,
    /// Minimum treasury reserve that must be kept (absolute amount)
    pub min_treasury_reserve_usd: Decimal,
    /// Default leverage limit for new margin accounts
    pub default_max_leverage: Decimal,
    /// Automatic rebalancing threshold (% deviation)
    pub rebalance_threshold_percent: Decimal,
    /// Currency for margin trading operations
    pub margin_trading_currency: Currency,
}

impl Default for CapitalAllocationConfig {
    fn default() -> Self {
        Self {
            max_margin_allocation_percent: Decimal::try_from(0.80).unwrap(), // 80%
            min_treasury_reserve_usd: Decimal::from(100000), // $100k minimum
            default_max_leverage: Decimal::from(10), // 10x default
            rebalance_threshold_percent: Decimal::try_from(0.05).unwrap(), // 5%
            margin_trading_currency: Currency::USDC,
        }
    }
}

/// Capital allocation state tracking
#[derive(Debug, Clone)]
pub struct CapitalAllocation {
    pub account_id: uuid::Uuid,
    pub allocated_amount: Decimal,
    pub max_allowed: Decimal,
    pub current_exposure: Decimal,
    pub last_rebalanced: chrono::DateTime<chrono::Utc>,
}

/// Capital allocator manages fund flows between Treasury and Margin
#[derive(Debug)]
pub struct CapitalAllocator {
    config: CapitalAllocationConfig,
    allocations: HashMap<uuid::Uuid, CapitalAllocation>,
    total_allocated: Decimal,
}

impl CapitalAllocator {
    /// Create new capital allocator
    pub fn new(config: CapitalAllocationConfig) -> Self {
        Self {
            config,
            allocations: HashMap::new(),
            total_allocated: Decimal::ZERO,
        }
    }
    
    /// Calculate available capital for margin allocation from treasury
    pub fn calculate_available_capital(&self, treasury: &Treasury) -> Result<Decimal> {
        let total_equity = treasury.total_equity();
        let max_allocation = total_equity * self.config.max_margin_allocation_percent;
        
        // Ensure minimum reserve
        let available = max_allocation.saturating_sub(self.total_allocated);
        let reserve_adjusted = available.saturating_sub(self.config.min_treasury_reserve_usd);
        
        Ok(reserve_adjusted.max(Decimal::ZERO))
    }
    
    /// Allocate capital to new margin account
    pub async fn allocate_to_margin(
        &mut self,
        treasury: &mut Treasury,
        margin_manager: &mut MarginManager,
        owner_id: String,
        amount: Decimal,
    ) -> Result<uuid::Uuid> {
        // Check available capital
        let available = self.calculate_available_capital(treasury)?;
        if amount > available {
            return Err(IntegrationError::InsufficientTreasuryFunds {
                requested: amount,
                available,
            });
        }
        
        // Check if treasury has sufficient available balance
        let treasury_balance = treasury.get_balance(self.config.margin_trading_currency)
            .map(|b| b.available)
            .unwrap_or(Decimal::ZERO);
            
        if amount > treasury_balance {
            return Err(IntegrationError::InsufficientTreasuryFunds {
                requested: amount,
                available: treasury_balance,
            });
        }
        
        // Create margin account
        let account_id = margin_manager.create_account(owner_id, amount);
        
        // Set leverage limit
        if let Some(account) = margin_manager.get_account_mut(account_id) {
            account.set_max_leverage(self.config.default_max_leverage)
                .map_err(|e| IntegrationError::MarginError(e.to_string()))?;
        }
        
        // Track allocation
        let allocation = CapitalAllocation {
            account_id,
            allocated_amount: amount,
            max_allowed: amount * self.config.default_max_leverage,
            current_exposure: Decimal::ZERO,
            last_rebalanced: chrono::Utc::now(),
        };
        
        self.allocations.insert(account_id, allocation);
        self.total_allocated += amount;
        
        info!(
            "Allocated ${} to margin account {}. Total allocated: ${}",
            amount, account_id, self.total_allocated
        );
        
        Ok(account_id)
    }
    
    /// Add more capital to existing margin account
    pub async fn add_capital(
        &mut self,
        treasury: &mut Treasury,
        margin_manager: &mut MarginManager,
        account_id: uuid::Uuid,
        amount: Decimal,
    ) -> Result<()> {
        let available = self.calculate_available_capital(treasury)?;
        if amount > available {
            return Err(IntegrationError::InsufficientTreasuryFunds {
                requested: amount,
                available,
            });
        }
        
        margin_manager.deposit(account_id, amount)
            .map_err(|e| IntegrationError::MarginError(e.to_string()))?;
        
        if let Some(allocation) = self.allocations.get_mut(&account_id) {
            allocation.allocated_amount += amount;
            allocation.max_allowed = allocation.allocated_amount * self.config.default_max_leverage;
        }
        
        self.total_allocated += amount;
        
        info!("Added ${} to margin account {}", amount, account_id);
        Ok(())
    }
    
    /// Withdraw capital from margin back to treasury
    pub async fn withdraw_from_margin(
        &mut self,
        _treasury: &mut Treasury,
        margin_manager: &mut MarginManager,
        account_id: uuid::Uuid,
        amount: Decimal,
    ) -> Result<()> {
        margin_manager.withdraw(account_id, amount)
            .map_err(|e| IntegrationError::MarginError(e.to_string()))?;
        
        if let Some(allocation) = self.allocations.get_mut(&account_id) {
            allocation.allocated_amount -= amount;
        }
        
        self.total_allocated -= amount;
        
        info!("Withdrew ${} from margin account {}", amount, account_id);
        Ok(())
    }
    
    /// Update exposure tracking for risk monitoring
    pub fn update_exposure(&mut self, account_id: uuid::Uuid, margin_manager: &MarginManager) {
        if let Some(account) = margin_manager.get_account(account_id) {
            if let Some(allocation) = self.allocations.get_mut(&account_id) {
                allocation.current_exposure = account.total_exposure();
            }
        }
    }
    
    /// Check if rebalancing is needed
    pub fn check_rebalance_needed(&self, account_id: uuid::Uuid) -> bool {
        if let Some(allocation) = self.allocations.get(&account_id) {
            if allocation.allocated_amount.is_zero() {
                return false;
            }
            
            let exposure_ratio = allocation.current_exposure / allocation.allocated_amount;
            let threshold = self.config.rebalance_threshold_percent;
            
            // Rebalance if exposure deviates significantly from allocation
            (exposure_ratio - Decimal::ONE).abs() > threshold
        } else {
            false
        }
    }
    
    /// Get total capital allocated to margin
    pub fn total_allocated(&self) -> Decimal {
        self.total_allocated
    }
    
    /// Get allocation details for account
    pub fn get_allocation(&self, account_id: uuid::Uuid) -> Option<&CapitalAllocation> {
        self.allocations.get(&account_id)
    }
    
    /// Get utilization ratio (allocated / total treasury equity)
    pub fn utilization_ratio(&self, treasury: &Treasury) -> Decimal {
        let total_equity = treasury.total_equity();
        if total_equity.is_zero() {
            return Decimal::ZERO;
        }
        
        self.total_allocated / total_equity
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_capital_allocation_flow() {
        let config = CapitalAllocationConfig::default();
        let mut allocator = CapitalAllocator::new(config);
        
        // Setup treasury with 500k USDC (fiat not supported)
        let mut treasury = Treasury::new().await.unwrap();
        let _deposit = treasury.process_deposit(
            Currency::USDC, 
            Decimal::from(500000),
            "0xabc123".to_string(),
            "0xfromaddress".to_string(),
            "ETH".to_string(),
        ).await.unwrap();
        
        // Setup margin manager
        let mut margin_manager = MarginManager::new();
        
        // Calculate available capital
        let available = allocator.calculate_available_capital(&treasury).unwrap();
        // 80% of $500k = $400k, minus $100k reserve = $300k
        assert_eq!(available, Decimal::from(300000));
        
        // Allocate $100k to margin
        let account_id = allocator.allocate_to_margin(
            &mut treasury,
            &mut margin_manager,
            "trader_001".to_string(),
            Decimal::from(100000),
        ).await.unwrap();
        
        assert_eq!(allocator.total_allocated(), Decimal::from(100000));
        
        let account = margin_manager.get_account(account_id).unwrap();
        assert_eq!(account.equity, Decimal::from(100000));
        assert_eq!(account.max_leverage, Decimal::from(10));
        
        // Add more capital
        allocator.add_capital(
            &mut treasury,
            &mut margin_manager,
            account_id,
            Decimal::from(50000),
        ).await.unwrap();
        
        assert_eq!(allocator.total_allocated(), Decimal::from(150000));
        
        // Withdraw some
        allocator.withdraw_from_margin(
            &mut treasury,
            &mut margin_manager,
            account_id,
            Decimal::from(30000),
        ).await.unwrap();
        
        assert_eq!(allocator.total_allocated(), Decimal::from(120000));
    }
    
    #[tokio::test]
    async fn test_insufficient_funds_protection() {
        let config = CapitalAllocationConfig::default();
        let mut allocator = CapitalAllocator::new(config);
        
        let mut treasury = Treasury::new().await.unwrap();
        let _deposit = treasury.process_deposit(
            Currency::USDC, 
            Decimal::from(50000),
            "0xghi789".to_string(),
            "0xfromaddress3".to_string(),
            "ETH".to_string(),
        ).await.unwrap();
        
        let mut margin_manager = MarginManager::new();
        
        // Try to allocate more than available
        let result = allocator.allocate_to_margin(
            &mut treasury,
            &mut margin_manager,
            "trader_001".to_string(),
            Decimal::from(100000),
        ).await;
        
        assert!(result.is_err());
    }
    
    #[tokio::test]
    async fn test_utilization_ratio() {
        let config = CapitalAllocationConfig {
            max_margin_allocation_percent: Decimal::ONE, // 100%
            min_treasury_reserve_usd: Decimal::ZERO,
            ..Default::default()
        };
        
        let mut allocator = CapitalAllocator::new(config);
        let mut treasury = Treasury::new().await.unwrap();
        let mut margin_manager = MarginManager::new();
        
        let _deposit = treasury.process_deposit(
            Currency::USDC, 
            Decimal::from(200000),
            "0xjkl012".to_string(),
            "0xfromaddress4".to_string(),
            "ETH".to_string(),
        ).await.unwrap();
        
        // Allocate $50k of $200k = 25%
        allocator.allocate_to_margin(
            &mut treasury,
            &mut margin_manager,
            "trader_001".to_string(),
            Decimal::from(50000),
        ).await.unwrap();
        
        let ratio = allocator.utilization_ratio(&treasury);
        assert_eq!(ratio, Decimal::try_from(0.25).unwrap());
    }
}
