//! Margin Module - Leveraged trading and risk management
//! 
//! This module provides:
//! - Margin accounts with configurable leverage
//! - Position tracking with unrealized P&L
//! - Margin requirement calculations
//! - Liquidation engine for risk management
//! - Integration with Treasury for capital allocation

pub mod account;
pub mod calculator;
pub mod error;
pub mod liquidation;
pub mod position;

pub use account::{MarginAccount, AccountStatus};
pub use calculator::{MarginCalculator, RiskMetrics};
pub use error::{MarginError, Result};
pub use liquidation::{LiquidationEngine, LiquidationResult};
pub use position::{Position, PositionSide};

use rust_decimal::Decimal;
use std::collections::HashMap;
use uuid::Uuid;

/// Margin manager - main entry point for leveraged trading
#[derive(Debug)]
pub struct MarginManager {
    accounts: HashMap<Uuid, MarginAccount>,
    calculator: MarginCalculator,
    liquidation_engine: LiquidationEngine,
}

impl MarginManager {
    /// Create new margin manager
    pub fn new() -> Self {
        Self {
            accounts: HashMap::new(),
            calculator: MarginCalculator::new(),
            liquidation_engine: LiquidationEngine::new(),
        }
    }
    
    /// Create new margin account
    pub fn create_account(&mut self, owner_id: String, initial_capital: Decimal) -> Uuid {
        let account = MarginAccount::new(owner_id, initial_capital);
        let id = account.id;
        self.accounts.insert(id, account);
        id
    }
    
    /// Get account reference
    pub fn get_account(&self, id: Uuid) -> Option<&MarginAccount> {
        self.accounts.get(&id)
    }
    
    /// Get mutable account reference
    pub fn get_account_mut(&mut self, id: Uuid) -> Option<&mut MarginAccount> {
        self.accounts.get_mut(&id)
    }
    
    /// Open position in account
    pub fn open_position(
        &mut self,
        account_id: Uuid,
        symbol: String,
        side: PositionSide,
        quantity: Decimal,
        price: Decimal,
        leverage: Decimal,
    ) -> Result<()> {
        let account = self.accounts.get_mut(&account_id)
            .ok_or_else(|| MarginError::PositionNotFound("Account not found".to_string()))?;
        
        let position = Position::new(symbol, side, quantity, price, leverage);
        account.open_position(position)
    }
    
    /// Close position
    pub fn close_position(
        &mut self,
        account_id: Uuid,
        symbol: &str,
        exit_price: Decimal,
    ) -> Result<Decimal> {
        let account = self.accounts.get_mut(&account_id)
            .ok_or_else(|| MarginError::PositionNotFound("Account not found".to_string()))?;
        
        account.close_position(symbol, exit_price)
    }
    
    /// Update market prices and check for liquidations
    pub async fn update_market_prices(&mut self, prices: &HashMap<String, Decimal>) -> Vec<(Uuid, LiquidationResult)> {
        let mut liquidations = Vec::new();
        
        for (id, account) in self.accounts.iter_mut() {
            account.update_prices(prices);
            
            // Check and execute liquidations
            if let Some(result) = self.liquidation_engine.process_account(account).await {
                liquidations.push((*id, result));
            }
        }
        
        liquidations
    }
    
    /// Calculate risk metrics for account
    pub fn calculate_risk(&self, account_id: Uuid) -> Option<RiskMetrics> {
        self.accounts.get(&account_id)
            .map(|account| self.calculator.calculate_risk(account))
    }
    
    /// Deposit capital to account
    pub fn deposit(&mut self, account_id: Uuid, amount: Decimal) -> Result<()> {
        let account = self.accounts.get_mut(&account_id)
            .ok_or_else(|| MarginError::PositionNotFound("Account not found".to_string()))?;
        
        account.deposit(amount);
        Ok(())
    }
    
    /// Withdraw capital from account
    pub fn withdraw(&mut self, account_id: Uuid, amount: Decimal) -> Result<()> {
        let account = self.accounts.get_mut(&account_id)
            .ok_or_else(|| MarginError::PositionNotFound("Account not found".to_string()))?;
        
        account.withdraw(amount)
    }
    
    /// Get reference to all accounts (for risk monitoring)
    pub fn get_accounts(&self) -> &HashMap<Uuid, MarginAccount> {
        &self.accounts
    }
    
    /// Get mutable reference to all accounts
    pub fn get_accounts_mut(&mut self) -> &mut HashMap<Uuid, MarginAccount> {
        &mut self.accounts
    }
    
    /// Get all account IDs with margin calls
    pub fn get_margin_calls(&self) -> Vec<Uuid> {
        self.accounts.iter()
            .filter(|(_, acc)| acc.is_margin_call())
            .map(|(id, _)| *id)
            .collect()
    }
    
    /// Get total exposure across all accounts
    pub fn total_exposure(&self) -> Decimal {
        self.accounts.values()
            .map(|acc| acc.total_exposure())
            .fold(Decimal::ZERO, |a, b| a + b)
    }
}

impl Default for MarginManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_margin_manager_creation() {
        let manager = MarginManager::new();
        assert!(manager.accounts.is_empty());
    }
    
    #[test]
    fn test_create_and_get_account() {
        let mut manager = MarginManager::new();
        
        let id = manager.create_account("trader_001".to_string(), Decimal::from(100000));
        
        let account = manager.get_account(id).unwrap();
        assert_eq!(account.owner_id, "trader_001");
        assert_eq!(account.equity, Decimal::from(100000));
    }
    
    #[test]
    fn test_open_and_close_position_flow() {
        let mut manager = MarginManager::new();
        let id = manager.create_account("trader_001".to_string(), Decimal::from(100000));
        
        // Open position
        manager.open_position(
            id,
            "BTC".to_string(),
            PositionSide::Long,
            Decimal::from(1),
            Decimal::from(50000),
            Decimal::from(5),
        ).unwrap();
        
        let account = manager.get_account(id).unwrap();
        assert_eq!(account.positions.len(), 1);
        
        // Close at profit
        let pnl = manager.close_position(id, "BTC", Decimal::from(55000)).unwrap();
        assert_eq!(pnl, Decimal::from(5000));
        
        let account = manager.get_account(id).unwrap();
        assert!(account.positions.is_empty());
        assert_eq!(account.equity, Decimal::from(105000));
    }
    
    #[tokio::test]
    async fn test_price_update_triggers_liquidation() {
        let mut manager = MarginManager::new();
        let id = manager.create_account("trader_001".to_string(), Decimal::from(10000));
        
        // Open risky position
        manager.open_position(
            id,
            "BTC".to_string(),
            PositionSide::Long,
            Decimal::from(1),
            Decimal::from(50000),
            Decimal::from(10), // 10x leverage
        ).unwrap();
        
        // Price crash
        let mut prices = HashMap::new();
        prices.insert("BTC".to_string(), Decimal::from(40000)); // -20%
        
        let liquidations = manager.update_market_prices(&prices).await;
        
        // Should have liquidated
        assert!(!liquidations.is_empty());
        
        let account = manager.get_account(id).unwrap();
        assert!(account.positions.is_empty() || account.status == AccountStatus::Liquidated);
    }
    
    #[test]
    fn test_calculate_risk_metrics() {
        let mut manager = MarginManager::new();
        let id = manager.create_account("trader_001".to_string(), Decimal::from(100000));
        
        manager.open_position(
            id,
            "BTC".to_string(),
            PositionSide::Long,
            Decimal::from(1),
            Decimal::from(50000),
            Decimal::from(5),
        ).unwrap();
        
        let metrics = manager.calculate_risk(id).unwrap();
        assert!(metrics.leverage_used > Decimal::ZERO);
    }
    
    #[test]
    fn test_margin_call_detection() {
        let mut manager = MarginManager::new();
        let id = manager.create_account("trader_001".to_string(), Decimal::from(10000));
        
        manager.open_position(
            id,
            "BTC".to_string(),
            PositionSide::Long,
            Decimal::from(1),
            Decimal::from(50000),
            Decimal::from(10),
        ).unwrap();
        
        // Simulate price drop
        let account = manager.get_account_mut(id).unwrap();
        use std::collections::HashMap;
        let mut prices = HashMap::new();
        prices.insert("BTC".to_string(), Decimal::from(42000));
        account.update_prices(&prices);
        
        assert!(account.is_margin_call());
        
        let margin_calls = manager.get_margin_calls();
        assert!(margin_calls.contains(&id));
    }
    
    #[test]
    fn test_deposit_and_withdraw() {
        let mut manager = MarginManager::new();
        let id = manager.create_account("trader_001".to_string(), Decimal::from(100000));
        
        // Deposit
        manager.deposit(id, Decimal::from(50000)).unwrap();
        assert_eq!(manager.get_account(id).unwrap().equity, Decimal::from(150000));
        
        // Withdraw
        manager.withdraw(id, Decimal::from(30000)).unwrap();
        assert_eq!(manager.get_account(id).unwrap().equity, Decimal::from(120000));
        
        // Try to withdraw too much
        let result = manager.withdraw(id, Decimal::from(200000));
        assert!(result.is_err());
    }
    
    #[tokio::test]
    async fn test_full_margin_lifecycle() {
        println!("\n📊 Testing Full Margin Lifecycle");
        
        let mut manager = MarginManager::new();
        
        // 1. Create account with $50k
        let id = manager.create_account("autotrader".to_string(), Decimal::from(50000));
        println!("✅ Created margin account with $50,000 equity");
        
        // 2. Open leveraged position
        manager.open_position(
            id,
            "BTC".to_string(),
            PositionSide::Long,
            Decimal::from(2),      // 2 BTC
            Decimal::from(40000),  // $40k per BTC
            Decimal::from(4),      // 4x leverage
        ).unwrap();
        
        let account = manager.get_account(id).unwrap();
        let notional = Decimal::from(2) * Decimal::from(40000); // $80k
        let margin_used = notional / Decimal::from(4);          // $20k
        
        println!("📈 Opened 2 BTC long at $40k with 4x leverage");
        println!("   Notional: ${}, Margin used: ${}", notional, margin_used);
        println!("   Available: ${}, Equity: ${}", account.available_margin, account.equity);
        
        assert_eq!(account.locked_margin, margin_used);
        
        // 3. Price rises 25% to $50k
        let mut prices = HashMap::new();
        prices.insert("BTC".to_string(), Decimal::from(50000));
        
        let liqs = manager.update_market_prices(&prices).await;
        assert!(liqs.is_empty()); // No liquidation
        
        let account = manager.get_account(id).unwrap();
        let unrealized_pnl = (Decimal::from(50000) - Decimal::from(40000)) * Decimal::from(2);
        
        println!("📊 BTC price rose to $50k (+25%)");
        println!("   Unrealized P&L: ${}", unrealized_pnl);
        println!("   New Equity: ${}", account.equity);
        
        assert_eq!(account.equity, Decimal::from(50000) + unrealized_pnl);
        
        // 4. Close position
        let pnl = manager.close_position(id, "BTC", Decimal::from(50000)).unwrap();
        let account = manager.get_account(id).unwrap();
        
        println!("💰 Closed position with ${} profit", pnl);
        println!("   Final Equity: ${}", account.equity);
        
        // P&L = (50000 - 40000) * 2 = $20,000
        // Initial equity $50k + $20k profit + $20k margin released = $90k
        assert_eq!(pnl, Decimal::from(20000));
        assert_eq!(account.equity, Decimal::from(90000));
        
        println!("✅ Margin lifecycle completed successfully!");
    }
}
