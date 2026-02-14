//! Kill Switch System
//!
//! Emergency stop mechanism for trading system.
//! Supports both manual and automatic activation.

use super::circuit_breaker::MarketState;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info};

/// Kill switch state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KillSwitchState {
    /// Ready to trade
    Armed,
    /// Trading halted
    Triggered,
    /// In recovery mode
    Recovery,
}

/// Kill switch trigger reason
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KillSwitchTrigger {
    pub reason: String,
    pub triggered_by: String,
    pub triggered_at: DateTime<Utc>,
    pub reset_at: Option<DateTime<Utc>>,
    pub reset_by: Option<String>,
}

/// Kill switch configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KillSwitchConfig {
    pub auto_trigger_on_crash: bool,
    pub auto_trigger_on_extreme_loss: Option<Decimal>,
    pub require_manual_reset: bool,
}

impl Default for KillSwitchConfig {
    fn default() -> Self {
        Self {
            auto_trigger_on_crash: true,
            auto_trigger_on_extreme_loss: Some(Decimal::try_from(0.15).unwrap()), // 15% loss
            require_manual_reset: true,
        }
    }
}

/// Kill switch implementation
#[derive(Debug)]
pub struct KillSwitch {
    config: KillSwitchConfig,
    state: Arc<RwLock<KillSwitchState>>,
    trigger_history: Arc<RwLock<Vec<KillSwitchTrigger>>>,
    current_trigger: Arc<RwLock<Option<KillSwitchTrigger>>>,
}

impl KillSwitch {
    pub fn new(config: KillSwitchConfig) -> Self {
        Self {
            config,
            state: Arc::new(RwLock::new(KillSwitchState::Armed)),
            trigger_history: Arc::new(RwLock::new(Vec::new())),
            current_trigger: Arc::new(RwLock::new(None)),
        }
    }
    
    pub fn default() -> Self {
        Self::new(KillSwitchConfig::default())
    }
    
    /// Check if kill switch allows trading
    pub async fn check(&self) -> Result<(), KillSwitchError> {
        let state = *self.state.read().await;
        
        match state {
            KillSwitchState::Armed => Ok(()),
            KillSwitchState::Recovery => Err(KillSwitchError::SystemInRecovery),
            KillSwitchState::Triggered => {
                let trigger = self.current_trigger.read().await
                    .as_ref()
                    .map(|t| t.reason.clone())
                    .unwrap_or_else(|| "Unknown reason".to_string());
                
                Err(KillSwitchError::KillSwitchActive(trigger))
            }
        }
    }
    
    /// Check if auto-trigger conditions are met
    pub fn check_auto_trigger(&self, market_state: &MarketState) -> Option<String> {
        // Check extreme loss
        if let Some(extreme_loss) = self.config.auto_trigger_on_extreme_loss {
            if market_state.daily_pnl < Decimal::ZERO 
                && market_state.daily_pnl.abs() >= extreme_loss {
                let loss_pct: f64 = (market_state.daily_pnl * Decimal::from(-100))
                    .try_into().unwrap_or(0.0);
                return Some(format!(
                    "Extreme daily loss: {}%",
                    loss_pct
                ));
            }
        }
        
        // Check correlation breakdown
        if market_state.correlation_breakdown {
            return Some("Market correlation breakdown detected".to_string());
        }
        
        None
    }
    
    /// Activate kill switch
    pub async fn activate(&self, reason: &str, triggered_by: &str) {
        let trigger = KillSwitchTrigger {
            reason: reason.to_string(),
            triggered_by: triggered_by.to_string(),
            triggered_at: Utc::now(),
            reset_at: None,
            reset_by: None,
        };
        
        *self.state.write().await = KillSwitchState::Triggered;
        *self.current_trigger.write().await = Some(trigger.clone());
        self.trigger_history.write().await.push(trigger);
        
        error!("🛑 KILL SWITCH ACTIVATED by {}: {}", triggered_by, reason);
    }
    
    /// Reset kill switch (requires authorization)
    pub async fn reset(&self, operator: &str) -> Result<(), KillSwitchError> {
        if self.config.require_manual_reset && operator == "system" {
            return Err(KillSwitchError::ManualResetRequired);
        }
        
        // Update current trigger with reset info
        if let Some(ref mut trigger) = *self.current_trigger.write().await {
            trigger.reset_at = Some(Utc::now());
            trigger.reset_by = Some(operator.to_string());
        }
        *self.current_trigger.write().await = None;
        
        *self.state.write().await = KillSwitchState::Armed;
        
        info!("✅ Kill switch reset by {}", operator);
        Ok(())
    }
    
    /// Get current state
    pub async fn get_state(&self) -> KillSwitchState {
        *self.state.read().await
    }
    
    /// Get trigger history
    pub async fn get_history(&self, limit: usize) -> Vec<KillSwitchTrigger> {
        let history = self.trigger_history.read().await;
        history.iter().rev().take(limit).cloned().collect()
    }
    
    /// Get current trigger if active
    pub async fn get_current_trigger(&self) -> Option<KillSwitchTrigger> {
        self.current_trigger.read().await.clone()
    }
    
    /// Check if kill switch is armed
    pub async fn is_armed(&self) -> bool {
        matches!(self.get_state().await, KillSwitchState::Armed)
    }
    
    /// Check if kill switch is triggered
    pub async fn is_triggered(&self) -> bool {
        matches!(self.get_state().await, KillSwitchState::Triggered)
    }
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum KillSwitchError {
    #[error("Kill switch is active: {0}")]
    KillSwitchActive(String),
    
    #[error("System is in recovery mode")]
    SystemInRecovery,
    
    #[error("Manual reset required")]
    ManualResetRequired,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_kill_switch_initial_state() {
        let ks = KillSwitch::default();
        
        assert!(ks.is_armed().await);
        assert!(!ks.is_triggered().await);
        assert!(ks.check().await.is_ok());
    }

    #[tokio::test]
    async fn test_kill_switch_activation() {
        let ks = KillSwitch::default();
        
        ks.activate("Test emergency", "operator").await;
        
        assert!(!ks.is_armed().await);
        assert!(ks.is_triggered().await);
        assert!(ks.check().await.is_err());
        
        let trigger = ks.get_current_trigger().await.unwrap();
        assert_eq!(trigger.reason, "Test emergency");
        assert_eq!(trigger.triggered_by, "operator");
    }

    #[tokio::test]
    async fn test_kill_switch_reset() {
        let ks = KillSwitch::default();
        
        ks.activate("Test", "operator").await;
        ks.reset("admin").await.unwrap();
        
        assert!(ks.is_armed().await);
        assert!(ks.check().await.is_ok());
    }

    #[test]
    fn test_auto_trigger_on_loss() {
        let ks = KillSwitch::default();
        
        let market_state = MarketState {
            daily_pnl: Decimal::try_from(-0.20).unwrap(), // 20% loss
            ..Default::default()
        };
        
        let trigger = ks.check_auto_trigger(&market_state);
        assert!(trigger.is_some());
        assert!(trigger.unwrap().contains("Extreme daily loss"));
    }

    #[test]
    fn test_no_auto_trigger_on_small_loss() {
        let ks = KillSwitch::default();
        
        let market_state = MarketState {
            daily_pnl: Decimal::try_from(-0.05).unwrap(), // 5% loss
            ..Default::default()
        };
        
        let trigger = ks.check_auto_trigger(&market_state);
        assert!(trigger.is_none());
    }

    #[tokio::test]
    async fn test_manual_reset_required() {
        let ks = KillSwitch::default();
        
        ks.activate("Test", "operator").await;
        
        // System cannot auto-reset when manual reset required
        let result = ks.reset("system").await;
        assert!(result.is_err());
        
        // Admin can reset
        let result = ks.reset("admin").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_trigger_history() {
        let ks = KillSwitch::default();
        
        ks.activate("Test 1", "op1").await;
        ks.reset("admin").await.ok();
        ks.activate("Test 2", "op2").await;
        
        let history = ks.get_history(10).await;
        assert_eq!(history.len(), 2);
    }
}
