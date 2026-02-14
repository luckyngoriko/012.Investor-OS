//! Position tracking with P&L calculation

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Position side
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PositionSide {
    Long,
    Short,
}

/// Trading position
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub id: Uuid,
    pub symbol: String,
    pub side: PositionSide,
    pub quantity: Decimal,
    pub entry_price: Decimal,
    pub current_price: Decimal,
    pub leverage: Decimal,
    pub opened_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Position {
    /// Create new position
    pub fn new(
        symbol: String,
        side: PositionSide,
        quantity: Decimal,
        entry_price: Decimal,
        leverage: Decimal,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            symbol,
            side,
            quantity,
            entry_price,
            current_price: entry_price,
            leverage,
            opened_at: now,
            updated_at: now,
        }
    }
    
    /// Update current price
    pub fn update_price(&mut self, price: Decimal) {
        self.current_price = price;
        self.updated_at = Utc::now();
    }
    
    /// Calculate unrealized P&L
    pub fn unrealized_pnl(&self) -> Decimal {
        let price_diff = match self.side {
            PositionSide::Long => self.current_price - self.entry_price,
            PositionSide::Short => self.entry_price - self.current_price,
        };
        
        price_diff * self.quantity
    }
    
    /// Calculate position value (notional)
    pub fn notional_value(&self) -> Decimal {
        self.current_price * self.quantity
    }
    
    /// Calculate margin used by this position
    pub fn margin_used(&self) -> Decimal {
        self.notional_value() / self.leverage
    }
    
    /// Calculate P&L percentage
    pub fn pnl_percentage(&self) -> Decimal {
        if self.entry_price.is_zero() {
            return Decimal::ZERO;
        }
        
        let price_diff = match self.side {
            PositionSide::Long => self.current_price - self.entry_price,
            PositionSide::Short => self.entry_price - self.current_price,
        };
        
        (price_diff / self.entry_price) * Decimal::from(100)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_long_position_pnl() {
        let mut pos = Position::new(
            "BTC".to_string(),
            PositionSide::Long,
            Decimal::from(1),
            Decimal::from(50000),
            Decimal::from(5), // 5x leverage
        );
        
        // Price goes up 10%
        pos.update_price(Decimal::from(55000));
        
        // P&L = (55000 - 50000) * 1 = $5000
        assert_eq!(pos.unrealized_pnl(), Decimal::from(5000));
        
        // Margin used = 55000 / 5 = $11000
        assert_eq!(pos.margin_used(), Decimal::from(11000));
    }
    
    #[test]
    fn test_short_position_pnl() {
        let mut pos = Position::new(
            "BTC".to_string(),
            PositionSide::Short,
            Decimal::from(1),
            Decimal::from(50000),
            Decimal::from(5),
        );
        
        // Price goes down 10% - good for short
        pos.update_price(Decimal::from(45000));
        
        // P&L = (50000 - 45000) * 1 = $5000 profit
        assert_eq!(pos.unrealized_pnl(), Decimal::from(5000));
    }
    
    #[test]
    fn test_leverage_margin_calculation() {
        let pos = Position::new(
            "ETH".to_string(),
            PositionSide::Long,
            Decimal::try_from(10.0).unwrap(),
            Decimal::from(3000),
            Decimal::from(10), // 10x leverage
        );
        
        // Notional = 3000 * 10 = $30,000
        assert_eq!(pos.notional_value(), Decimal::from(30000));
        
        // Margin = 30000 / 10 = $3,000
        assert_eq!(pos.margin_used(), Decimal::from(3000));
    }
}
