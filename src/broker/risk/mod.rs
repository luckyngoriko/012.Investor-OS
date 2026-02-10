//! Risk Management
//!
//! S6-D5: Risk Pre-checks - Validate orders against constraints

use rust_decimal::Decimal;
use tracing::{info, warn};

use crate::broker::{BrokerConfig, Order, OrderSide, Position, Result};

/// Risk checker for validating orders before submission
pub struct RiskChecker {
    config: BrokerConfig,
}

/// Risk check result
#[derive(Debug, Clone)]
pub struct RiskCheckResult {
    pub passed: bool,
    pub violations: Vec<RiskViolation>,
}

/// Risk violation details
#[derive(Debug, Clone)]
pub struct RiskViolation {
    pub rule: String,
    pub message: String,
    pub severity: RiskSeverity,
}

/// Risk severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiskSeverity {
    Warning,
    Error,
    Fatal,
}

impl RiskChecker {
    /// Create a new risk checker
    pub fn new(config: BrokerConfig) -> Self {
        Self { config }
    }

    /// Validate an order against risk rules
    pub async fn validate_order(
        &self,
        order: &Order,
        positions: &[Position],
        account_value: Decimal,
    ) -> Result<RiskCheckResult> {
        let mut violations = Vec::new();

        // Check 1: Maximum order size
        if let Some(order_value) = self.calculate_order_value(order).await? {
            if order_value > self.config.max_order_size {
                violations.push(RiskViolation {
                    rule: "MAX_ORDER_SIZE".to_string(),
                    message: format!(
                        "Order value ${} exceeds maximum ${}",
                        order_value, self.config.max_order_size
                    ),
                    severity: RiskSeverity::Error,
                });
            }
        }

        // Check 2: Maximum position size
        if let Some(position_value) = self.calculate_position_value_after_order(order, positions).await? {
            if position_value > self.config.max_position_size {
                violations.push(RiskViolation {
                    rule: "MAX_POSITION_SIZE".to_string(),
                    message: format!(
                        "Position value ${} would exceed maximum ${}",
                        position_value, self.config.max_position_size
                    ),
                    severity: RiskSeverity::Error,
                });
            }
        }

        // Check 3: Concentration limit (no more than 20% in single position)
        if let Some(position_value) = self.calculate_position_value_after_order(order, positions).await? {
            let concentration = position_value / account_value;
            if concentration > Decimal::from(20) / Decimal::from(100) {
                violations.push(RiskViolation {
                    rule: "CONCENTRATION_LIMIT".to_string(),
                    message: format!(
                        "Position would be {:.1}% of portfolio (max 20%)",
                        concentration * Decimal::from(100)
                    ),
                    severity: RiskSeverity::Warning,
                });
            }
        }

        // Check 4: Self-trade prevention
        if self.would_cause_self_trade(order, positions) {
            violations.push(RiskViolation {
                rule: "SELF_TRADE".to_string(),
                message: "Order would result in self-trade".to_string(),
                severity: RiskSeverity::Error,
            });
        }

        // Check 5: Daily loss limit
        // Would check realized P&L for the day

        // Check 6: Paper trading warning
        if self.config.paper_trading {
            violations.push(RiskViolation {
                rule: "PAPER_TRADING".to_string(),
                message: "Running in PAPER TRADING mode - no real money at risk".to_string(),
                severity: RiskSeverity::Warning,
            });
        }

        // Determine if check passed
        let fatal_or_error = violations.iter()
            .any(|v| matches!(v.severity, RiskSeverity::Fatal | RiskSeverity::Error));

        let result = RiskCheckResult {
            passed: !fatal_or_error,
            violations,
        };

        if result.passed {
            info!("Risk check passed for order {}", order.id);
        } else {
            warn!("Risk check failed for order {}: {:?}", order.id, result.violations);
        }

        Ok(result)
    }

    /// Quick check if order is allowed
    pub fn is_order_allowed(&self, order: &Order) -> bool {
        // Basic checks that don't require async
        
        // Check quantity is positive
        if order.quantity <= Decimal::ZERO {
            return false;
        }

        // Check limit price for limit orders
        if order.order_type == crate::broker::OrderType::Limit
            && (order.limit_price.is_none() || order.limit_price.unwrap() <= Decimal::ZERO) {
                return false;
            }

        true
    }

    // Private helper methods

    async fn calculate_order_value(&self, order: &Order) -> Result<Option<Decimal>> {
        // Would get current market price and calculate value
        // For now, use limit price if available
        if let Some(price) = order.limit_price {
            Ok(Some(price * order.quantity))
        } else {
            Ok(None) // Market order - value unknown until fill
        }
    }

    async fn calculate_position_value_after_order(
        &self,
        order: &Order,
        positions: &[Position],
    ) -> Result<Option<Decimal>> {
        // Find existing position
        let existing_position = positions.iter()
            .find(|p| p.ticker == order.ticker);

        // Calculate new position size
        let new_quantity = if let Some(pos) = existing_position {
            match order.side {
                OrderSide::Buy => pos.quantity + order.quantity,
                OrderSide::Sell => {
                    if order.quantity >= pos.quantity {
                        Decimal::ZERO
                    } else {
                        pos.quantity - order.quantity
                    }
                }
            }
        } else {
            match order.side {
                OrderSide::Buy => order.quantity,
                OrderSide::Sell => Decimal::ZERO, // Short not supported
            }
        };

        // Calculate value
        if let Some(price) = order.limit_price {
            Ok(Some(price * new_quantity))
        } else {
            Ok(None)
        }
    }

    fn would_cause_self_trade(&self, _order: &Order, _positions: &[Position]) -> bool {
        // Would check for opposing orders in the system
        // Simplified for now
        false
    }
}

/// Pre-trade risk configuration
#[derive(Debug, Clone)]
pub struct RiskConfig {
    pub max_order_value: Decimal,
    pub max_position_value: Decimal,
    pub max_concentration_pct: Decimal,
    pub daily_loss_limit: Decimal,
    pub require_price_limit: bool,
}

impl Default for RiskConfig {
    fn default() -> Self {
        Self {
            max_order_value: Decimal::from(50000),
            max_position_value: Decimal::from(100000),
            max_concentration_pct: Decimal::from(20) / Decimal::from(100),
            daily_loss_limit: Decimal::from(5000),
            require_price_limit: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::broker::{OrderType, TimeInForce};
    use uuid::Uuid;

    fn create_test_order() -> Order {
        Order::new(
            "AAPL",
            OrderSide::Buy,
            Decimal::from(100),
            OrderType::Limit,
            Uuid::new_v4(),
        )
        .with_limit_price(Decimal::from(150))
    }

    #[test]
    fn test_risk_checker_allows_valid_order() {
        let config = BrokerConfig::default();
        let checker = RiskChecker::new(config);
        
        let order = create_test_order();
        assert!(checker.is_order_allowed(&order));
    }

    #[test]
    fn test_risk_checker_rejects_zero_quantity() {
        let config = BrokerConfig::default();
        let checker = RiskChecker::new(config);
        
        let mut order = create_test_order();
        order.quantity = Decimal::ZERO;
        
        assert!(!checker.is_order_allowed(&order));
    }

    #[test]
    fn test_risk_checker_rejects_limit_order_without_price() {
        let config = BrokerConfig::default();
        let checker = RiskChecker::new(config);
        
        let order = Order::new(
            "AAPL",
            OrderSide::Buy,
            Decimal::from(100),
            OrderType::Limit,
            Uuid::new_v4(),
        );
        // No limit price set
        
        assert!(!checker.is_order_allowed(&order));
    }
}
