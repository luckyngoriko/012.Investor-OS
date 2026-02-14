//! Limit Enforcer
//!
//! Enforces trading limits to prevent excessive risk-taking

use chrono::{DateTime, Duration, Utc};
use rust_decimal::Decimal;
use std::collections::HashMap;
use tracing::{debug, warn};

use super::Result;

/// Trading limits configuration
#[derive(Debug, Clone)]
pub struct TradingLimits {
    /// Max position size per symbol (in base currency)
    pub max_position_size: Decimal,
    /// Max total portfolio exposure (% of equity, 0-1)
    pub max_portfolio_exposure: Decimal,
    /// Max single trade size
    pub max_trade_size: Decimal,
    /// Max daily loss limit
    pub daily_loss_limit: Decimal,
    /// Max daily trades
    pub max_daily_trades: u32,
    /// Max orders per minute (rate limiting)
    pub max_orders_per_minute: u32,
    /// Max drawdown % before halting
    pub max_drawdown_pct: Decimal,
    /// Require human approval for trades above this size
    pub large_trade_threshold: Decimal,
    /// Max concentration in single symbol (% of portfolio)
    pub max_concentration_pct: Decimal,
}

impl Default for TradingLimits {
    fn default() -> Self {
        Self {
            max_position_size: Decimal::from(100000),      // $100k per symbol
            max_portfolio_exposure: Decimal::from(2),      // 200% (allows leverage)
            max_trade_size: Decimal::from(50000),          // $50k per trade
            daily_loss_limit: Decimal::from(5000),         // $5k daily loss
            max_daily_trades: 100,                          // 100 trades/day
            max_orders_per_minute: 10,                      // 10 orders/minute
            max_drawdown_pct: Decimal::from(10),           // 10% max drawdown
            large_trade_threshold: Decimal::from(25000),   // $25k+ needs approval
            max_concentration_pct: Decimal::from(20),      // 20% max in one symbol
        }
    }
}

/// Limit enforcer tracks current usage against limits
#[derive(Debug)]
pub struct LimitEnforcer {
    limits: TradingLimits,
    daily_stats: DailyStats,
    position_sizes: HashMap<String, Decimal>,
    order_history: Vec<DateTime<Utc>>,
    equity_high: Decimal,
    current_equity: Decimal,
}

/// Daily trading statistics
#[derive(Debug, Clone)]
struct DailyStats {
    date: DateTime<Utc>,
    trades_executed: u32,
    daily_pnl: Decimal,
}

/// Result of limit check
#[derive(Debug, Clone)]
pub struct LimitCheck {
    pub passed: bool,
    pub failed_limits: Vec<LimitType>,
    pub warnings: Vec<String>,
}

/// Types of limits that can be checked
#[derive(Debug, Clone, PartialEq)]
pub enum LimitType {
    PositionSize,
    TradeSize,
    DailyLoss,
    DailyTrades,
    OrderRate,
    Drawdown,
    Concentration,
}

impl LimitEnforcer {
    /// Create new enforcer with given limits
    pub fn new(limits: TradingLimits) -> Self {
        Self {
            limits,
            daily_stats: DailyStats::new(),
            position_sizes: HashMap::new(),
            order_history: Vec::new(),
            equity_high: Decimal::from(100000), // Default starting equity
            current_equity: Decimal::from(100000),
        }
    }

    /// Update limits
    pub fn update_limits(&mut self, limits: TradingLimits) {
        self.limits = limits;
    }

    /// Check if action is within limits
    pub fn check_action(&self, action: &super::Action) -> Result<LimitCheck> {
        let mut failed = Vec::new();
        let mut warnings = Vec::new();

        // Check trade size
        let notional = action.quantity * action.price.unwrap_or_default();
        if notional > self.limits.max_trade_size {
            failed.push(LimitType::TradeSize);
            warn!(
                "Trade size limit exceeded: {} > {}",
                notional, self.limits.max_trade_size
            );
        }

        // Check large trade threshold
        if notional > self.limits.large_trade_threshold {
            warnings.push(format!(
                "Large trade: {} exceeds threshold {}",
                notional, self.limits.large_trade_threshold
            ));
        }

        // Check position size
        let current_position = self.position_sizes.get(&action.symbol).copied()
            .unwrap_or(Decimal::ZERO);
        let new_position = match action.side {
            crate::broker::OrderSide::Buy => current_position + action.quantity,
            crate::broker::OrderSide::Sell => {
                if current_position >= action.quantity {
                    current_position - action.quantity
                } else {
                    Decimal::ZERO // Short position
                }
            }
        };
        
        let position_notional = new_position * action.price.unwrap_or_default();
        if position_notional > self.limits.max_position_size {
            failed.push(LimitType::PositionSize);
            warn!(
                "Position size limit exceeded for {}: {} > {}",
                action.symbol, position_notional, self.limits.max_position_size
            );
        }

        // Check concentration
        if !self.current_equity.is_zero() {
            let concentration = position_notional / self.current_equity * Decimal::from(100);
            if concentration > self.limits.max_concentration_pct {
                failed.push(LimitType::Concentration);
                warn!(
                    "Concentration limit exceeded: {}% > {}%",
                    concentration, self.limits.max_concentration_pct
                );
            }
        }

        // Check daily loss
        if self.daily_stats.daily_pnl < -self.limits.daily_loss_limit {
            failed.push(LimitType::DailyLoss);
            warn!(
                "Daily loss limit exceeded: {}",
                self.daily_stats.daily_pnl
            );
        }

        // Check daily trades
        if self.daily_stats.trades_executed >= self.limits.max_daily_trades {
            failed.push(LimitType::DailyTrades);
            warn!(
                "Daily trade limit reached: {}",
                self.daily_stats.trades_executed
            );
        }

        // Check order rate
        let recent_orders = self.order_history.iter()
            .filter(|t| Utc::now() - **t < Duration::minutes(1))
            .count() as u32;
        if recent_orders >= self.limits.max_orders_per_minute {
            failed.push(LimitType::OrderRate);
            warn!("Order rate limit exceeded: {} orders/min", recent_orders);
        }

        // Check drawdown
        let drawdown = self.calculate_drawdown();
        if drawdown > self.limits.max_drawdown_pct {
            failed.push(LimitType::Drawdown);
            warn!("Max drawdown exceeded: {}%", drawdown);
        }

        Ok(LimitCheck {
            passed: failed.is_empty(),
            failed_limits: failed,
            warnings,
        })
    }

    /// Record a trade execution
    pub fn record_trade(&mut self, symbol: &str, quantity: Decimal, pnl: Decimal) {
        // Reset daily stats if new day
        if !self.is_same_day(&self.daily_stats.date, &Utc::now()) {
            self.daily_stats = DailyStats::new();
        }

        self.daily_stats.trades_executed += 1;
        self.daily_stats.daily_pnl += pnl;

        // Update position size
        let current = self.position_sizes.entry(symbol.to_string())
            .or_insert(Decimal::ZERO);
        *current += quantity;

        debug!(
            "Recorded trade: {} {} (PnL: {})",
            symbol, quantity, pnl
        );
    }

    /// Record order submission (for rate limiting)
    pub fn record_order(&mut self) {
        self.order_history.push(Utc::now());
        
        // Clean old history (older than 5 minutes)
        let cutoff = Utc::now() - Duration::minutes(5);
        self.order_history.retain(|t| *t > cutoff);
    }

    /// Update equity and track high water mark
    pub fn update_equity(&mut self, equity: Decimal) {
        self.current_equity = equity;
        if equity > self.equity_high {
            self.equity_high = equity;
        }
    }

    /// Calculate current drawdown %
    fn calculate_drawdown(&self) -> Decimal {
        if self.equity_high.is_zero() {
            return Decimal::ZERO;
        }
        let drawdown = (self.equity_high - self.current_equity) / self.equity_high * Decimal::from(100);
        drawdown.max(Decimal::ZERO)
    }

    /// Check if two dates are the same day
    fn is_same_day(&self, d1: &DateTime<Utc>, d2: &DateTime<Utc>) -> bool {
        d1.date_naive() == d2.date_naive()
    }

    /// Get current limits
    pub fn limits(&self) -> &TradingLimits {
        &self.limits
    }

    /// Get daily stats
    pub fn daily_stats(&self) -> &DailyStats {
        &self.daily_stats
    }

    /// Get current drawdown
    pub fn current_drawdown(&self) -> Decimal {
        self.calculate_drawdown()
    }
}

impl Default for LimitEnforcer {
    fn default() -> Self {
        Self::new(TradingLimits::default())
    }
}

impl DailyStats {
    fn new() -> Self {
        Self {
            date: Utc::now(),
            trades_executed: 0,
            daily_pnl: Decimal::ZERO,
        }
    }

    pub fn trades_executed(&self) -> u32 {
        self.trades_executed
    }

    pub fn daily_pnl(&self) -> Decimal {
        self.daily_pnl
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trading_limits_default() {
        let limits = TradingLimits::default();
        assert_eq!(limits.max_trade_size, Decimal::from(50000));
        assert_eq!(limits.max_daily_trades, 100);
    }

    #[test]
    fn test_limit_enforcer_creation() {
        let enforcer = LimitEnforcer::default();
        assert_eq!(enforcer.current_drawdown(), Decimal::ZERO);
    }

    #[test]
    fn test_drawdown_calculation() {
        let mut enforcer = LimitEnforcer::default();
        
        enforcer.update_equity(Decimal::from(100000));
        enforcer.update_equity(Decimal::from(90000)); // 10% drawdown
        
        assert_eq!(enforcer.current_drawdown(), Decimal::from(10));
    }

    #[test]
    fn test_daily_stats_reset() {
        let mut enforcer = LimitEnforcer::default();
        
        enforcer.record_trade("AAPL", Decimal::from(100), Decimal::from(1000));
        assert_eq!(enforcer.daily_stats.trades_executed, 1);
        
        // Should accumulate same day
        enforcer.record_trade("GOOGL", Decimal::from(50), Decimal::from(-500));
        assert_eq!(enforcer.daily_stats.trades_executed, 2);
        assert_eq!(enforcer.daily_stats.daily_pnl, Decimal::from(500));
    }
}
