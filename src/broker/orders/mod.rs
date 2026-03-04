//! Order Management
//!
//! S6-D3: Order Management - Place, modify, cancel orders
//! S6-D7: Order Journal - Log all broker interactions

use chrono::Utc;
use sqlx::PgPool;
use std::sync::Arc;
use tracing::{error, info};
use uuid::Uuid;

use crate::broker::{Broker, BrokerError, Execution, Order, OrderStatus, Result};

/// Order manager for tracking and persisting orders
pub struct OrderManager {
    pool: PgPool,
    broker: Arc<dyn Broker>,
}

impl OrderManager {
    /// Create a new order manager
    pub fn new(pool: PgPool, broker: Arc<dyn Broker>) -> Self {
        Self { pool, broker }
    }

    /// Submit a new order to the broker and persist it
    pub async fn submit_order(&self, order: &mut Order) -> Result<()> {
        // Persist order to database first
        self.persist_order(order).await?;

        // Submit to broker
        match self.broker.place_order(order).await {
            Ok(()) => {
                info!(
                    "Order submitted: {} {} {} shares of {}",
                    order.id,
                    order.side.as_str(),
                    order.quantity,
                    order.ticker
                );

                // Update in database
                self.update_order_status(order).await?;

                // Log to order journal
                self.log_order_action(order, "SUBMIT", None).await?;

                Ok(())
            }
            Err(e) => {
                error!("Failed to submit order {}: {}", order.id, e);
                order.status = OrderStatus::ApiCancelled;
                self.update_order_status(order).await?;
                self.log_order_action(order, "SUBMIT_FAILED", Some(&e.to_string()))
                    .await?;
                Err(e)
            }
        }
    }

    /// Cancel an existing order
    pub async fn cancel_order(&self, order: &mut Order) -> Result<()> {
        if !order.is_active() {
            return Err(BrokerError::InvalidOrder("Order is not active".to_string()));
        }

        match self.broker.cancel_order(order).await {
            Ok(()) => {
                info!("Order cancelled: {}", order.id);
                self.update_order_status(order).await?;
                self.log_order_action(order, "CANCEL", None).await?;
                Ok(())
            }
            Err(e) => {
                error!("Failed to cancel order {}: {}", order.id, e);
                self.log_order_action(order, "CANCEL_FAILED", Some(&e.to_string()))
                    .await?;
                Err(e)
            }
        }
    }

    /// Update order status from broker
    pub async fn refresh_order_status(&self, order: &mut Order) -> Result<OrderStatus> {
        let status = self.broker.get_order_status(order).await?;

        if status != order.status {
            info!(
                "Order {} status changed: {:?} -> {:?}",
                order.id, order.status, status
            );
            order.status = status;
            order.updated_at = Utc::now();
            self.update_order_status(order).await?;

            // Log status change
            self.log_order_action(order, "STATUS_UPDATE", None).await?;
        }

        Ok(status)
    }

    /// Get order by ID
    pub async fn get_order(&self, _order_id: Uuid) -> Result<Option<Order>> {
        // Implementation would query database
        Ok(None)
    }

    /// Get orders for a portfolio
    pub async fn get_portfolio_orders(
        &self,
        _portfolio_id: Uuid,
        _status_filter: Option<OrderStatus>,
    ) -> Result<Vec<Order>> {
        // Implementation would query database
        Ok(vec![])
    }

    /// Get active orders for a portfolio
    pub async fn get_active_orders(&self, portfolio_id: Uuid) -> Result<Vec<Order>> {
        self.get_portfolio_orders(portfolio_id, None)
            .await
            .map(|orders| orders.into_iter().filter(|o| o.is_active()).collect())
    }

    /// Record an execution
    pub async fn record_execution(&self, execution: &Execution) -> Result<()> {
        info!(
            "Execution recorded: {} {} {} shares @ ${}",
            execution.id,
            execution.side.as_str(),
            execution.quantity,
            execution.price
        );
        Ok(())
    }

    /// Get executions for an order
    pub async fn get_order_executions(&self, _order_id: Uuid) -> Result<Vec<Execution>> {
        // Implementation would query database
        Ok(vec![])
    }

    // Private helper methods

    async fn persist_order(&self, _order: &Order) -> Result<()> {
        // Implementation would insert into database
        Ok(())
    }

    async fn update_order_status(&self, _order: &Order) -> Result<()> {
        // Implementation would update database
        Ok(())
    }

    /// Log order action to journal (S6-D7)
    async fn log_order_action(
        &self,
        order: &Order,
        action: &str,
        error: Option<&str>,
    ) -> Result<()> {
        info!(
            "Order Journal: {} {} {} {} shares - Status: {:?} Error: {:?}",
            order.id,
            action,
            order.side.as_str(),
            order.quantity,
            order.status,
            error
        );
        Ok(())
    }
}
