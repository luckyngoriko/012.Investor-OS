//! Liquidation engine for handling margin calls

use rust_decimal::Decimal;
use chrono::Utc;
use tracing::{warn, error};

use super::account::{MarginAccount, AccountStatus};
use super::error::{MarginError, Result};
use super::position::PositionSide;

/// Liquidation engine for risk management
#[derive(Debug, Clone)]
pub struct LiquidationEngine {
    /// Minimum margin ratio before forced liquidation (safety buffer)
    pub liquidation_threshold: Decimal,
    /// Whether partial liquidations are allowed
    pub allow_partial: bool,
    /// Slippage assumption for liquidation (percentage)
    pub liquidation_slippage: Decimal,
}

impl Default for LiquidationEngine {
    fn default() -> Self {
        Self {
            liquidation_threshold: Decimal::try_from(1.02).unwrap(), // 102% (just above 100%)
            allow_partial: true,
            liquidation_slippage: Decimal::try_from(0.005).unwrap(), // 0.5% slippage
        }
    }
}

impl LiquidationEngine {
    /// Create new liquidation engine
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Check if account needs liquidation
    pub fn check_liquidation_needed(&self, account: &MarginAccount) -> bool {
        if account.locked_margin.is_zero() {
            return false;
        }
        
        let ratio = account.margin_ratio();
        ratio < self.liquidation_threshold
    }
    
    /// Calculate which positions to liquidate
    pub fn select_positions_for_liquidation(
        &self,
        account: &MarginAccount,
    ) -> Vec<String> {
        if !self.allow_partial {
            // Liquidate everything
            return account.positions.keys().cloned().collect();
        }
        
        // Strategy: Liquidate largest positions first to quickly improve margin
        let mut positions: Vec<_> = account.positions.values().collect();
        positions.sort_by(|a, b| {
            b.notional_value().cmp(&a.notional_value())
        });
        
        let mut to_liquidate = Vec::new();
        let mut simulated_locked = account.locked_margin;
        let mut simulated_equity = account.equity;
        
        for position in positions {
            // Check if we've restored sufficient margin
            let simulated_ratio = if simulated_locked.is_zero() {
                Decimal::MAX
            } else {
                simulated_equity / simulated_locked
            };
            
            if simulated_ratio > self.liquidation_threshold * Decimal::try_from(1.1).unwrap() {
                // Have enough buffer, stop liquidating
                break;
            }
            
            to_liquidate.push(position.symbol.clone());
            
            // Simulate removing this position
            let pnl = position.unrealized_pnl();
            let margin_released = position.margin_used();
            
            simulated_locked -= margin_released;
            simulated_equity += pnl;
        }
        
        to_liquidate
    }
    
    /// Execute liquidation of selected positions
    pub async fn liquidate_positions(
        &self,
        account: &mut MarginAccount,
        symbols: Vec<String>,
    ) -> Result<LiquidationResult> {
        if account.status == AccountStatus::Liquidated {
            return Err(MarginError::AccountLiquidated { equity: account.equity });
        }
        
        account.status = AccountStatus::Liquidating;
        
        let mut result = LiquidationResult {
            positions_closed: 0,
            total_notional: Decimal::ZERO,
            total_pnl: Decimal::ZERO,
            slippage_cost: Decimal::ZERO,
            timestamp: Utc::now(),
        };
        
        for symbol in symbols {
            if let Some(position) = account.positions.get(&symbol) {
                let notional = position.notional_value();
                let slippage = notional * self.liquidation_slippage;
                
                // Apply slippage against position
                let exit_price = match position.side {
                    PositionSide::Long => {
                        position.current_price * (Decimal::ONE - self.liquidation_slippage)
                    }
                    PositionSide::Short => {
                        position.current_price * (Decimal::ONE + self.liquidation_slippage)
                    }
                };
                
                match account.close_position(&symbol, exit_price) {
                    Ok(pnl) => {
                        result.positions_closed += 1;
                        result.total_notional += notional;
                        result.total_pnl += pnl;
                        result.slippage_cost += slippage;
                        
                        warn!(
                            "Liquidated {} position: PnL={}, Slippage={}",
                            symbol, pnl, slippage
                        );
                    }
                    Err(e) => {
                        error!("Failed to liquidate {}: {}", symbol, e);
                    }
                }
            }
        }
        
        // Update account status
        if account.positions.is_empty() {
            account.status = AccountStatus::Liquidated;
        } else if account.margin_ratio() > self.liquidation_threshold {
            account.status = AccountStatus::Active;
        } else {
            account.status = AccountStatus::MarginCall;
        }
        
        Ok(result)
    }
    
    /// Run full liquidation check and execution
    pub async fn process_account(&self, account: &mut MarginAccount) -> Option<LiquidationResult> {
        if !self.check_liquidation_needed(account) {
            return None;
        }
        
        warn!(
            "Liquidation triggered for account {}: ratio={}",
            account.id,
            account.margin_ratio()
        );
        
        let to_liquidate = self.select_positions_for_liquidation(account);
        
        if to_liquidate.is_empty() {
            return None;
        }
        
        match self.liquidate_positions(account, to_liquidate).await {
            Ok(result) => Some(result),
            Err(e) => {
                error!("Liquidation failed: {}", e);
                None
            }
        }
    }
}

/// Result of liquidation operation
#[derive(Debug, Clone)]
pub struct LiquidationResult {
    pub positions_closed: usize,
    pub total_notional: Decimal,
    pub total_pnl: Decimal,
    pub slippage_cost: Decimal,
    pub timestamp: chrono::DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::margin::position::Position;
    
    #[tokio::test]
    async fn test_liquidation_triggered() {
        let engine = LiquidationEngine::new();
        let mut account = MarginAccount::new(
            "test".to_string(),
            Decimal::from(10000),
        );
        
        // Open highly leveraged position
        account.open_position(Position::new(
            "BTC".to_string(),
            PositionSide::Long,
            Decimal::from(1),
            Decimal::from(50000),
            Decimal::from(5), // 5x leverage for stronger effect
        )).unwrap();
        
        // Price drops 30% - should definitely liquidate
        use std::collections::HashMap;
        let mut prices = HashMap::new();
        prices.insert("BTC".to_string(), Decimal::from(35000)); // -30%
        account.update_prices(&prices);
        
        println!("Equity: {}, Locked: {}, Ratio: {}", 
            account.equity, account.locked_margin, account.margin_ratio());
        
        // Should need liquidation
        assert!(engine.check_liquidation_needed(&account));
        
        // Process liquidation
        let result = engine.process_account(&mut account).await;
        
        assert!(result.is_some());
        let liq = result.unwrap();
        assert_eq!(liq.positions_closed, 1);
        assert!(account.positions.is_empty() || account.status == AccountStatus::Liquidated);
    }
    
    #[test]
    fn test_partial_liquidation_selection() {
        let engine = LiquidationEngine::new();
        let mut account = MarginAccount::new(
            "test".to_string(),
            Decimal::from(12000), // Just enough for both positions
        );
        
        // Multiple positions with high leverage
        account.open_position(Position::new(
            "BTC".to_string(),
            PositionSide::Long,
            Decimal::from(1),
            Decimal::from(50000),
            Decimal::from(10), // $5k margin
        )).unwrap();
        
        account.open_position(Position::new(
            "ETH".to_string(),
            PositionSide::Long,
            Decimal::from(5),
            Decimal::from(3000),
            Decimal::from(10), // $1.5k margin  
        )).unwrap();
        
        // Price drops to create liquidation need
        use std::collections::HashMap;
        let mut prices = HashMap::new();
        prices.insert("BTC".to_string(), Decimal::from(42000)); // -16%
        prices.insert("ETH".to_string(), Decimal::from(2500));   // -17%
        account.update_prices(&prices);
        
        // Should select largest position (BTC) first
        let to_liq = engine.select_positions_for_liquidation(&account);
        assert!(!to_liq.is_empty());
        assert_eq!(to_liq.first().unwrap(), "BTC");
    }
    
    #[tokio::test]
    async fn test_no_liquidation_when_healthy() {
        let engine = LiquidationEngine::new();
        let mut account = MarginAccount::new(
            "test".to_string(),
            Decimal::from(100000),
        );
        
        account.open_position(Position::new(
            "BTC".to_string(),
            PositionSide::Long,
            Decimal::from(1),
            Decimal::from(50000),
            Decimal::from(2), // Conservative 2x leverage
        )).unwrap();
        
        // Small price drop
        use std::collections::HashMap;
        let mut prices = HashMap::new();
        prices.insert("BTC".to_string(), Decimal::from(48000));
        account.update_prices(&prices);
        
        // Should NOT need liquidation
        assert!(!engine.check_liquidation_needed(&account));
        
        let result = engine.process_account(&mut account).await;
        assert!(result.is_none());
    }
}
