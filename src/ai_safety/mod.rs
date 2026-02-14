//! AI Safety & Control Module
//!
//! Sprint 26: AI Safety & Control
//! - Kill Switch: Emergency stop mechanism
//! - Circuit Breaker: Automatic trading halts
//! - Limit Enforcer: Trading limits and constraints
//! - Guardrails: Ethical constraints and pattern detection
//! - Human Override: Allows human intervention
//! - Explainable Decisions: Decision logging and audit trail

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{error, info, warn};
use uuid::Uuid;

pub mod circuit_breaker;
pub mod explainability;
pub mod guardrails;
pub mod kill_switch;
pub mod limits;
pub mod override_ctrl;

pub use circuit_breaker::{CircuitBreaker, BreakerConfig, BreakerCondition, BreakerAction, MarketState};
pub use explainability::{DecisionExplanation, ExplainableDecision, DecisionAuditor};
pub use guardrails::{Guardrails, OrderCheckRequest, EthicalViolation, ViolationSeverity};
pub use kill_switch::{KillSwitch, KillSwitchConfig, KillSwitchState, KillSwitchTrigger};
pub use limits::{LimitEnforcer, TradingLimits, LimitType, LimitCheck};
pub use override_ctrl::{HumanOverride, OverrideRequest, OverrideStatus, OverrideType};

/// AI Safety errors
#[derive(Error, Debug, Clone)]
pub enum SafetyError {
    #[error("Limit exceeded: {0}")]
    LimitExceeded(String),
    
    #[error("Human override required: {0}")]
    OverrideRequired(String),
    
    #[error("Override denied: {0}")]
    OverrideDenied(String),
    
    #[error("Kill switch activated: {0}")]
    KillSwitch(String),
    
    #[error("Decision blocked: {0}")]
    DecisionBlocked(String),
    
    #[error("Circuit breaker triggered: {0}")]
    CircuitBreaker(String),
    
    #[error("Ethical violation: {0}")]
    EthicalViolation(String),
}

pub type Result<T> = std::result::Result<T, SafetyError>;

/// AI Safety Controller - main coordinator
#[derive(Debug)]
pub struct SafetyController {
    limit_enforcer: RwLock<LimitEnforcer>,
    override_ctrl: RwLock<HumanOverride>,
    auditor: RwLock<DecisionAuditor>,
    kill_switch: RwLock<KillSwitch>,
    circuit_breaker: RwLock<CircuitBreaker>,
    guardrails: RwLock<Guardrails>,
    paused: RwLock<bool>,
}

impl SafetyController {
    /// Create new safety controller with default limits
    pub fn new() -> Self {
        Self {
            limit_enforcer: RwLock::new(LimitEnforcer::default()),
            override_ctrl: RwLock::new(HumanOverride::new()),
            auditor: RwLock::new(DecisionAuditor::new()),
            kill_switch: RwLock::new(KillSwitch::default()),
            circuit_breaker: RwLock::new(CircuitBreaker::default()),
            guardrails: RwLock::new(Guardrails::default()),
            paused: RwLock::new(false),
        }
    }

    /// Create with custom limits
    pub fn with_limits(limits: TradingLimits) -> Self {
        Self {
            limit_enforcer: RwLock::new(LimitEnforcer::new(limits)),
            override_ctrl: RwLock::new(HumanOverride::new()),
            auditor: RwLock::new(DecisionAuditor::new()),
            kill_switch: RwLock::new(KillSwitch::default()),
            circuit_breaker: RwLock::new(CircuitBreaker::default()),
            guardrails: RwLock::new(Guardrails::default()),
            paused: RwLock::new(false),
        }
    }

    /// Check if system is safe to proceed with action
    pub async fn check_action(&self, action: &Action) -> Result<Decision> {
        // Check kill switch first
        let ks = self.kill_switch.read().await;
        if let Err(e) = ks.check().await {
            return Err(SafetyError::KillSwitch(e.to_string()));
        }
        drop(ks);

        // Check circuit breakers
        let cb = self.circuit_breaker.read().await;
        if let Err(e) = cb.check() {
            return Err(SafetyError::CircuitBreaker(e.to_string()));
        }
        drop(cb);

        // Check if paused
        if *self.paused.read().await {
            return Err(SafetyError::OverrideRequired(
                "System is paused, awaiting human intervention".to_string()
            ));
        }

        // Check limits
        let enforcer = self.limit_enforcer.read().await;
        let limit_check = enforcer.check_action(action)?;
        drop(enforcer);

        // If limit would be exceeded, require human override
        if !limit_check.passed {
            let mut override_ctrl = self.override_ctrl.write().await;
            let request = override_ctrl.request_override(
                OverrideType::LimitExceeded,
                format!("Limit check failed: {:?}", limit_check.failed_limits),
                action.clone(),
            );
            drop(override_ctrl);

            return Err(SafetyError::OverrideRequired(
                format!("Override required: {:?}", request.id)
            ));
        }

        // Log the decision
        let mut auditor = self.auditor.write().await;
        let explanation = DecisionExplanation {
            decision_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            action: action.clone(),
            reasoning: format!("All limits passed: {:?}", limit_check),
            factors: vec!["Within trading limits".to_string()],
            confidence: 1.0,
            metadata: std::collections::HashMap::new(),
        };
        auditor.log_decision(explanation.clone());

        Ok(Decision {
            approved: true,
            explanation,
        })
    }

    /// Update market state and check circuit breakers
    pub async fn update_market_state(&self, state: &MarketState) -> Vec<SafetyEvent> {
        let mut events = Vec::new();
        
        // Check circuit breakers
        let mut cb = self.circuit_breaker.write().await;
        if let Some(trigger) = cb.update(state) {
            warn!("Circuit breaker triggered: {}", trigger.breaker_name);
            events.push(SafetyEvent::CircuitBreakerTriggered {
                breaker_name: trigger.breaker_name,
                condition: format!("{:?}", trigger.condition),
                threshold: trigger.threshold,
                actual_value: trigger.actual_value,
            });
        }
        drop(cb);
        
        // Check kill switch conditions
        let ks = self.kill_switch.read().await;
        if let Some(reason) = ks.check_auto_trigger(state) {
            drop(ks);
            error!("Kill switch auto-triggered: {}", reason);
            let ks = self.kill_switch.write().await;
            ks.activate(&reason, "system").await;
            events.push(SafetyEvent::KillSwitchActivated {
                reason: reason.clone(),
                triggered_by: "system".to_string(),
            });
        }
        
        events
    }

    /// Request human override for an action
    pub async fn request_override(
        &self,
        override_type: OverrideType,
        reason: String,
        action: Action,
    ) -> OverrideRequest {
        let mut ctrl = self.override_ctrl.write().await;
        ctrl.request_override(override_type, reason, action)
    }

    /// Approve an override request
    pub async fn approve_override(
        &self,
        request_id: Uuid,
        approver: String,
    ) -> Result<()> {
        let mut ctrl = self.override_ctrl.write().await;
        ctrl.approve_override(request_id, approver)
            .map_err(SafetyError::OverrideDenied)
    }

    /// Deny an override request
    pub async fn deny_override(
        &self,
        request_id: Uuid,
        reason: String,
    ) -> Result<()> {
        let mut ctrl = self.override_ctrl.write().await;
        ctrl.deny_override(request_id, reason)
            .map_err(SafetyError::OverrideDenied)
    }

    /// Activate kill switch - emergency stop
    pub async fn activate_kill_switch(&self, reason: String, operator: &str) {
        warn!("🛑 KILL SWITCH ACTIVATED by {}: {}", operator, reason);
        
        let ks = self.kill_switch.write().await;
        ks.activate(&reason, operator).await;
        drop(ks);
        
        // Log the emergency
        let mut auditor = self.auditor.write().await;
        auditor.log_emergency("Kill Switch Activated", &reason);
    }

    /// Deactivate kill switch (requires manual intervention)
    pub async fn deactivate_kill_switch(&self, operator: &str) -> Result<()> {
        info!("Kill switch deactivation requested by: {}", operator);
        
        let ks = self.kill_switch.write().await;
        ks.reset(operator).await
            .map_err(|e| SafetyError::KillSwitch(format!("Reset failed: {}", e)))?;
        drop(ks);
        
        let mut auditor = self.auditor.write().await;
        auditor.log_event("Kill Switch Deactivated", operator);
        
        Ok(())
    }

    /// Pause system (non-emergency)
    pub async fn pause(&self, reason: String) {
        info!("System paused: {}", reason);
        *self.paused.write().await = true;
        
        let mut auditor = self.auditor.write().await;
        auditor.log_event("System Paused", &reason);
    }

    /// Resume system
    pub async fn resume(&self, operator: String) {
        info!("System resumed by: {}", operator);
        *self.paused.write().await = false;
        
        let mut auditor = self.auditor.write().await;
        auditor.log_event("System Resumed", &operator);
    }

    /// Update trading limits
    pub async fn update_limits(&self, limits: TradingLimits) {
        let mut enforcer = self.limit_enforcer.write().await;
        enforcer.update_limits(limits);
        
        let mut auditor = self.auditor.write().await;
        auditor.log_event("Limits Updated", "");
    }

    /// Get current status
    pub async fn status(&self) -> SafetyStatus {
        let ks = self.kill_switch.read().await;
        let cb = self.circuit_breaker.read().await;
        
        SafetyStatus {
            kill_switch_state: ks.get_state().await,
            paused: *self.paused.read().await,
            pending_overrides: self.override_ctrl.read().await.pending_count(),
            last_decision: self.auditor.read().await.last_decision_time(),
            active_circuit_breakers: cb.get_active(),
        }
    }

    /// Get audit log
    pub async fn get_audit_log(&self, limit: usize) -> Vec<DecisionExplanation> {
        self.auditor.read().await.get_recent(limit)
    }
}

impl Default for SafetyController {
    fn default() -> Self {
        Self::new()
    }
}

/// An action the AI wants to take
#[derive(Debug, Clone)]
pub struct Action {
    pub action_type: ActionType,
    pub symbol: String,
    pub quantity: Decimal,
    pub price: Option<Decimal>,
    pub side: crate::broker::OrderSide,
    pub strategy: String,
    pub confidence: f64,
}

/// Types of actions
#[derive(Debug, Clone)]
pub enum ActionType {
    PlaceOrder,
    CancelOrder,
    ClosePosition,
    ModifyOrder,
    EmergencyFlatten,
}

/// Decision result
#[derive(Debug, Clone)]
pub struct Decision {
    pub approved: bool,
    pub explanation: DecisionExplanation,
}

/// Safety system status
#[derive(Debug, Clone)]
pub struct SafetyStatus {
    pub kill_switch_state: KillSwitchState,
    pub paused: bool,
    pub pending_overrides: usize,
    pub last_decision: Option<DateTime<Utc>>,
    pub active_circuit_breakers: Vec<String>,
}

/// Safety events for reporting
#[derive(Debug, Clone)]
pub enum SafetyEvent {
    KillSwitchActivated {
        reason: String,
        triggered_by: String,
    },
    CircuitBreakerTriggered {
        breaker_name: String,
        condition: String,
        threshold: f64,
        actual_value: f64,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_safety_controller_creation() {
        let controller = SafetyController::new();
        
        let status = controller.status().await;
        assert!(matches!(status.kill_switch_state, KillSwitchState::Armed));
        assert!(!status.paused);
    }

    #[tokio::test]
    async fn test_kill_switch() {
        let controller = SafetyController::new();
        
        controller.activate_kill_switch("Test emergency".to_string(), "operator").await;
        
        let status = controller.status().await;
        assert!(matches!(status.kill_switch_state, KillSwitchState::Triggered));
        
        let action = Action {
            action_type: ActionType::PlaceOrder,
            symbol: "AAPL".to_string(),
            quantity: Decimal::from(100),
            price: Some(Decimal::from(150)),
            side: crate::broker::OrderSide::Buy,
            strategy: "test".to_string(),
            confidence: 0.8,
        };
        
        let result = controller.check_action(&action).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), SafetyError::KillSwitch(_)));
        
        // Reset kill switch
        controller.deactivate_kill_switch("admin").await.unwrap();
        let status = controller.status().await;
        assert!(matches!(status.kill_switch_state, KillSwitchState::Armed));
    }

    #[tokio::test]
    async fn test_pause_and_resume() {
        let controller = SafetyController::new();
        
        controller.pause("Maintenance".to_string()).await;
        
        let status = controller.status().await;
        assert!(status.paused);
        
        controller.resume("admin".to_string()).await;
        
        let status = controller.status().await;
        assert!(!status.paused);
    }
    
    #[tokio::test]
    async fn test_circuit_breaker_integration() {
        let controller = SafetyController::new();
        
        let market_state = MarketState {
            daily_pnl: Decimal::try_from(-0.06).unwrap(), // 6% loss
            ..Default::default()
        };
        
        let events = controller.update_market_state(&market_state).await;
        
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], SafetyEvent::CircuitBreakerTriggered { .. }));
        
        let status = controller.status().await;
        assert_eq!(status.active_circuit_breakers.len(), 1);
    }
}
