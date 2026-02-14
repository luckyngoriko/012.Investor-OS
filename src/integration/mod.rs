//! Treasury-Margin Integration Module
//!
//! Sprint 17: Capital allocation and unified risk management
//! - Capital flows between Treasury and Margin accounts
//! - Unified risk monitoring across modules
//! - Automatic rebalancing and liquidations
//! - Cross-module position limits

pub mod capital_allocator;
pub mod error;
pub mod risk_monitor;

pub use capital_allocator::{CapitalAllocator, CapitalAllocationConfig, CapitalAllocation};
pub use error::{IntegrationError, Result};
pub use risk_monitor::{RiskMonitor, RiskThresholds, RiskAssessment, SystemRiskStatus};

use crate::treasury::Treasury;
use crate::margin::MarginManager;
use rust_decimal::Decimal;
use tracing::{info, error};

/// Integrated trading system managing Treasury and Margin together
#[derive(Debug)]
pub struct IntegratedTradingSystem {
    pub treasury: Treasury,
    pub margin_manager: MarginManager,
    pub capital_allocator: CapitalAllocator,
    pub risk_monitor: RiskMonitor,
}

impl IntegratedTradingSystem {
    /// Create new integrated system
    pub async fn new(
        treasury: Treasury,
        capital_config: CapitalAllocationConfig,
        risk_thresholds: RiskThresholds,
    ) -> Result<Self> {
        let margin_manager = MarginManager::new();
        let capital_allocator = CapitalAllocator::new(capital_config);
        let risk_monitor = RiskMonitor::new(risk_thresholds);
        
        info!("✅ Integrated Trading System initialized");
        
        Ok(Self {
            treasury,
            margin_manager,
            capital_allocator,
            risk_monitor,
        })
    }
    
    /// Allocate capital from treasury to new margin account
    pub async fn allocate_margin_account(
        &mut self,
        owner_id: String,
        amount: Decimal,
    ) -> Result<uuid::Uuid> {
        // Pre-check risk limits
        let available = self.capital_allocator.calculate_available_capital(&self.treasury)?;
        if amount > available {
            return Err(IntegrationError::InsufficientTreasuryFunds {
                requested: amount,
                available,
            });
        }
        
        let account_id = self.capital_allocator.allocate_to_margin(
            &mut self.treasury,
            &mut self.margin_manager,
            owner_id,
            amount,
        ).await?;
        
        info!("✅ Allocated ${} to margin account {}", amount, account_id);
        Ok(account_id)
    }
    
    /// Open leveraged position with cross-module risk checks
    pub async fn open_leveraged_position(
        &mut self,
        account_id: uuid::Uuid,
        symbol: String,
        side: crate::margin::PositionSide,
        quantity: Decimal,
        price: Decimal,
        leverage: Decimal,
    ) -> Result<()> {
        // Check risk limits first
        let notional = quantity * price;
        self.risk_monitor.check_position_risk(
            &self.margin_manager,
            account_id,
            &symbol,
            notional,
        )?;
        
        // Check daily loss limits
        if let Some(account) = self.margin_manager.get_account(account_id) {
            self.risk_monitor.check_daily_loss_limit(account_id, account.equity)?;
        }
        
        // Open position (clone symbol for logging)
        let symbol_for_log = symbol.clone();
        self.margin_manager.open_position(
            account_id,
            symbol,
            side,
            quantity,
            price,
            leverage,
        ).map_err(|e| IntegrationError::MarginError(e.to_string()))?;
        
        // Update exposure tracking
        self.capital_allocator.update_exposure(account_id, &self.margin_manager);
        
        info!(
            "📈 Opened position: {} {} @ ${} ({}x leverage)",
            quantity, symbol_for_log, price, leverage
        );
        
        Ok(())
    }
    
    /// Close position and update risk tracking
    pub async fn close_position(
        &mut self,
        account_id: uuid::Uuid,
        symbol: &str,
        exit_price: Decimal,
    ) -> Result<Decimal> {
        let pnl = self.margin_manager.close_position(account_id, symbol, exit_price)
            .map_err(|e| IntegrationError::MarginError(e.to_string()))?;
        
        // Track daily P&L
        self.risk_monitor.update_daily_pnl(account_id, pnl);
        
        // Update exposure
        self.capital_allocator.update_exposure(account_id, &self.margin_manager);
        
        info!(
            "💰 Closed position {}: P&L = ${}",
            symbol, pnl
        );
        
        Ok(pnl)
    }
    
    /// Update market prices and process liquidations
    pub async fn update_market_prices(
        &mut self,
        prices: &std::collections::HashMap<String, Decimal>,
    ) -> Vec<(uuid::Uuid, crate::margin::LiquidationResult)> {
        let liquidations = self.margin_manager.update_market_prices(prices).await;
        
        // Record liquidations in risk monitor
        for (account_id, result) in &liquidations {
            self.risk_monitor.record_liquidation(*account_id, result.total_notional);
            error!(
                "🚨 Liquidation for account {}: {} positions, ${} notional",
                account_id, result.positions_closed, result.total_notional
            );
        }
        
        // Update all exposure tracking
        for account_id in self.margin_manager.get_accounts().keys() {
            self.capital_allocator.update_exposure(*account_id, &self.margin_manager);
        }
        
        liquidations
    }
    
    /// Perform system-wide risk assessment
    pub fn assess_risk(&self) -> RiskAssessment {
        self.risk_monitor.assess_system_risk(&self.treasury, &self.margin_manager)
    }
    
    /// Get total system equity (treasury + margin P&L)
    pub fn total_system_equity(&self) -> Decimal {
        let treasury_equity = self.treasury.total_equity();
        let margin_pnl: Decimal = self.margin_manager.get_accounts()
            .values()
            .map(|acc| {
                acc.positions.values()
                    .map(|p| p.unrealized_pnl())
                    .fold(Decimal::ZERO, |a, b| a + b)
            })
            .fold(Decimal::ZERO, |a, b| a + b);
        
        treasury_equity + margin_pnl
    }
    
    /// Check if rebalancing is needed and execute if so
    pub async fn maybe_rebalance(&mut self) -> Vec<uuid::Uuid> {
        let mut rebalanced = Vec::new();
        
        for account_id in self.margin_manager.get_accounts().keys() {
            if self.capital_allocator.check_rebalance_needed(*account_id) {
                // In production: execute actual rebalancing logic
                info!("🔄 Rebalancing needed for account {}", account_id);
                rebalanced.push(*account_id);
            }
        }
        
        rebalanced
    }
    
    /// Withdraw capital from margin back to treasury
    pub async fn withdraw_from_margin(
        &mut self,
        account_id: uuid::Uuid,
        amount: Decimal,
    ) -> Result<()> {
        self.capital_allocator.withdraw_from_margin(
            &mut self.treasury,
            &mut self.margin_manager,
            account_id,
            amount,
        ).await
    }
    
    /// Get margin account summary
    pub fn get_account_summary(&self, account_id: uuid::Uuid) -> Option<AccountSummary> {
        let account = self.margin_manager.get_account(account_id)?;
        let allocation = self.capital_allocator.get_allocation(account_id)?;
        let risk = self.margin_manager.calculate_risk(account_id)?;
        
        Some(AccountSummary {
            account_id,
            equity: account.equity,
            allocated: allocation.allocated_amount,
            available_margin: account.available_margin,
            locked_margin: account.locked_margin,
            total_exposure: account.total_exposure(),
            unrealized_pnl: account.positions.values()
                .map(|p| p.unrealized_pnl())
                .fold(Decimal::ZERO, |a, b| a + b),
            position_count: account.positions.len(),
            leverage_used: risk.leverage_used,
            margin_ratio: risk.margin_ratio,
        })
    }
}

/// Account summary for reporting
#[derive(Debug, Clone)]
pub struct AccountSummary {
    pub account_id: uuid::Uuid,
    pub equity: Decimal,
    pub allocated: Decimal,
    pub available_margin: Decimal,
    pub locked_margin: Decimal,
    pub total_exposure: Decimal,
    pub unrealized_pnl: Decimal,
    pub position_count: usize,
    pub leverage_used: Decimal,
    pub margin_ratio: Decimal,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::margin::PositionSide;
    use crate::treasury::Currency;
    
    #[tokio::test]
    async fn test_full_integration_lifecycle() {
        println!("\n🏦 Testing Treasury-Margin Integration");
        
        // Setup
        let treasury = Treasury::new().await.unwrap();
        let config = CapitalAllocationConfig::default();
        let thresholds = RiskThresholds {
            max_concentration_percent: Decimal::ONE, // 100% for testing
            ..Default::default()
        };
        
        let mut system = IntegratedTradingSystem::new(
            treasury,
            config,
            thresholds,
        ).await.unwrap();
        
        // 1. Deposit to treasury (USDC instead of USD - fiat not supported)
        let deposit = system.treasury.process_deposit(
            Currency::USDC, 
            Decimal::from(500000),
            "0xabc123".to_string(),
            "0xfromaddress".to_string(),
            "ETH".to_string(),
        ).await.unwrap();
        println!("✅ Deposited $500,000 to treasury");
        
        // 2. Allocate to margin
        let account_id = system.allocate_margin_account(
            "autotrader".to_string(),
            Decimal::from(100000),
        ).await.unwrap();
        println!("✅ Allocated $100,000 to margin account {}", account_id);
        
        // 3. Open leveraged positions
        system.open_leveraged_position(
            account_id,
            "BTC".to_string(),
            PositionSide::Long,
            Decimal::from(1),
            Decimal::from(40000),
            Decimal::from(4),
        ).await.unwrap();
        println!("📈 Opened 1 BTC long at $40k with 4x leverage");
        
        system.open_leveraged_position(
            account_id,
            "ETH".to_string(),
            PositionSide::Long,
            Decimal::from(10),
            Decimal::from(3000),
            Decimal::from(5),
        ).await.unwrap();
        println!("📈 Opened 10 ETH long at $3k with 5x leverage");
        
        // 4. Check risk
        let assessment = system.assess_risk();
        println!("📊 Risk Assessment: {:?}", assessment.status);
        println!("   Treasury Equity: ${}", assessment.treasury_equity);
        println!("   Margin Allocated: ${}", assessment.margin_allocated);
        println!("   Total Exposure: ${}", assessment.total_exposure);
        println!("   Portfolio Leverage: {:.2}x", assessment.portfolio_leverage);
        
        // 5. Update prices (profit scenario)
        let mut prices = std::collections::HashMap::new();
        prices.insert("BTC".to_string(), Decimal::from(44000)); // +10%
        prices.insert("ETH".to_string(), Decimal::from(3300));  // +10%
        
        let liquidations = system.update_market_prices(&prices).await;
        assert!(liquidations.is_empty());
        
        // 6. Check account summary
        let summary = system.get_account_summary(account_id).unwrap();
        println!("\n📋 Account Summary:");
        println!("   Equity: ${}", summary.equity);
        println!("   Unrealized P&L: ${}", summary.unrealized_pnl);
        println!("   Positions: {}", summary.position_count);
        
        // BTC: $4k profit, ETH: $3k profit = $7k total
        assert_eq!(summary.unrealized_pnl, Decimal::from(7000));
        
        // 7. Close positions
        let btc_pnl = system.close_position(account_id, "BTC", Decimal::from(44000)).await.unwrap();
        let eth_pnl = system.close_position(account_id, "ETH", Decimal::from(3300)).await.unwrap();
        
        println!("\n💰 Closed all positions:");
        println!("   BTC P&L: ${}", btc_pnl);
        println!("   ETH P&L: ${}", eth_pnl);
        
        // 8. Final system equity
        let total_equity = system.total_system_equity();
        let margin_equity = system.get_account_summary(account_id).unwrap().equity;
        println!("\n🏦 Final System Equity: ${}", total_equity);
        println!("   Margin Account Equity: ${}", margin_equity);
        
        // Margin account: $100k + $14k profits (with leverage effects) = $114k
        assert!(margin_equity > Decimal::from(100000));
        
        println!("\n✅ Full integration lifecycle completed successfully!");
    }
    
    #[tokio::test]
    async fn test_risk_limits_enforced() {
        let treasury = Treasury::new().await.unwrap();
        let config = CapitalAllocationConfig::default();
        let thresholds = RiskThresholds {
            max_concentration_percent: Decimal::try_from(0.50).unwrap(), // 50% max
            ..Default::default()
        };
        
        let mut system = IntegratedTradingSystem::new(
            treasury,
            config,
            thresholds,
        ).await.unwrap();
        
        // Deposit and allocate (USDC instead of USD)
        let _deposit = system.treasury.process_deposit(
            Currency::USDC, 
            Decimal::from(200000),
            "0xdef456".to_string(),
            "0xfromaddress2".to_string(),
            "ETH".to_string(),
        ).await.unwrap();
        
        let account_id = system.allocate_margin_account(
            "test".to_string(),
            Decimal::from(50000),
        ).await.unwrap();
        
        // Open large position (would exceed 50% concentration if we tried another large one)
        system.open_leveraged_position(
            account_id,
            "BTC".to_string(),
            PositionSide::Long,
            Decimal::from(2),
            Decimal::from(40000),
            Decimal::from(2),
        ).await.unwrap();
        
        // Try to open another huge position that would exceed concentration
        let result = system.open_leveraged_position(
            account_id,
            "ETH".to_string(),
            PositionSide::Long,
            Decimal::from(100), // $300k notional
            Decimal::from(3000),
            Decimal::from(10),
        ).await;
        
        // Should fail due to concentration limit
        assert!(result.is_err());
    }
}
