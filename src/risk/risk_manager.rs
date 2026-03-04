//! Risk Manager - orchestrates position sizing, stop-loss, and portfolio risk

use chrono::Utc;
use rust_decimal::Decimal;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

use super::{
    portfolio_risk::{PortfolioRisk, RiskMetrics, VaRConfig},
    position_sizing::{PositionSizer, SizingConfig},
    stop_loss::{StopLossConfig, StopLossManager, StopLossType},
    Result,
};

/// Risk limits configuration
#[derive(Debug, Clone)]
pub struct RiskLimits {
    /// Maximum portfolio drawdown before halting
    pub max_drawdown: Decimal,
    /// Maximum daily loss
    pub max_daily_loss: Decimal,
    /// Maximum position size as % of portfolio
    pub max_position_weight: Decimal,
    /// Maximum VaR (95%) as % of portfolio
    pub max_var_95: Decimal,
    /// Maximum open positions
    pub max_open_positions: usize,
    /// Maximum leverage
    pub max_leverage: Decimal,
    /// Minimum risk/reward ratio
    pub min_risk_reward_ratio: Decimal,
}

impl Default for RiskLimits {
    fn default() -> Self {
        Self {
            max_drawdown: Decimal::try_from(0.20).unwrap(), // 20%
            max_daily_loss: Decimal::try_from(0.05).unwrap(), // 5%
            max_position_weight: Decimal::try_from(0.25).unwrap(), // 25%
            max_var_95: Decimal::try_from(0.03).unwrap(),   // 3%
            max_open_positions: 20,
            max_leverage: Decimal::from(10),
            min_risk_reward_ratio: Decimal::from(2), // 2:1
        }
    }
}

/// Risk assessment result
#[derive(Debug, Clone)]
pub struct RiskAssessment {
    pub approved: bool,
    pub reason: Option<String>,
    pub suggested_size: Option<Decimal>,
    pub risk_score: Decimal, // 0-1, higher = riskier
}

/// Risk Manager coordinates all risk functions
#[derive(Debug, Clone)]
pub struct RiskManager {
    limits: RiskLimits,
    position_sizer: PositionSizer,
    stop_loss_manager: Arc<RwLock<StopLossManager>>,
    portfolio_risk: Arc<RwLock<PortfolioRisk>>,

    // Daily tracking
    daily_pnl: Arc<RwLock<Decimal>>,
    daily_starting_equity: Arc<RwLock<Decimal>>,
    open_positions: Arc<RwLock<usize>>,

    // Circuit breaker
    circuit_breaker_triggered: Arc<RwLock<bool>>,
}

impl RiskManager {
    /// Create a new risk manager
    pub fn new(limits: RiskLimits, sizing_config: SizingConfig, var_config: VaRConfig) -> Self {
        Self {
            limits: limits.clone(),
            position_sizer: PositionSizer::new(sizing_config),
            stop_loss_manager: Arc::new(RwLock::new(StopLossManager::new())),
            portfolio_risk: Arc::new(RwLock::new(PortfolioRisk::new(var_config))),
            daily_pnl: Arc::new(RwLock::new(Decimal::ZERO)),
            daily_starting_equity: Arc::new(RwLock::new(Decimal::ZERO)),
            open_positions: Arc::new(RwLock::new(0)),
            circuit_breaker_triggered: Arc::new(RwLock::new(false)),
        }
    }

    /// Initialize with starting equity
    pub async fn initialize(&self, starting_equity: Decimal) {
        *self.daily_starting_equity.write().await = starting_equity;
        self.portfolio_risk
            .write()
            .await
            .update_equity(starting_equity);
        info!("Risk manager initialized with equity: {}", starting_equity);
    }

    /// Assess risk for a proposed trade
    pub async fn assess_trade(
        &self,
        _symbol: &str,
        entry_price: Decimal,
        stop_loss: Decimal,
        take_profit: Option<Decimal>,
        available_capital: Decimal,
        current_portfolio_value: Decimal,
    ) -> Result<RiskAssessment> {
        // Check circuit breaker
        if *self.circuit_breaker_triggered.read().await {
            return Ok(RiskAssessment {
                approved: false,
                reason: Some("Circuit breaker triggered - trading halted".to_string()),
                suggested_size: None,
                risk_score: Decimal::ONE,
            });
        }

        // Check daily loss limit
        let daily_pnl = *self.daily_pnl.read().await;
        let daily_starting = *self.daily_starting_equity.read().await;
        let daily_loss_pct = if !daily_starting.is_zero() {
            (-daily_pnl) / daily_starting
        } else {
            Decimal::ZERO
        };

        if daily_loss_pct >= self.limits.max_daily_loss {
            return Ok(RiskAssessment {
                approved: false,
                reason: Some(format!(
                    "Daily loss limit reached: {}%",
                    daily_loss_pct * Decimal::from(100)
                )),
                suggested_size: None,
                risk_score: Decimal::ONE,
            });
        }

        // Check max positions
        let open_positions = *self.open_positions.read().await;
        if open_positions >= self.limits.max_open_positions {
            return Ok(RiskAssessment {
                approved: false,
                reason: Some(format!(
                    "Maximum open positions ({}) reached",
                    self.limits.max_open_positions
                )),
                suggested_size: None,
                risk_score: Decimal::ONE,
            });
        }

        // Check risk/reward ratio
        if let Some(tp) = take_profit {
            let risk = (entry_price - stop_loss).abs();
            let reward = (tp - entry_price).abs();

            if !risk.is_zero() {
                let ratio = reward / risk;
                if ratio < self.limits.min_risk_reward_ratio {
                    return Ok(RiskAssessment {
                        approved: false,
                        reason: Some(format!(
                            "Risk/reward ratio {} below minimum {}",
                            ratio, self.limits.min_risk_reward_ratio
                        )),
                        suggested_size: None,
                        risk_score: Decimal::ONE,
                    });
                }
            }
        }

        // Calculate position size
        let size = self.position_sizer.calculate_size(
            available_capital,
            entry_price,
            Some(stop_loss),
            None, // No volatility data
            None, // No win rate
            None, // No win/loss ratio
        )?;

        // Check position weight
        let position_value = size * entry_price;
        let weight = if !current_portfolio_value.is_zero() {
            position_value / current_portfolio_value
        } else {
            Decimal::ONE
        };

        if weight > self.limits.max_position_weight {
            let max_size = self.limits.max_position_weight * current_portfolio_value / entry_price;
            return Ok(RiskAssessment {
                approved: false,
                reason: Some(format!(
                    "Position weight {}% exceeds maximum {}%",
                    weight * Decimal::from(100),
                    self.limits.max_position_weight * Decimal::from(100)
                )),
                suggested_size: Some(max_size),
                risk_score: Decimal::try_from(0.8).unwrap(),
            });
        }

        // Calculate risk score (simplified)
        let risk_score = weight / self.limits.max_position_weight;

        Ok(RiskAssessment {
            approved: true,
            reason: None,
            suggested_size: Some(size),
            risk_score,
        })
    }

    /// Create a stop-loss for a position
    pub async fn create_stop_loss(
        &self,
        position_id: String,
        symbol: String,
        entry_price: Decimal,
        quantity: Decimal,
        is_long: bool,
        stop_loss_price: Decimal,
    ) -> Result<String> {
        let config = StopLossConfig {
            stop_type: StopLossType::Fixed,
            stop_price: Some(stop_loss_price),
            ..Default::default()
        };

        self.stop_loss_manager.write().await.create_stop_loss(
            position_id,
            symbol,
            entry_price,
            quantity,
            is_long,
            config,
            None,
        )
    }

    /// Create a trailing stop
    pub async fn create_trailing_stop(
        &self,
        position_id: String,
        symbol: String,
        entry_price: Decimal,
        quantity: Decimal,
        is_long: bool,
        trailing_pct: Decimal,
    ) -> Result<String> {
        let config = StopLossConfig {
            stop_type: StopLossType::Trailing,
            trailing_distance: trailing_pct,
            ..Default::default()
        };

        self.stop_loss_manager.write().await.create_stop_loss(
            position_id,
            symbol,
            entry_price,
            quantity,
            is_long,
            config,
            None,
        )
    }

    /// Check and update stop-losses
    pub async fn check_stop_losses(&self, prices: &[(String, Decimal)]) -> Vec<String> {
        let mut triggered = Vec::new();
        let slm = self.stop_loss_manager.read().await;

        for (symbol, price) in prices {
            for (order_id, order) in slm.get_all_orders() {
                if order.symbol == *symbol && slm.check_trigger(order_id, *price).is_some() {
                    triggered.push(order_id.clone());
                }
            }
        }

        triggered
    }

    /// Update equity and check risk limits
    pub async fn update_equity(&self, equity: Decimal) -> Result<RiskMetrics> {
        let mut pr = self.portfolio_risk.write().await;
        pr.update_equity(equity);

        // Check drawdown
        let (max_dd, current_dd) = pr.calculate_max_drawdown();
        drop(pr); // Release lock

        if current_dd > self.limits.max_drawdown {
            warn!(
                "Max drawdown {}% exceeded limit {}%",
                current_dd * Decimal::from(100),
                self.limits.max_drawdown * Decimal::from(100)
            );

            let mut cb = self.circuit_breaker_triggered.write().await;
            *cb = true;
        }

        // Calculate metrics
        // Note: In real implementation, we'd maintain a returns history
        let metrics = RiskMetrics {
            max_drawdown: max_dd,
            current_drawdown: current_dd,
            calculated_at: Utc::now(),
            ..Default::default()
        };

        Ok(metrics)
    }

    /// Update P&L and check daily limits
    pub async fn update_pnl(&self, pnl: Decimal) -> bool {
        let mut daily = self.daily_pnl.write().await;
        *daily += pnl;
        let daily_starting = *self.daily_starting_equity.read().await;

        let loss_pct = if !daily_starting.is_zero() {
            (-*daily) / daily_starting
        } else {
            Decimal::ZERO
        };

        if loss_pct >= self.limits.max_daily_loss {
            warn!(
                "Daily loss limit {}% reached with P&L {}",
                self.limits.max_daily_loss * Decimal::from(100),
                *daily
            );

            let mut cb = self.circuit_breaker_triggered.write().await;
            *cb = true;
            return false;
        }

        true
    }

    /// Record new position
    pub async fn add_position(&self) {
        let mut count = self.open_positions.write().await;
        *count += 1;
    }

    /// Remove closed position
    pub async fn remove_position(&self, position_id: &str) {
        self.stop_loss_manager
            .write()
            .await
            .cancel_for_position(position_id);
        let mut count = self.open_positions.write().await;
        if *count > 0 {
            *count -= 1;
        }
    }

    /// Reset circuit breaker (manual override)
    pub async fn reset_circuit_breaker(&self) {
        let mut cb = self.circuit_breaker_triggered.write().await;
        *cb = false;
        info!("Circuit breaker reset");
    }

    /// Check if trading is allowed
    pub async fn is_trading_allowed(&self) -> bool {
        !*self.circuit_breaker_triggered.read().await
    }

    /// Get current risk metrics
    pub async fn get_metrics(&self) -> RiskMetrics {
        let pr = self.portfolio_risk.read().await;
        let (max_dd, current_dd) = pr.calculate_max_drawdown();
        RiskMetrics {
            max_drawdown: max_dd,
            current_drawdown: current_dd,
            calculated_at: Utc::now(),
            ..Default::default()
        }
    }

    /// Get stop-loss manager (acquires lock)
    pub async fn stop_loss_manager(&self) -> tokio::sync::RwLockReadGuard<'_, StopLossManager> {
        self.stop_loss_manager.read().await
    }

    /// Get mutable stop-loss manager (acquires lock)
    pub async fn stop_loss_manager_mut(
        &self,
    ) -> tokio::sync::RwLockWriteGuard<'_, StopLossManager> {
        self.stop_loss_manager.write().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_risk_manager_creation() {
        let manager = RiskManager::new(
            RiskLimits::default(),
            SizingConfig::default(),
            VaRConfig::default(),
        );
        assert_eq!(
            manager.limits.max_drawdown,
            Decimal::try_from(0.20).unwrap()
        );
    }

    #[tokio::test]
    async fn test_trade_assessment_approved() {
        // Use appropriate risk % and min position size
        let sizing_config = SizingConfig {
            risk_percent: Decimal::try_from(0.01).unwrap(), // 1% risk per trade
            min_position_size: Decimal::try_from(0.001).unwrap(), // Small min size
            max_position_size: Decimal::from(1000000),
            ..Default::default()
        };
        let limits = RiskLimits {
            max_position_weight: Decimal::try_from(0.50).unwrap(), // 50% max
            ..Default::default()
        };
        let manager = RiskManager::new(limits, sizing_config, VaRConfig::default());

        let assessment = manager
            .assess_trade(
                "BTC",
                Decimal::from(50000),
                Decimal::from(49000),       // 2% stop
                Some(Decimal::from(52000)), // 4% profit
                Decimal::from(100000),
                Decimal::from(100000),
            )
            .await
            .unwrap();

        assert!(
            assessment.approved,
            "Trade should be approved: {:?}",
            assessment.reason
        );
        assert!(assessment.suggested_size.is_some());
    }

    #[tokio::test]
    async fn test_trade_assessment_poor_risk_reward() {
        let limits = RiskLimits {
            min_risk_reward_ratio: Decimal::from(3), // Require 3:1
            ..Default::default()
        };
        let manager = RiskManager::new(limits, SizingConfig::default(), VaRConfig::default());

        let assessment = manager
            .assess_trade(
                "BTC",
                Decimal::from(50000),
                Decimal::from(49000),       // 2% risk
                Some(Decimal::from(50500)), // 1% reward = 0.5:1 ratio
                Decimal::from(100000),
                Decimal::from(100000),
            )
            .await
            .unwrap();

        assert!(!assessment.approved);
        assert!(assessment.reason.unwrap().contains("Risk/reward"));
    }

    #[tokio::test]
    async fn test_position_weight_limit() {
        let limits = RiskLimits {
            max_position_weight: Decimal::try_from(0.10).unwrap(), // 10%
            ..Default::default()
        };
        let sizing_config = SizingConfig {
            risk_percent: Decimal::try_from(0.50).unwrap(), // 50% risk - would create large position
            max_position_size: Decimal::from(1000000),
            ..Default::default()
        };
        let manager = RiskManager::new(limits, sizing_config, VaRConfig::default());

        let assessment = manager
            .assess_trade(
                "BTC",
                Decimal::from(50000),
                Decimal::from(49500), // 1% stop
                Some(Decimal::from(55000)),
                Decimal::from(100000),
                Decimal::from(100000),
            )
            .await
            .unwrap();

        // Should be rejected due to position weight
        assert!(!assessment.approved);
        assert!(assessment.reason.unwrap().contains("Position weight"));
    }

    #[tokio::test]
    async fn test_circuit_breaker_on_drawdown() {
        let manager = RiskManager::new(
            RiskLimits {
                max_drawdown: Decimal::try_from(0.10).unwrap(), // 10% max drawdown
                ..Default::default()
            },
            SizingConfig::default(),
            VaRConfig::default(),
        );

        manager.initialize(Decimal::from(100000)).await;

        // Simulate 15% drawdown
        let metrics = manager.update_equity(Decimal::from(85000)).await.unwrap();
        eprintln!(
            "Drawdown: {}%",
            metrics.current_drawdown * Decimal::from(100)
        );

        // Trading should be halted
        assert!(
            !manager.is_trading_allowed().await,
            "Trading should be halted after {}% drawdown",
            metrics.current_drawdown * Decimal::from(100)
        );

        // New trade should be rejected
        let assessment = manager
            .assess_trade(
                "BTC",
                Decimal::from(50000),
                Decimal::from(49000),
                Some(Decimal::from(52000)),
                Decimal::from(100000),
                Decimal::from(85000),
            )
            .await
            .unwrap();

        assert!(!assessment.approved);
        assert!(assessment.reason.unwrap().contains("Circuit breaker"));
    }

    #[tokio::test]
    async fn test_daily_loss_limit() {
        let mut manager = RiskManager::new(
            RiskLimits {
                max_daily_loss: Decimal::try_from(0.05).unwrap(), // 5%
                ..Default::default()
            },
            SizingConfig::default(),
            VaRConfig::default(),
        );

        manager.initialize(Decimal::from(100000)).await;

        // Update with -6% loss
        let allowed = manager.update_pnl(Decimal::from(-6000)).await;
        assert!(!allowed);
        assert!(!manager.is_trading_allowed().await);
    }

    #[tokio::test]
    async fn test_max_positions_limit() {
        let manager = RiskManager::new(
            RiskLimits {
                max_open_positions: 2,
                ..Default::default()
            },
            SizingConfig::default(),
            VaRConfig::default(),
        );

        // Add 2 positions
        manager.add_position().await;
        manager.add_position().await;

        // Third trade should be rejected
        let assessment = manager
            .assess_trade(
                "BTC",
                Decimal::from(50000),
                Decimal::from(49000),
                Some(Decimal::from(52000)),
                Decimal::from(100000),
                Decimal::from(100000),
            )
            .await
            .unwrap();

        assert!(!assessment.approved);
        assert!(assessment
            .reason
            .unwrap()
            .contains("Maximum open positions"));
    }

    #[tokio::test]
    async fn test_reset_circuit_breaker() {
        let manager = RiskManager::new(
            RiskLimits {
                max_daily_loss: Decimal::try_from(0.01).unwrap(),
                ..Default::default()
            },
            SizingConfig::default(),
            VaRConfig::default(),
        );

        manager.initialize(Decimal::from(100000)).await;
        manager.update_pnl(Decimal::from(-5000)).await;

        assert!(!manager.is_trading_allowed().await);

        manager.reset_circuit_breaker().await;
        assert!(manager.is_trading_allowed().await);
    }
}
