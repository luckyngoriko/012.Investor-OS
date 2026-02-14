//! Inventory management for market making

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Inventory position
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryPosition {
    pub symbol: String,
    pub quantity: Decimal,       // Positive = long, Negative = short
    pub avg_entry_price: Decimal,
    pub unrealized_pnl: Decimal,
    pub realized_pnl: Decimal,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl InventoryPosition {
    pub fn new(symbol: impl Into<String>) -> Self {
        Self {
            symbol: symbol.into(),
            quantity: Decimal::ZERO,
            avg_entry_price: Decimal::ZERO,
            unrealized_pnl: Decimal::ZERO,
            realized_pnl: Decimal::ZERO,
            updated_at: chrono::Utc::now(),
        }
    }
    
    /// Add fill to position
    pub fn add_fill(&mut self, qty: Decimal, price: Decimal, is_buy: bool) {
        let signed_qty = if is_buy { qty } else { -qty };
        let new_qty = self.quantity + signed_qty;
        
        if self.quantity.is_zero() {
            // New position
            self.avg_entry_price = price;
        } else if (self.quantity > Decimal::ZERO) == (new_qty > Decimal::ZERO) {
            // Adding to existing position
            let total_cost = self.quantity * self.avg_entry_price + signed_qty * price;
            self.avg_entry_price = total_cost / new_qty.abs();
        } else if new_qty.is_zero() {
            // Position closed
            let pnl = (price - self.avg_entry_price) * self.quantity;
            self.realized_pnl += pnl;
            self.avg_entry_price = Decimal::ZERO;
            self.unrealized_pnl = Decimal::ZERO; // Clear unrealized when closed
        } else {
            // Position flipped
            let closed_qty = self.quantity;
            let pnl = (price - self.avg_entry_price) * closed_qty;
            self.realized_pnl += pnl;
            self.avg_entry_price = price;
        }
        
        self.quantity = new_qty;
        self.updated_at = chrono::Utc::now();
    }
    
    /// Update unrealized P&L with current market price
    pub fn update_unrealized_pnl(&mut self, mark_price: Decimal) {
        if self.quantity.is_zero() {
            self.unrealized_pnl = Decimal::ZERO;
        } else {
            self.unrealized_pnl = (mark_price - self.avg_entry_price) * self.quantity;
        }
        self.updated_at = chrono::Utc::now();
    }
    
    /// Get total P&L
    pub fn total_pnl(&self) -> Decimal {
        self.realized_pnl + self.unrealized_pnl
    }
    
    /// Get position value at given price
    pub fn position_value(&self, price: Decimal) -> Decimal {
        self.quantity * price
    }
    
    /// Check if position is long
    pub fn is_long(&self) -> bool {
        self.quantity > Decimal::ZERO
    }
    
    /// Check if position is short
    pub fn is_short(&self) -> bool {
        self.quantity < Decimal::ZERO
    }
}

/// Inventory limits
#[derive(Debug, Clone)]
pub struct InventoryLimits {
    pub max_position: Decimal,        // Max absolute position
    pub max_notional: Decimal,        // Max position value
    pub target_inventory: Decimal,    // Target inventory ratio (-1 to 1)
    pub skew_factor: Decimal,         // How much to skew quotes
}

impl Default for InventoryLimits {
    fn default() -> Self {
        Self {
            max_position: Decimal::from(100),      // 100 units
            max_notional: Decimal::from(1000000),  // $1M
            target_inventory: Decimal::ZERO,       // Neutral
            skew_factor: Decimal::try_from(0.5).unwrap(),
        }
    }
}

/// Inventory manager
#[derive(Debug)]
pub struct InventoryManager {
    positions: HashMap<String, InventoryPosition>,
    limits: InventoryLimits,
    total_realized_pnl: Decimal,
}

impl InventoryManager {
    pub fn new(limits: InventoryLimits) -> Self {
        Self {
            positions: HashMap::new(),
            limits,
            total_realized_pnl: Decimal::ZERO,
        }
    }
    
    /// Record a fill
    pub fn record_fill(&mut self, symbol: &str, qty: Decimal, price: Decimal, is_buy: bool) {
        let position = self.positions
            .entry(symbol.to_string())
            .or_insert_with(|| InventoryPosition::new(symbol));
        
        let old_realized = position.realized_pnl;
        position.add_fill(qty, price, is_buy);
        
        // Update total realized P&L
        self.total_realized_pnl += position.realized_pnl - old_realized;
    }
    
    /// Update mark prices
    pub fn update_marks(&mut self, marks: &HashMap<String, Decimal>) {
        for (symbol, price) in marks {
            if let Some(position) = self.positions.get_mut(symbol) {
                position.update_unrealized_pnl(*price);
            }
        }
    }
    
    /// Get position
    pub fn get_position(&self, symbol: &str) -> Option<&InventoryPosition> {
        self.positions.get(symbol)
    }
    
    /// Get mutable position
    pub fn get_position_mut(&mut self, symbol: &str) -> Option<&mut InventoryPosition> {
        self.positions.get_mut(symbol)
    }
    
    /// Calculate quote skew based on inventory
    /// Returns adjustment to add to bid/ask (positive = wider spread on that side)
    pub fn calculate_skew(&self, symbol: &str, mid_price: Decimal) -> (Decimal, Decimal) {
        let position = self.get_position(symbol);
        
        if position.is_none() || mid_price.is_zero() {
            return (Decimal::ZERO, Decimal::ZERO);
        }
        
        let pos = position.unwrap();
        let inventory_ratio = pos.quantity / self.limits.max_position;
        
        // Skew quotes to encourage inventory reduction
        // Long position -> lower bid/ask to encourage selling
        // Short position -> raise bid/ask to encourage buying
        let base_skew = inventory_ratio * self.limits.skew_factor * mid_price;
        
        if pos.is_long() {
            // Lower both sides to sell
            (-base_skew, -base_skew)
        } else {
            // Raise both sides to buy
            (-base_skew, -base_skew) // Negative because we're short
        }
    }
    
    /// Check if we can add to position
    pub fn can_add_position(&self, symbol: &str, additional_qty: Decimal) -> bool {
        let current = self.get_position(symbol)
            .map(|p| p.quantity)
            .unwrap_or(Decimal::ZERO);
        
        let new_qty = current + additional_qty;
        new_qty.abs() <= self.limits.max_position
    }
    
    /// Get total inventory value
    pub fn total_inventory_value(&self, prices: &HashMap<String, Decimal>) -> Decimal {
        self.positions.iter()
            .map(|(symbol, pos)| {
                let price = prices.get(symbol).copied().unwrap_or(Decimal::ZERO);
                pos.position_value(price).abs()
            })
            .sum()
    }
    
    /// Get total unrealized P&L
    pub fn total_unrealized_pnl(&self) -> Decimal {
        self.positions.values()
            .map(|p| p.unrealized_pnl)
            .sum()
    }
    
    /// Get total realized P&L
    pub fn total_realized_pnl(&self) -> Decimal {
        self.total_realized_pnl
    }
    
    /// Get all positions
    pub fn positions(&self) -> &HashMap<String, InventoryPosition> {
        &self.positions
    }
    
    /// Get positions needing hedge (outside target range)
    pub fn positions_needing_hedge(&self) -> Vec<&InventoryPosition> {
        self.positions.values()
            .filter(|p| {
                let ratio = p.quantity.abs() / self.limits.max_position;
                ratio > Decimal::try_from(0.8).unwrap() // 80% of limit
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_position_add_fill() {
        let mut pos = InventoryPosition::new("BTC");
        
        // Buy 1 BTC at $50k
        pos.add_fill(Decimal::ONE, Decimal::from(50000), true);
        assert_eq!(pos.quantity, Decimal::ONE);
        assert_eq!(pos.avg_entry_price, Decimal::from(50000));
        
        // Buy another 1 BTC at $51k
        pos.add_fill(Decimal::ONE, Decimal::from(51000), true);
        assert_eq!(pos.quantity, Decimal::from(2));
        // Avg = (50000 + 51000) / 2 = 50500
        assert_eq!(pos.avg_entry_price, Decimal::from(50500));
    }
    
    #[test]
    fn test_position_close() {
        let mut pos = InventoryPosition::new("BTC");
        
        // Buy then sell
        pos.add_fill(Decimal::ONE, Decimal::from(50000), true);
        pos.add_fill(Decimal::ONE, Decimal::from(51000), false);
        
        assert!(pos.quantity.is_zero());
        assert_eq!(pos.realized_pnl, Decimal::from(1000)); // $1k profit
    }
    
    #[test]
    fn test_position_flip() {
        let mut pos = InventoryPosition::new("BTC");
        
        // Buy 1, then sell 2 (flip to short)
        pos.add_fill(Decimal::ONE, Decimal::from(50000), true);
        pos.add_fill(Decimal::from(2), Decimal::from(51000), false);
        
        assert_eq!(pos.quantity, Decimal::from(-1)); // Short 1 BTC
        assert_eq!(pos.realized_pnl, Decimal::from(1000));
        assert_eq!(pos.avg_entry_price, Decimal::from(51000));
    }
    
    #[test]
    fn test_update_unrealized_pnl() {
        let mut pos = InventoryPosition::new("BTC");
        pos.add_fill(Decimal::ONE, Decimal::from(50000), true);
        
        pos.update_unrealized_pnl(Decimal::from(51000));
        
        // Unrealized P&L = (51000 - 50000) * 1 = $1000
        assert_eq!(pos.unrealized_pnl, Decimal::from(1000));
    }
    
    #[test]
    fn test_inventory_manager() {
        let limits = InventoryLimits::default();
        let mut manager = InventoryManager::new(limits);
        
        manager.record_fill("BTC", Decimal::ONE, Decimal::from(50000), true);
        
        let pos = manager.get_position("BTC").unwrap();
        assert_eq!(pos.quantity, Decimal::ONE);
        
        // Check can add more
        assert!(manager.can_add_position("BTC", Decimal::from(50)));
        
        // Check cannot exceed limit
        assert!(!manager.can_add_position("BTC", Decimal::from(200)));
    }
    
    #[test]
    fn test_calculate_skew() {
        let limits = InventoryLimits {
            max_position: Decimal::from(100),
            skew_factor: Decimal::ONE,
            ..Default::default()
        };
        let mut manager = InventoryManager::new(limits);
        
        manager.record_fill("BTC", Decimal::from(50), Decimal::from(50000), true);
        
        let (bid_skew, ask_skew) = manager.calculate_skew("BTC", Decimal::from(50000));
        
        // Long position should skew negative (lower prices to sell)
        assert!(bid_skew < Decimal::ZERO);
        assert!(ask_skew < Decimal::ZERO);
    }
    
    #[test]
    fn test_positions_needing_hedge() {
        let limits = InventoryLimits {
            max_position: Decimal::from(100),
            ..Default::default()
        };
        let mut manager = InventoryManager::new(limits);
        
        manager.record_fill("BTC", Decimal::from(85), Decimal::from(50000), true);
        manager.record_fill("ETH", Decimal::from(50), Decimal::from(3000), true);
        
        let need_hedge = manager.positions_needing_hedge();
        
        // Only BTC is at 85% (>80% threshold)
        assert_eq!(need_hedge.len(), 1);
        assert_eq!(need_hedge[0].symbol, "BTC");
    }
}
