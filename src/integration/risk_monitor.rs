//! Unified risk monitoring across Treasury and Margin modules

use rust_decimal::Decimal;
use tracing::{info, warn};
use std::collections::HashMap;

use crate::treasury::Treasury;
use crate::margin::{MarginManager, AccountStatus};
use super::error::{IntegrationError, Result};

/// System-wide risk thresholds
#[derive(Debug, Clone)]
pub struct RiskThresholds {
    /// Maximum allowed margin utilization (% of treasury)
    pub max_margin_utilization: Decimal,
    /// Maximum portfolio leverage (aggregate)
    pub max_portfolio_leverage: Decimal,
    /// Margin call threshold for intervention
    pub margin_call_threshold: Decimal,
    /// Maximum single position concentration (%)
    pub max_concentration_percent: Decimal,
    /// Daily loss limit (% of equity)
    pub daily_loss_limit_percent: Decimal,
}

impl Default for RiskThresholds {
    fn default() -> Self {
        Self {
            max_margin_utilization: Decimal::try_from(0.80).unwrap(), // 80%
            max_portfolio_leverage: Decimal::from(15), // 15x max
            margin_call_threshold: Decimal::try_from(1.10).unwrap(), // 110% margin ratio
            max_concentration_percent: Decimal::try_from(0.30).unwrap(), // 30%
            daily_loss_limit_percent: Decimal::try_from(0.05).unwrap(), // 5%
        }
    }
}

/// Risk status for the entire system
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SystemRiskStatus {
    Healthy,
    Warning(String),  // Warning with reason
    Critical(String), // Critical with reason
}

/// Comprehensive risk assessment
#[derive(Debug, Clone)]
pub struct RiskAssessment {
    pub status: SystemRiskStatus,
    pub treasury_equity: Decimal,
    pub margin_allocated: Decimal,
    pub margin_utilization: Decimal,
    pub total_exposure: Decimal,
    pub portfolio_leverage: Decimal,
    pub accounts_at_risk: usize,
    pub liquidation_count: usize,
    pub largest_position_symbol: Option<String>,
    pub largest_position_concentration: Decimal,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Unified risk monitor
#[derive(Debug)]
pub struct RiskMonitor {
    thresholds: RiskThresholds,
    daily_pnl_tracker: HashMap<uuid::Uuid, Decimal>, // Track daily P&L per account
    liquidation_history: Vec<(chrono::DateTime<chrono::Utc>, uuid::Uuid, Decimal)>,
}

impl RiskMonitor {
    /// Create new risk monitor
    pub fn new(thresholds: RiskThresholds) -> Self {
        Self {
            thresholds,
            daily_pnl_tracker: HashMap::new(),
            liquidation_history: Vec::new(),
        }
    }
    
    /// Perform comprehensive risk assessment
    pub fn assess_system_risk(
        &self,
        treasury: &Treasury,
        margin_manager: &MarginManager,
    ) -> RiskAssessment {
        let treasury_equity = treasury.total_equity();
        
        // Aggregate margin data
        let accounts = margin_manager.get_accounts();
        let margin_allocated = accounts
            .values()
            .map(|acc| acc.equity)
            .fold(Decimal::ZERO, |a, b| a + b);
        
        let total_exposure = margin_manager.total_exposure();
        let portfolio_leverage = if treasury_equity.is_zero() {
            Decimal::ZERO
        } else {
            total_exposure / treasury_equity
        };
        
        let margin_utilization = if treasury_equity.is_zero() {
            Decimal::ZERO
        } else {
            margin_allocated / treasury_equity
        };
        
        // Count accounts at risk
        let accounts_at_risk = accounts
            .values()
            .filter(|acc| acc.is_margin_call() || acc.status == AccountStatus::Liquidating)
            .count();
        
        // Find largest position
        let mut largest_concentration = Decimal::ZERO;
        let mut largest_symbol = None;
        
        for account in accounts.values() {
            for position in account.positions.values() {
                let concentration = if total_exposure.is_zero() {
                    Decimal::ZERO
                } else {
                    position.notional_value() / total_exposure
                };
                
                if concentration > largest_concentration {
                    largest_concentration = concentration;
                    largest_symbol = Some(position.symbol.clone());
                }
            }
        }
        
        // Determine overall status
        let status = self.determine_status(
            margin_utilization,
            portfolio_leverage,
            accounts_at_risk,
            largest_concentration,
        );
        
        RiskAssessment {
            status,
            treasury_equity,
            margin_allocated,
            margin_utilization,
            total_exposure,
            portfolio_leverage,
            accounts_at_risk,
            liquidation_count: self.liquidation_history.len(),
            largest_position_symbol: largest_symbol,
            largest_position_concentration: largest_concentration,
            timestamp: chrono::Utc::now(),
        }
    }
    
    /// Determine system risk status
    fn determine_status(
        &self,
        margin_utilization: Decimal,
        portfolio_leverage: Decimal,
        accounts_at_risk: usize,
        concentration: Decimal,
    ) -> SystemRiskStatus {
        // Critical conditions
        if margin_utilization > self.thresholds.max_margin_utilization {
            return SystemRiskStatus::Critical(
                format!("Margin utilization {}% exceeds {}% limit",
                    margin_utilization * Decimal::from(100),
                    self.thresholds.max_margin_utilization * Decimal::from(100)
                )
            );
        }
        
        if portfolio_leverage > self.thresholds.max_portfolio_leverage {
            return SystemRiskStatus::Critical(
                format!("Portfolio leverage {}x exceeds {}x limit",
                    portfolio_leverage,
                    self.thresholds.max_portfolio_leverage
                )
            );
        }
        
        if accounts_at_risk > 0 {
            return SystemRiskStatus::Critical(
                format!("{} accounts at risk of liquidation", accounts_at_risk)
            );
        }
        
        // Warning conditions
        if concentration > self.thresholds.max_concentration_percent {
            return SystemRiskStatus::Warning(
                format!("Position concentration {}% exceeds {}%",
                    concentration * Decimal::from(100),
                    self.thresholds.max_concentration_percent * Decimal::from(100)
                )
            );
        }
        
        if margin_utilization > self.thresholds.max_margin_utilization * Decimal::try_from(0.9).unwrap() {
            return SystemRiskStatus::Warning(
                "Margin utilization approaching limit".to_string()
            );
        }
        
        SystemRiskStatus::Healthy
    }
    
    /// Check if new position would breach risk limits
    pub fn check_position_risk(
        &self,
        margin_manager: &MarginManager,
        account_id: uuid::Uuid,
        _symbol: &str,
        notional: Decimal,
    ) -> Result<()> {
        let account = margin_manager.get_account(account_id)
            .ok_or_else(|| IntegrationError::MarginError("Account not found".to_string()))?;
        
        // Check concentration (only if there are existing positions)
        let current_exposure = margin_manager.total_exposure();
        if !current_exposure.is_zero() {
            let new_total = current_exposure + notional;
            let new_concentration = notional / new_total;
            
            if new_concentration > self.thresholds.max_concentration_percent {
                return Err(IntegrationError::RiskThresholdBreached(
                    format!("Position concentration {}% would exceed {}% limit",
                        new_concentration * Decimal::from(100),
                        self.thresholds.max_concentration_percent * Decimal::from(100)
                    )
                ));
            }
        }
        
        // Check account leverage
        let new_exposure = account.total_exposure() + notional;
        let new_leverage = new_exposure / account.equity;
        
        if new_leverage > self.thresholds.max_portfolio_leverage {
            return Err(IntegrationError::RiskThresholdBreached(
                format!("Account leverage {}x would exceed {}x limit",
                    new_leverage,
                    self.thresholds.max_portfolio_leverage
                )
            ));
        }
        
        Ok(())
    }
    
    /// Record liquidation event
    pub fn record_liquidation(&mut self, account_id: uuid::Uuid, amount: Decimal) {
        self.liquidation_history.push((chrono::Utc::now(), account_id, amount));
        warn!("Recorded liquidation for account {}: ${}", account_id, amount);
    }
    
    /// Update daily P&L tracking
    pub fn update_daily_pnl(&mut self, account_id: uuid::Uuid, pnl: Decimal) {
        *self.daily_pnl_tracker.entry(account_id).or_insert(Decimal::ZERO) += pnl;
    }
    
    /// Check daily loss limits
    pub fn check_daily_loss_limit(&self, account_id: uuid::Uuid, account_equity: Decimal) -> Result<()> {
        if let Some(daily_pnl) = self.daily_pnl_tracker.get(&account_id) {
            if *daily_pnl < Decimal::ZERO {
                let loss_percent = daily_pnl.abs() / account_equity;
                if loss_percent > self.thresholds.daily_loss_limit_percent {
                    return Err(IntegrationError::RiskThresholdBreached(
                        format!("Daily loss {}% exceeds {}% limit",
                            loss_percent * Decimal::from(100),
                            self.thresholds.daily_loss_limit_percent * Decimal::from(100)
                        )
                    ));
                }
            }
        }
        Ok(())
    }
    
    /// Get liquidation count (last 24h)
    pub fn recent_liquidation_count(&self) -> usize {
        let day_ago = chrono::Utc::now() - chrono::Duration::hours(24);
        self.liquidation_history.iter()
            .filter(|(ts, _, _)| *ts > day_ago)
            .count()
    }
    
    /// Reset daily trackers (call at midnight UTC)
    pub fn reset_daily_trackers(&mut self) {
        self.daily_pnl_tracker.clear();
        info!("Daily risk trackers reset");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::margin::PositionSide;
    
    #[tokio::test]
    async fn test_risk_assessment_healthy() {
        let monitor = RiskMonitor::new(RiskThresholds::default());
        let treasury = Treasury::new().await.unwrap();
        let margin_manager = MarginManager::new();
        
        let assessment = monitor.assess_system_risk(&treasury, &margin_manager);
        
        assert!(matches!(assessment.status, SystemRiskStatus::Healthy));
        assert_eq!(assessment.accounts_at_risk, 0);
    }
    
    #[test]
    fn test_concentration_limit_check() {
        let monitor = RiskMonitor::new(RiskThresholds {
            max_concentration_percent: Decimal::try_from(0.30).unwrap(),
            ..Default::default()
        });
        
        let mut margin_manager = MarginManager::new();
        let id = margin_manager.create_account("test".to_string(), Decimal::from(100000));
        
        // Open positions creating 50% concentration
        margin_manager.open_position(
            id, "BTC".to_string(), PositionSide::Long,
            Decimal::from(1), Decimal::from(50000), Decimal::from(1)
        ).unwrap();
        
        // Try to add another large BTC position that would exceed concentration
        let result = monitor.check_position_risk(
            &margin_manager, id, "BTC", Decimal::from(100000)
        );
        
        // Should fail due to concentration
        assert!(result.is_err());
    }
    
    #[test]
    fn test_daily_loss_tracking() {
        let mut monitor = RiskMonitor::new(RiskThresholds {
            daily_loss_limit_percent: Decimal::try_from(0.05).unwrap(), // 5%
            ..Default::default()
        });
        
        let account_id = uuid::Uuid::new_v4();
        let equity = Decimal::from(100000);
        
        // Record 3% loss - should be OK
        monitor.update_daily_pnl(account_id, Decimal::from(-3000));
        assert!(monitor.check_daily_loss_limit(account_id, equity).is_ok());
        
        // Add 3% more loss (total 6%) - should breach
        monitor.update_daily_pnl(account_id, Decimal::from(-3000));
        assert!(monitor.check_daily_loss_limit(account_id, equity).is_err());
    }
    
    #[test]
    fn test_liquidation_tracking() {
        let mut monitor = RiskMonitor::new(RiskThresholds::default());
        
        let id = uuid::Uuid::new_v4();
        monitor.record_liquidation(id, Decimal::from(50000));
        monitor.record_liquidation(id, Decimal::from(30000));
        
        assert_eq!(monitor.liquidation_history.len(), 2);
        assert_eq!(monitor.recent_liquidation_count(), 2);
    }
}
