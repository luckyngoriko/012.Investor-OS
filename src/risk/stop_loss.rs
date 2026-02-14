//! Stop-loss and take-profit management

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use std::collections::HashMap;
use tracing::info;
use uuid::Uuid;

use super::Result;

/// Stop-loss order types
#[derive(Debug, Clone, Copy, PartialEq)]
#[derive(Default)]
pub enum StopLossType {
    /// Fixed price stop-loss
    #[default]
    Fixed,
    /// Trailing stop-loss (follows price)
    Trailing,
    /// ATR-based stop-loss
    AtrBased,
    /// Time-based stop (exit after duration)
    TimeBased,
}


/// Stop-loss configuration
#[derive(Debug, Clone)]
pub struct StopLossConfig {
    pub stop_type: StopLossType,
    /// Fixed stop price or percentage
    pub stop_price: Option<Decimal>,
    /// Trailing distance (for trailing stops)
    pub trailing_distance: Decimal,
    /// ATR multiplier (for ATR-based stops)
    pub atr_multiplier: Decimal,
    /// Time limit in seconds (for time-based stops)
    pub time_limit_seconds: i64,
}

impl Default for StopLossConfig {
    fn default() -> Self {
        Self {
            stop_type: StopLossType::Fixed,
            stop_price: None,
            trailing_distance: Decimal::try_from(0.05).unwrap(), // 5%
            atr_multiplier: Decimal::from(2),
            time_limit_seconds: 86400, // 24 hours
        }
    }
}

/// Take-profit configuration with multiple targets
#[derive(Debug, Clone)]
pub struct TakeProfitConfig {
    /// Multiple profit targets (price/percentage, position percentage to close)
    pub targets: Vec<(Decimal, Decimal)>,
    /// Move stop to breakeven after first target
    pub move_to_breakeven: bool,
    /// Trailing take profit after final target
    pub trailing_take_profit: bool,
}

impl Default for TakeProfitConfig {
    fn default() -> Self {
        Self {
            targets: vec![
                (Decimal::try_from(1.02).unwrap(), Decimal::try_from(0.25).unwrap()), // +2%, close 25%
                (Decimal::try_from(1.04).unwrap(), Decimal::try_from(0.25).unwrap()), // +4%, close 25%
                (Decimal::try_from(1.06).unwrap(), Decimal::try_from(0.25).unwrap()), // +6%, close 25%
                (Decimal::try_from(1.10).unwrap(), Decimal::try_from(0.25).unwrap()), // +10%, close 25%
            ],
            move_to_breakeven: true,
            trailing_take_profit: false,
        }
    }
}

/// Active stop-loss order
#[derive(Debug, Clone)]
pub struct StopLossOrder {
    pub id: String,
    pub position_id: String,
    pub symbol: String,
    pub stop_price: Decimal,
    pub trigger_price: Decimal, // For trailing stops, this moves
    pub stop_type: StopLossType,
    pub entry_price: Decimal,
    pub quantity: Decimal,
    pub is_long: bool,
    pub created_at: DateTime<Utc>,
    pub breakeven_triggered: bool,
}

/// Stop-loss manager handles all stop-loss orders
#[derive(Debug, Clone, Default)]
pub struct StopLossManager {
    orders: HashMap<String, StopLossOrder>,
    by_position: HashMap<String, String>, // position_id -> order_id
}

impl StopLossManager {
    /// Create a new stop-loss manager
    pub fn new() -> Self {
        Self {
            orders: HashMap::new(),
            by_position: HashMap::new(),
        }
    }

    /// Create a stop-loss order for a position
    pub fn create_stop_loss(
        &mut self,
        position_id: String,
        symbol: String,
        entry_price: Decimal,
        quantity: Decimal,
        is_long: bool,
        config: StopLossConfig,
        current_atr: Option<Decimal>,
    ) -> Result<String> {
        let stop_price = match config.stop_type {
            StopLossType::Fixed => {
                config.stop_price.ok_or_else(|| {
                    super::RiskError::CalculationError(
                        "Stop price required for fixed stop".to_string(),
                    )
                })?
            }
            StopLossType::Trailing => {
                // Initial stop is entry - trailing_distance
                if is_long {
                    entry_price * (Decimal::ONE - config.trailing_distance)
                } else {
                    entry_price * (Decimal::ONE + config.trailing_distance)
                }
            }
            StopLossType::AtrBased => {
                let atr = current_atr.ok_or_else(|| {
                    super::RiskError::CalculationError("ATR required for ATR-based stop".to_string())
                })?;
                if is_long {
                    entry_price - atr * config.atr_multiplier
                } else {
                    entry_price + atr * config.atr_multiplier
                }
            }
            StopLossType::TimeBased => {
                // Time-based stops don't have a price trigger
                Decimal::ZERO
            }
        };

        let trigger_price = stop_price;

        let symbol_clone = symbol.clone();
        let order = StopLossOrder {
            id: Uuid::new_v4().to_string(),
            position_id: position_id.clone(),
            symbol,
            stop_price,
            trigger_price,
            stop_type: config.stop_type,
            entry_price,
            quantity,
            is_long,
            created_at: Utc::now(),
            breakeven_triggered: false,
        };

        let order_id = order.id.clone();
        self.orders.insert(order_id.clone(), order);
        self.by_position.insert(position_id, order_id.clone());

        info!(
            "Created {} stop-loss for {} at {}, entry={}",
            if is_long { "long" } else { "short" },
            symbol_clone,
            stop_price,
            entry_price
        );

        Ok(order_id)
    }

    /// Update stop-loss price (e.g., for trailing stops)
    pub fn update_stop_price(&mut self, order_id: &str, new_price: Decimal) -> Result<()> {
        if let Some(order) = self.orders.get_mut(order_id) {
            let old_price = order.stop_price;
            
            // For trailing stops, only move in favorable direction
            match order.stop_type {
                StopLossType::Trailing => {
                    if order.is_long && new_price > old_price {
                        order.stop_price = new_price;
                        order.trigger_price = new_price;
                        info!("Trailing stop updated: {} -> {}", old_price, new_price);
                    } else if !order.is_long && new_price < old_price {
                        order.stop_price = new_price;
                        order.trigger_price = new_price;
                        info!("Trailing stop updated: {} -> {}", old_price, new_price);
                    }
                }
                _ => {
                    order.stop_price = new_price;
                }
            }
            Ok(())
        } else {
            Err(super::RiskError::CalculationError(
                format!("Stop-loss order {} not found", order_id)
            ))
        }
    }

    /// Check if stop-loss should be triggered
    pub fn check_trigger(&self, order_id: &str, current_price: Decimal) -> Option<StopLossOrder> {
        if let Some(order) = self.orders.get(order_id) {
            let triggered = match order.stop_type {
                StopLossType::TimeBased => {
                    let elapsed = Utc::now() - order.created_at;
                    elapsed.num_seconds() > self
                        .get_config_for_order(order_id)
                        .map(|c| c.time_limit_seconds)
                        .unwrap_or(86400)
                }
                _ => {
                    if order.is_long {
                        current_price <= order.trigger_price
                    } else {
                        current_price >= order.trigger_price
                    }
                }
            };

            if triggered {
                return Some(order.clone());
            }
        }
        None
    }

    /// Update trailing stop based on new price
    pub fn update_trailing_stop(&mut self, order_id: &str, current_price: Decimal, config: &StopLossConfig) -> Result<bool> {
        if let Some(order) = self.orders.get(order_id) {
            if order.stop_type != StopLossType::Trailing {
                return Ok(false);
            }

            // Calculate new potential stop price
            let new_stop = if order.is_long {
                current_price * (Decimal::ONE - config.trailing_distance)
            } else {
                current_price * (Decimal::ONE + config.trailing_distance)
            };

            // Only update if new stop is better (higher for longs, lower for shorts)
            let should_update = if order.is_long {
                new_stop > order.stop_price
            } else {
                new_stop < order.stop_price
            };

            if should_update {
                self.update_stop_price(order_id, new_stop)?;
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Move stop to breakeven (after profit target hit)
    pub fn move_to_breakeven(&mut self, order_id: &str) -> Result<bool> {
        if let Some(order) = self.orders.get_mut(order_id) {
            if order.breakeven_triggered {
                return Ok(false);
            }

            let breakeven = order.entry_price;
            
            // Only move if it improves the stop
            let should_move = if order.is_long {
                breakeven > order.stop_price
            } else {
                breakeven < order.stop_price
            };

            if should_move {
                order.stop_price = breakeven;
                order.trigger_price = breakeven;
                order.breakeven_triggered = true;
                info!("Moved stop to breakeven for order {}", order_id);
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Cancel a stop-loss order
    pub fn cancel_stop_loss(&mut self, order_id: &str) -> Option<StopLossOrder> {
        if let Some(order) = self.orders.remove(order_id) {
            self.by_position.remove(&order.position_id);
            info!("Cancelled stop-loss order {}", order_id);
            Some(order)
        } else {
            None
        }
    }

    /// Cancel stop-loss for a position
    pub fn cancel_for_position(&mut self, position_id: &str) -> Option<StopLossOrder> {
        if let Some(order_id) = self.by_position.get(position_id).cloned() {
            self.cancel_stop_loss(&order_id)
        } else {
            None
        }
    }

    /// Get stop-loss order by ID
    pub fn get_order(&self, order_id: &str) -> Option<&StopLossOrder> {
        self.orders.get(order_id)
    }

    /// Get stop-loss for a position
    pub fn get_for_position(&self, position_id: &str) -> Option<&StopLossOrder> {
        self.by_position
            .get(position_id)
            .and_then(|id| self.orders.get(id))
    }

    /// Get all active stop-loss orders
    pub fn get_all_orders(&self) -> &HashMap<String, StopLossOrder> {
        &self.orders
    }

    // Helper to get config (simplified - in real impl, store config with order)
    fn get_config_for_order(&self, _order_id: &str) -> Option<StopLossConfig> {
        Some(StopLossConfig::default())
    }

    /// Calculate take-profit levels
    pub fn calculate_take_profit_levels(
        entry_price: Decimal,
        is_long: bool,
        config: &TakeProfitConfig,
    ) -> Vec<(Decimal, Decimal)> {
        config
            .targets
            .iter()
            .map(|(multiplier, close_pct)| {
                let price = if is_long {
                    entry_price * multiplier
                } else {
                    entry_price / multiplier
                };
                (price, *close_pct)
            })
            .collect()
    }

    /// Check if any take-profit target should be triggered
    pub fn check_take_profit(
        current_price: Decimal,
        entry_price: Decimal,
        is_long: bool,
        config: &TakeProfitConfig,
    ) -> Vec<(Decimal, Decimal)> {
        let mut triggered = Vec::new();

        for (multiplier, close_pct) in &config.targets {
            let target_price = if is_long {
                entry_price * multiplier
            } else {
                entry_price / multiplier
            };

            let hit = if is_long {
                current_price >= target_price
            } else {
                current_price <= target_price
            };

            if hit {
                triggered.push((*close_pct, target_price));
            }
        }

        triggered
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_fixed_stop_loss() {
        let mut manager = StopLossManager::new();
        let config = StopLossConfig {
            stop_type: StopLossType::Fixed,
            stop_price: Some(Decimal::from(90)),
            ..Default::default()
        };

        let order_id = manager
            .create_stop_loss(
                "pos1".to_string(),
                "BTC".to_string(),
                Decimal::from(100),
                Decimal::from(10),
                true,
                config,
                None,
            )
            .unwrap();

        let order = manager.get_order(&order_id).unwrap();
        assert_eq!(order.stop_price, Decimal::from(90));
        assert_eq!(order.is_long, true);
    }

    #[test]
    fn test_trailing_stop_creation() {
        let mut manager = StopLossManager::new();
        let config = StopLossConfig {
            stop_type: StopLossType::Trailing,
            trailing_distance: Decimal::try_from(0.05).unwrap(),
            ..Default::default()
        };

        let order_id = manager
            .create_stop_loss(
                "pos1".to_string(),
                "BTC".to_string(),
                Decimal::from(100),
                Decimal::from(10),
                true,
                config,
                None,
            )
            .unwrap();

        let order = manager.get_order(&order_id).unwrap();
        // Stop should be 5% below entry: 100 * 0.95 = 95
        assert_eq!(order.stop_price, Decimal::from(95));
    }

    #[test]
    fn test_stop_loss_trigger() {
        let mut manager = StopLossManager::new();
        let config = StopLossConfig {
            stop_type: StopLossType::Fixed,
            stop_price: Some(Decimal::from(90)),
            ..Default::default()
        };

        let order_id = manager
            .create_stop_loss(
                "pos1".to_string(),
                "BTC".to_string(),
                Decimal::from(100),
                Decimal::from(10),
                true,
                config,
                None,
            )
            .unwrap();

        // Price at stop should trigger
        assert!(manager.check_trigger(&order_id, Decimal::from(90)).is_some());
        // Price below stop should trigger
        assert!(manager.check_trigger(&order_id, Decimal::from(89)).is_some());
        // Price above stop should not trigger
        assert!(manager.check_trigger(&order_id, Decimal::from(91)).is_none());
    }

    #[test]
    fn test_update_trailing_stop() {
        let mut manager = StopLossManager::new();
        let config = StopLossConfig {
            stop_type: StopLossType::Trailing,
            trailing_distance: Decimal::try_from(0.05).unwrap(),
            ..Default::default()
        };

        let order_id = manager
            .create_stop_loss(
                "pos1".to_string(),
                "BTC".to_string(),
                Decimal::from(100),
                Decimal::from(10),
                true,
                config.clone(),
                None,
            )
            .unwrap();

        // Initial stop at 95
        assert_eq!(manager.get_order(&order_id).unwrap().stop_price, Decimal::from(95));

        // Price rises to 110, trailing stop should update to 104.5
        let updated = manager.update_trailing_stop(&order_id, Decimal::from(110), &config).unwrap();
        assert!(updated);
        
        let new_stop = manager.get_order(&order_id).unwrap().stop_price;
        // 110 * 0.95 = 104.5
        assert_eq!(new_stop, Decimal::try_from(104.5).unwrap());

        // Price drops, stop should not move down
        let updated = manager.update_trailing_stop(&order_id, Decimal::from(105), &config).unwrap();
        assert!(!updated);
        assert_eq!(manager.get_order(&order_id).unwrap().stop_price, new_stop);
    }

    #[test]
    fn test_move_to_breakeven() {
        let mut manager = StopLossManager::new();
        let config = StopLossConfig {
            stop_type: StopLossType::Fixed,
            stop_price: Some(Decimal::from(95)),
            ..Default::default()
        };

        let order_id = manager
            .create_stop_loss(
                "pos1".to_string(),
                "BTC".to_string(),
                Decimal::from(100),
                Decimal::from(10),
                true,
                config,
                None,
            )
            .unwrap();

        // Set initial stop lower
        manager.update_stop_price(&order_id, Decimal::from(90)).unwrap();
        
        // Move to breakeven
        let moved = manager.move_to_breakeven(&order_id).unwrap();
        assert!(moved);
        
        let order = manager.get_order(&order_id).unwrap();
        assert_eq!(order.stop_price, Decimal::from(100)); // Entry price
        assert!(order.breakeven_triggered);

        // Second attempt should not move
        let moved = manager.move_to_breakeven(&order_id).unwrap();
        assert!(!moved);
    }

    #[test]
    fn test_take_profit_calculation() {
        let config = TakeProfitConfig::default();
        let entry = Decimal::from(100);

        let levels = StopLossManager::calculate_take_profit_levels(entry, true, &config);

        assert_eq!(levels.len(), 4);
        // First target: 100 * 1.02 = 102
        assert_eq!(levels[0].0, Decimal::try_from(102.0).unwrap());
        assert_eq!(levels[0].1, Decimal::try_from(0.25).unwrap());
    }

    #[test]
    fn test_check_take_profit() {
        let config = TakeProfitConfig::default();
        let entry = Decimal::from(100);

        // Price hits 103, should trigger first target
        let triggered = StopLossManager::check_take_profit(
            Decimal::from(103),
            entry,
            true,
            &config,
        );

        assert_eq!(triggered.len(), 1);
        assert_eq!(triggered[0].0, Decimal::try_from(0.25).unwrap()); // 25% to close
    }
}
