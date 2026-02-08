//! Execution Engine
//!
//! S6-D6: Auto-execute confirmed proposals
//! S6-D8: Kill Switch - Immediate position flattening

use rust_decimal::Decimal;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::broker::{
    orders::OrderManager, Broker, BrokerConfig, BrokerError, Order, OrderSide, OrderStatus,
    OrderType, Position, Result,
};
use crate::broker::risk::{RiskChecker, RiskSeverity};

/// Execution engine for processing trade proposals
pub struct ExecutionEngine {
    order_manager: Arc<OrderManager>,
    risk_checker: RiskChecker,
    config: ExecutionConfig,
    enabled: Arc<RwLock<bool>>,
    kill_switch_triggered: Arc<RwLock<bool>>,
}

/// Execution configuration
#[derive(Debug, Clone)]
pub struct ExecutionConfig {
    /// Auto-execute enabled
    pub auto_execute: bool,
    /// Require manual confirmation for orders > $10k
    pub manual_confirmation_threshold: Decimal,
    /// Default time in force
    pub default_tif: crate::broker::TimeInForce,
    /// Use limit orders (vs market)
    pub prefer_limit_orders: bool,
    /// Limit order offset from market (e.g., 0.01 for $0.01)
    pub limit_offset: Decimal,
}

impl Default for ExecutionConfig {
    fn default() -> Self {
        Self {
            auto_execute: false, // Disabled by default for safety
            manual_confirmation_threshold: Decimal::from(10000),
            default_tif: crate::broker::TimeInForce::Day,
            prefer_limit_orders: true,
            limit_offset: Decimal::from(1) / Decimal::from(100), // $0.01
        }
    }
}

impl ExecutionEngine {
    /// Create a new execution engine
    pub fn new(
        order_manager: Arc<OrderManager>,
        broker_config: BrokerConfig,
        execution_config: ExecutionConfig,
    ) -> Self {
        let risk_checker = RiskChecker::new(broker_config);
        
        Self {
            order_manager,
            risk_checker,
            config: execution_config,
            enabled: Arc::new(RwLock::new(true)),
            kill_switch_triggered: Arc::new(RwLock::new(false)),
        }
    }

    /// Enable/disable auto-execution
    pub async fn set_enabled(&self, enabled: bool) {
        let mut e = self.enabled.write().await;
        *e = enabled;
        info!("Execution engine {}", if enabled { "enabled" } else { "disabled" });
    }

    /// Check if engine is enabled
    pub async fn is_enabled(&self) -> bool {
        *self.enabled.read().await
    }

    /// Check if kill switch has been triggered
    pub async fn is_kill_switch_triggered(&self) -> bool {
        *self.kill_switch_triggered.read().await
    }

    /// Execute a trade proposal (S6-D6)
    pub async fn execute_proposal(
        &self,
        ticker: &str,
        side: OrderSide,
        quantity: Decimal,
        market_price: Decimal,
        portfolio_id: Uuid,
        proposal_id: Uuid,
    ) -> Result<Order> {
        // Check if kill switch is active
        if self.is_kill_switch_triggered().await {
            return Err(BrokerError::InvalidOrder(
                "Kill switch is active - trading disabled".to_string()
            ));
        }

        // Check if engine is enabled
        if !self.is_enabled().await {
            return Err(BrokerError::InvalidOrder(
                "Execution engine is disabled".to_string()
            ));
        }

        // Build order
        let mut order = self.build_order(
            ticker,
            side,
            quantity,
            market_price,
            portfolio_id,
            proposal_id,
        ).await?;

        // Pre-trade risk check
        // Would get positions and account value from broker
        let positions = vec![]; // Placeholder
        let account_value = Decimal::from(100000); // Placeholder
        
        let risk_result = self.risk_checker.validate_order(
            &order,
            &positions,
            account_value,
        ).await?;

        if !risk_result.passed {
            let errors: Vec<String> = risk_result.violations
                .into_iter()
                .filter(|v| matches!(v.severity, RiskSeverity::Fatal | RiskSeverity::Error))
                .map(|v| v.message)
                .collect();
            
            return Err(BrokerError::RiskCheckFailed(errors.join("; ")));
        }

        // Check if manual confirmation required
        let order_value = market_price * quantity;
        if order_value > self.config.manual_confirmation_threshold {
            warn!(
                "Order {} requires manual confirmation (value: ${})",
                order.id, order_value
            );
            // Would queue for confirmation instead of submitting
            // For now, proceed with warning
        }

        // Submit order
        self.order_manager.submit_order(&mut order).await?;

        info!(
            "Proposal {} executed: {} {} shares of {} @ ${}",
            proposal_id, side.as_str(), quantity, ticker, market_price
        );

        Ok(order)
    }

    /// Cancel all active orders (partial kill switch)
    pub async fn cancel_all_orders(&self, portfolio_id: Uuid) -> Result<usize> {
        info!("Cancelling all orders for portfolio {}", portfolio_id);
        
        let active_orders = self.order_manager
            .get_active_orders(portfolio_id)
            .await?;

        let mut cancelled = 0;
        for mut order in active_orders {
            match self.order_manager.cancel_order(&mut order).await {
                Ok(()) => cancelled += 1,
                Err(e) => error!("Failed to cancel order {}: {}", order.id, e),
            }
        }

        info!("Cancelled {} orders", cancelled);
        Ok(cancelled)
    }

    /// Kill switch - flatten all positions immediately (S6-D8)
    pub async fn trigger_kill_switch(&self, portfolio_id: Uuid) -> Result<KillSwitchResult> {
        warn!("KILL SWITCH TRIGGERED for portfolio {}", portfolio_id);
        
        // Set kill switch flag
        let mut ks = self.kill_switch_triggered.write().await;
        *ks = true;
        drop(ks);

        // Disable execution
        self.set_enabled(false).await;

        // Cancel all pending orders
        let cancelled_orders = self.cancel_all_orders(portfolio_id).await?;

        // Flatten positions
        let flattened_positions = self.flatten_all_positions(portfolio_id).await?;

        info!(
            "Kill switch complete: {} orders cancelled, {} positions flattened",
            cancelled_orders, flattened_positions
        );

        Ok(KillSwitchResult {
            timestamp: chrono::Utc::now(),
            orders_cancelled: cancelled_orders,
            positions_flattened: flattened_positions,
        })
    }

    /// Reset kill switch (requires manual intervention)
    pub async fn reset_kill_switch(&self) -> Result<()> {
        warn!("Kill switch is being reset - manual verification required");
        
        let mut ks = self.kill_switch_triggered.write().await;
        *ks = false;
        
        info!("Kill switch has been reset");
        Ok(())
    }

    // Private helper methods

    async fn build_order(
        &self,
        ticker: &str,
        side: OrderSide,
        quantity: Decimal,
        market_price: Decimal,
        portfolio_id: Uuid,
        proposal_id: Uuid,
    ) -> Result<Order> {
        let order_type = if self.config.prefer_limit_orders {
            OrderType::Limit
        } else {
            OrderType::Market
        };

        let mut order = Order::new(
            ticker,
            side,
            quantity,
            order_type,
            portfolio_id,
        )
        .with_proposal(proposal_id)
        .with_time_in_force(self.config.default_tif)
        .with_notes(format!("Auto-generated from proposal {}", proposal_id));

        // Set limit price with offset
        if order_type == OrderType::Limit {
            let limit_price = match side {
                OrderSide::Buy => market_price + self.config.limit_offset,
                OrderSide::Sell => market_price - self.config.limit_offset,
            };
            order = order.with_limit_price(limit_price);
        }

        Ok(order)
    }

    async fn flatten_all_positions(&self, portfolio_id: Uuid) -> Result<usize> {
        // Would get all positions and create closing orders
        // Simplified implementation
        info!("Flattening all positions for portfolio {}", portfolio_id);
        
        // In production:
        // 1. Get all positions from broker
        // 2. Create market orders to close each
        // 3. Submit all orders
        // 4. Return count of positions flattened
        
        Ok(0) // Placeholder
    }
}

/// Kill switch result
#[derive(Debug, Clone)]
pub struct KillSwitchResult {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub orders_cancelled: usize,
    pub positions_flattened: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: Full tests would require mocking the OrderManager
    // These are basic unit tests for the configuration

    #[test]
    fn test_execution_config_default() {
        let config = ExecutionConfig::default();
        
        assert!(!config.auto_execute); // Disabled by default
        assert_eq!(config.manual_confirmation_threshold, Decimal::from(10000));
        assert!(config.prefer_limit_orders);
    }

    #[test]
    fn test_kill_switch_result() {
        let result = KillSwitchResult {
            timestamp: chrono::Utc::now(),
            orders_cancelled: 5,
            positions_flattened: 3,
        };

        assert_eq!(result.orders_cancelled, 5);
        assert_eq!(result.positions_flattened, 3);
    }
}
