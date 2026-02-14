//! Margin account with equity and leverage tracking

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use std::collections::HashMap;

use super::position::{Position, PositionSide};
use super::error::{MarginError, Result};

/// Margin account status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccountStatus {
    Active,
    MarginCall,     // Below maintenance margin
    Liquidating,    // Positions being liquidated
    Liquidated,     // All positions closed
}

/// Margin account for leveraged trading
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarginAccount {
    pub id: Uuid,
    pub owner_id: String,
    pub status: AccountStatus,
    
    // Capital
    pub equity: Decimal,           // Total account value
    pub available_margin: Decimal, // Free to use for new positions
    pub locked_margin: Decimal,    // Used by open positions
    
    // Leverage settings
    pub max_leverage: Decimal,     // Max allowed leverage (e.g., 20x)
    pub maintenance_margin_rate: Decimal, // Min margin ratio (e.g., 0.5 = 50%)
    
    // Positions
    pub positions: HashMap<String, Position>, // symbol -> Position
    
    // Tracking
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub total_traded_volume: Decimal,
}

impl MarginAccount {
    /// Create new margin account
    pub fn new(owner_id: String, initial_capital: Decimal) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            owner_id,
            status: AccountStatus::Active,
            equity: initial_capital,
            available_margin: initial_capital,
            locked_margin: Decimal::ZERO,
            max_leverage: Decimal::from(20),       // Default 20x max
            maintenance_margin_rate: Decimal::try_from(0.05).unwrap(), // 5% maintenance
            positions: HashMap::new(),
            created_at: now,
            updated_at: now,
            total_traded_volume: Decimal::ZERO,
        }
    }
    
    /// Set max leverage (with safety limits)
    pub fn set_max_leverage(&mut self, leverage: Decimal) -> Result<()> {
        let max_allowed = Decimal::from(100); // Hard limit at 100x
        
        if leverage > max_allowed || leverage < Decimal::ONE {
            return Err(MarginError::InvalidLeverage(leverage, max_allowed));
        }
        
        self.max_leverage = leverage;
        self.updated_at = Utc::now();
        Ok(())
    }
    
    /// Open new position
    pub fn open_position(&mut self, position: Position) -> Result<()> {
        if self.status == AccountStatus::Liquidated {
            return Err(MarginError::AccountLiquidated { 
                equity: self.equity 
            });
        }
        
        let margin_required = position.margin_used();
        
        // Check available margin
        if margin_required > self.available_margin {
            return Err(MarginError::InsufficientMargin {
                required: margin_required,
                available: self.available_margin,
            });
        }
        
        // Check leverage limit
        if position.leverage > self.max_leverage {
            return Err(MarginError::InvalidLeverage(
                position.leverage, 
                self.max_leverage
            ));
        }
        
        // Lock margin and add position
        self.locked_margin += margin_required;
        self.available_margin -= margin_required;
        self.total_traded_volume += position.notional_value();
        
        // Add or update position
        self.positions.insert(position.symbol.clone(), position);
        self.updated_at = Utc::now();
        
        Ok(())
    }
    
    /// Close position
    pub fn close_position(&mut self, symbol: &str, exit_price: Decimal) -> Result<Decimal> {
        let position = self.positions.remove(symbol)
            .ok_or_else(|| MarginError::PositionNotFound(symbol.to_string()))?;
        
        // Calculate final P&L
        let pnl = {
            let mut pos = position.clone();
            pos.update_price(exit_price);
            pos.unrealized_pnl()
        };
        
        // Release margin
        let margin_released = position.margin_used();
        self.locked_margin -= margin_released;
        self.available_margin += margin_released + pnl;
        self.equity += pnl;
        
        self.updated_at = Utc::now();
        Ok(pnl)
    }
    
    /// Update all position prices and recalculate equity
    pub fn update_prices(&mut self, prices: &HashMap<String, Decimal>) {
        let mut total_pnl = Decimal::ZERO;
        
        for (symbol, position) in self.positions.iter_mut() {
            if let Some(price) = prices.get(symbol) {
                position.update_price(*price);
                total_pnl += position.unrealized_pnl();
            }
        }
        
        // Equity = initial + unrealized P&L (simplified)
        self.equity = self.available_margin + self.locked_margin + total_pnl;
        self.updated_at = Utc::now();
    }
    
    /// Calculate margin ratio (equity / locked_margin)
    pub fn margin_ratio(&self) -> Decimal {
        if self.locked_margin.is_zero() {
            return Decimal::MAX; // No positions open
        }
        
        self.equity / self.locked_margin
    }
    
    /// Check if margin call is triggered
    pub fn is_margin_call(&self) -> bool {
        let min_ratio = Decimal::ONE + self.maintenance_margin_rate;
        self.margin_ratio() < min_ratio && !self.locked_margin.is_zero()
    }
    
    /// Get total notional exposure
    pub fn total_exposure(&self) -> Decimal {
        self.positions.values()
            .map(|p| p.notional_value())
            .fold(Decimal::ZERO, |acc, v| acc + v)
    }
    
    /// Get portfolio delta (directional exposure)
    pub fn net_exposure(&self) -> Decimal {
        self.positions.values()
            .map(|p| {
                match p.side {
                    PositionSide::Long => p.notional_value(),
                    PositionSide::Short => -p.notional_value(),
                }
            })
            .fold(Decimal::ZERO, |acc, v| acc + v)
    }
    
    /// Deposit capital
    pub fn deposit(&mut self, amount: Decimal) {
        self.equity += amount;
        self.available_margin += amount;
        self.updated_at = Utc::now();
    }
    
    /// Withdraw capital (if available)
    pub fn withdraw(&mut self, amount: Decimal) -> Result<()> {
        if amount > self.available_margin {
            return Err(MarginError::InsufficientMargin {
                required: amount,
                available: self.available_margin,
            });
        }
        
        self.equity -= amount;
        self.available_margin -= amount;
        self.updated_at = Utc::now();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_new_account() {
        let account = MarginAccount::new(
            "trader_001".to_string(),
            Decimal::from(100000),
        );
        
        assert_eq!(account.equity, Decimal::from(100000));
        assert_eq!(account.available_margin, Decimal::from(100000));
        assert!(account.positions.is_empty());
        assert_eq!(account.max_leverage, Decimal::from(20));
    }
    
    #[test]
    fn test_open_position_reduces_available_margin() {
        let mut account = MarginAccount::new(
            "trader_001".to_string(),
            Decimal::from(100000),
        );
        
        let position = Position::new(
            "BTC".to_string(),
            PositionSide::Long,
            Decimal::from(1),
            Decimal::from(50000),
            Decimal::from(5),
        );
        
        account.open_position(position).unwrap();
        
        // Margin used = 50000 / 5 = $10,000
        assert_eq!(account.locked_margin, Decimal::from(10000));
        assert_eq!(account.available_margin, Decimal::from(90000));
    }
    
    #[test]
    fn test_insufficient_margin() {
        let mut account = MarginAccount::new(
            "trader_001".to_string(),
            Decimal::from(5000), // Only $5k
        );
        
        let position = Position::new(
            "BTC".to_string(),
            PositionSide::Long,
            Decimal::from(1),
            Decimal::from(50000),
            Decimal::from(2), // Needs $25k margin
        );
        
        let result = account.open_position(position);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_close_position_realizes_pnl() {
        let mut account = MarginAccount::new(
            "trader_001".to_string(),
            Decimal::from(100000),
        );
        
        let position = Position::new(
            "BTC".to_string(),
            PositionSide::Long,
            Decimal::from(1),
            Decimal::from(50000),
            Decimal::from(5),
        );
        
        account.open_position(position).unwrap();
        
        // Close at profit
        let pnl = account.close_position("BTC", Decimal::from(55000)).unwrap();
        
        // P&L = (55000 - 50000) * 1 = $5000
        assert_eq!(pnl, Decimal::from(5000));
        
        // Equity increased by P&L
        assert_eq!(account.equity, Decimal::from(105000));
        
        // Position removed
        assert!(account.positions.is_empty());
    }
    
    #[test]
    fn test_margin_call_detection() {
        let mut account = MarginAccount::new(
            "trader_001".to_string(),
            Decimal::from(10000),
        );
        
        let position = Position::new(
            "BTC".to_string(),
            PositionSide::Long,
            Decimal::from(1),
            Decimal::from(50000),
            Decimal::from(10), // 10x leverage, margin = $5,000
        );
        
        account.open_position(position).unwrap();
        
        // Update price down 20% to $40,000
        // Equity = $10,000 - $10,000 loss = ~$0
        let mut prices = HashMap::new();
        prices.insert("BTC".to_string(), Decimal::from(40000));
        account.update_prices(&prices);
        
        // Should trigger margin call
        assert!(account.is_margin_call());
    }
}
