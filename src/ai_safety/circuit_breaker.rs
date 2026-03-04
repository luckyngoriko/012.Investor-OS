//! Circuit Breaker System
//!
//! Automatic trading halts based on market conditions:
//! - Daily loss limit
//! - Maximum drawdown
//! - Volatility spikes
//! - Correlation breakdowns
//! - Order frequency and rejection spikes

use chrono::{DateTime, Duration, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{info, warn};

/// Circuit breaker condition types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BreakerCondition {
    /// Daily loss percentage
    DailyLoss,
    /// Maximum drawdown from peak
    Drawdown,
    /// Volatility spike multiplier
    VolatilitySpike,
    /// Correlation matrix breakdown
    CorrelationBreakdown,
    /// Trading frequency too high
    FrequencyLimit,
    /// Order rejection rate spike
    OrderRejectionSpike,
}

/// Circuit breaker configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BreakerConfig {
    pub name: String,
    pub condition: BreakerCondition,
    pub threshold: f64,
    pub cooldown_seconds: u64,
    pub auto_reset: bool,
}

impl BreakerConfig {
    pub fn daily_loss_limit(threshold: Decimal) -> Self {
        Self {
            name: "Daily Loss Limit".to_string(),
            condition: BreakerCondition::DailyLoss,
            threshold: threshold.try_into().unwrap_or(0.05),
            cooldown_seconds: 3600, // 1 hour
            auto_reset: false,
        }
    }
    
    pub fn drawdown_limit(threshold: Decimal) -> Self {
        Self {
            name: "Max Drawdown".to_string(),
            condition: BreakerCondition::Drawdown,
            threshold: threshold.try_into().unwrap_or(0.10),
            cooldown_seconds: 86400, // 24 hours
            auto_reset: false,
        }
    }
    
    pub fn volatility_spike(multiplier: f64) -> Self {
        Self {
            name: "Volatility Spike".to_string(),
            condition: BreakerCondition::VolatilitySpike,
            threshold: multiplier,
            cooldown_seconds: 1800, // 30 minutes
            auto_reset: true,
        }
    }

    /// Trigger when number of orders in the last minute reaches a threshold.
    pub fn frequency_limit(max_orders_per_minute: u32) -> Self {
        Self {
            name: "Order Frequency".to_string(),
            condition: BreakerCondition::FrequencyLimit,
            threshold: f64::from(max_orders_per_minute),
            cooldown_seconds: 300, // 5 minutes
            auto_reset: true,
        }
    }

    /// Trigger when rejection ratio in the last minute exceeds threshold.
    pub fn rejection_spike(max_rejection_ratio: f64) -> Self {
        Self {
            name: "Rejection Spike".to_string(),
            condition: BreakerCondition::OrderRejectionSpike,
            threshold: max_rejection_ratio,
            cooldown_seconds: 600, // 10 minutes
            auto_reset: true,
        }
    }
}

/// Circuit breaker rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BreakerRule {
    pub config: BreakerConfig,
    pub triggered: bool,
    pub triggered_at: Option<DateTime<Utc>>,
    pub triggered_value: Option<f64>,
    pub reset_at: Option<DateTime<Utc>>,
}

impl BreakerRule {
    pub fn new(config: BreakerConfig) -> Self {
        Self {
            config,
            triggered: false,
            triggered_at: None,
            triggered_value: None,
            reset_at: None,
        }
    }
    
    pub fn trigger(&mut self, value: f64) {
        if !self.triggered {
            self.triggered = true;
            self.triggered_at = Some(Utc::now());
            self.triggered_value = Some(value);
            warn!(
                "⚡ Circuit breaker '{}' triggered at {:.2} (threshold: {:.2})",
                self.config.name, value, self.config.threshold
            );
        }
    }
    
    pub fn reset(&mut self) {
        if self.triggered {
            self.triggered = false;
            self.reset_at = Some(Utc::now());
            info!("✅ Circuit breaker '{}' reset", self.config.name);
        }
    }
    
    pub fn should_auto_reset(&self) -> bool {
        if !self.triggered || !self.config.auto_reset {
            return false;
        }
        
        if let Some(triggered_at) = self.triggered_at {
            let cooldown = Duration::seconds(self.config.cooldown_seconds as i64);
            Utc::now() > triggered_at + cooldown
        } else {
            false
        }
    }
}

/// Circuit breaker action
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BreakerAction {
    /// Pause new orders only
    PauseNewOrders,
    /// Flatten all positions
    FlattenPositions,
    /// Emergency liquidation
    EmergencyLiquidate,
    /// Full system halt
    FullHalt,
}

/// Circuit breaker trigger information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerInfo {
    pub breaker_name: String,
    pub condition: BreakerCondition,
    pub threshold: f64,
    pub actual_value: f64,
    pub triggered_at: DateTime<Utc>,
    pub action_taken: Option<BreakerAction>,
}

/// Market state for circuit breaker checks
#[derive(Debug, Clone, Default)]
pub struct MarketState {
    pub daily_pnl: Decimal,
    pub current_drawdown: Decimal,
    pub volatility_index: f64,
    pub correlation_breakdown: bool,
    pub orders_last_minute: u32,
    pub rejected_orders_last_minute: u32,
    pub timestamp: DateTime<Utc>,
}

/// Circuit breaker implementation
#[derive(Debug)]
pub struct CircuitBreaker {
    rules: HashMap<String, BreakerRule>,
    trigger_history: Vec<TriggerInfo>,
    baseline_volatility: f64,
}

impl CircuitBreaker {
    pub fn new(configs: Vec<BreakerConfig>) -> Self {
        let mut rules = HashMap::new();
        
        for config in configs {
            rules.insert(config.name.clone(), BreakerRule::new(config));
        }
        
        Self {
            rules,
            trigger_history: Vec::new(),
            baseline_volatility: 0.0,
        }
    }
    
    pub fn default() -> Self {
        Self::new(vec![
            BreakerConfig::daily_loss_limit(Decimal::try_from(0.05).unwrap()),
            BreakerConfig::drawdown_limit(Decimal::try_from(0.10).unwrap()),
            BreakerConfig::volatility_spike(3.0),
        ])
    }
    
    /// Check if any circuit breakers are active
    pub fn check(&self) -> Result<(), CircuitBreakerError> {
        let active_breakers: Vec<String> = self.rules
            .values()
            .filter(|r| r.triggered)
            .map(|r| format!("{} (since {})", r.config.name, 
                r.triggered_at.map(|t| t.to_rfc3339()).unwrap_or_default()))
            .collect();
        
        if !active_breakers.is_empty() {
            Err(CircuitBreakerError::CircuitBreakerActive(active_breakers.join(", ")))
        } else {
            Ok(())
        }
    }
    
    /// Update circuit breaker state with current market conditions
    pub fn update(&mut self, state: &MarketState) -> Option<TriggerInfo> {
        let mut new_trigger = None;
        
        for (name, rule) in &mut self.rules {
            // Skip already triggered rules, check for auto-reset
            if rule.triggered {
                if rule.should_auto_reset() {
                    rule.reset();
                }
                continue;
            }
            
            // Check condition
            let triggered = match rule.config.condition {
                BreakerCondition::DailyLoss => {
                    let daily_loss_pct: f64 = state.daily_pnl.try_into().unwrap_or(0.0);
                    daily_loss_pct < 0.0 && daily_loss_pct.abs() >= rule.config.threshold
                }
                BreakerCondition::Drawdown => {
                    let dd: f64 = state.current_drawdown.try_into().unwrap_or(0.0);
                    dd >= rule.config.threshold
                }
                BreakerCondition::VolatilitySpike => {
                    if self.baseline_volatility > 0.0 {
                        let ratio = state.volatility_index / self.baseline_volatility;
                        ratio >= rule.config.threshold
                    } else {
                        false
                    }
                }
                BreakerCondition::CorrelationBreakdown => state.correlation_breakdown,
                BreakerCondition::FrequencyLimit => {
                    let orders = state.orders_last_minute as f64;
                    orders >= rule.config.threshold
                }
                BreakerCondition::OrderRejectionSpike => {
                    if state.orders_last_minute > 0 {
                        let rejected_ratio = state.rejected_orders_last_minute as f64
                            / state.orders_last_minute as f64;
                        rejected_ratio >= rule.config.threshold
                    } else {
                        false
                    }
                }
            };
            
            if triggered {
                let value = match rule.config.condition {
                    BreakerCondition::DailyLoss => {
                        let val: f64 = state.daily_pnl.try_into().unwrap_or(0.0);
                        val.abs()
                    }
                    BreakerCondition::Drawdown => state.current_drawdown.try_into().unwrap_or(0.0),
                    BreakerCondition::VolatilitySpike => state.volatility_index,
                    BreakerCondition::FrequencyLimit => state.orders_last_minute as f64,
                    BreakerCondition::OrderRejectionSpike => {
                        if state.orders_last_minute > 0 {
                            state.rejected_orders_last_minute as f64 / state.orders_last_minute as f64
                        } else {
                            0.0
                        }
                    }
                    _ => rule.config.threshold,
                };
                
                rule.trigger(value);
                
                let info = TriggerInfo {
                    breaker_name: name.clone(),
                    condition: rule.config.condition,
                    threshold: rule.config.threshold,
                    actual_value: value,
                    triggered_at: Utc::now(),
                    action_taken: Some(BreakerAction::PauseNewOrders),
                };
                
                self.trigger_history.push(info.clone());
                new_trigger = Some(info);
            }
        }
        
        new_trigger
    }
    
    /// Get list of active circuit breakers
    pub fn get_active(&self) -> Vec<String> {
        self.rules
            .values()
            .filter(|r| r.triggered)
            .map(|r| r.config.name.clone())
            .collect()
    }
    
    /// Get specific rule status
    pub fn get_rule(&self, name: &str) -> Option<&BreakerRule> {
        self.rules.get(name)
    }
    
    /// Get trigger history
    pub fn get_history(&self, limit: usize) -> Vec<&TriggerInfo> {
        self.trigger_history.iter().rev().take(limit).collect()
    }
    
    /// Set baseline volatility
    pub fn set_baseline_volatility(&mut self, volatility: f64) {
        self.baseline_volatility = volatility;
    }
    
    /// Manually reset a circuit breaker
    pub fn reset(&mut self, name: &str) -> bool {
        if let Some(rule) = self.rules.get_mut(name) {
            rule.reset();
            true
        } else {
            false
        }
    }
    
    /// Reset all circuit breakers
    pub fn reset_all(&mut self) {
        for rule in self.rules.values_mut() {
            rule.reset();
        }
    }
    
    /// Add a new circuit breaker rule
    pub fn add_rule(&mut self, config: BreakerConfig) {
        self.rules.insert(config.name.clone(), BreakerRule::new(config));
    }
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum CircuitBreakerError {
    #[error("Circuit breaker active: {0}")]
    CircuitBreakerActive(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circuit_breaker_initial_state() {
        let cb = CircuitBreaker::default();
        
        assert!(cb.check().is_ok());
        assert!(cb.get_active().is_empty());
    }

    #[test]
    fn test_daily_loss_trigger() {
        let mut cb = CircuitBreaker::default();
        
        let state = MarketState {
            daily_pnl: Decimal::try_from(-0.06).unwrap(), // 6% loss
            ..Default::default()
        };
        
        let trigger = cb.update(&state);
        
        assert!(trigger.is_some());
        assert!(cb.check().is_err());
        assert_eq!(cb.get_active().len(), 1);
        assert!(cb.get_active()[0].contains("Daily Loss"));
    }

    #[test]
    fn test_drawdown_trigger() {
        let mut cb = CircuitBreaker::default();
        
        let state = MarketState {
            current_drawdown: Decimal::try_from(0.12).unwrap(), // 12% drawdown
            ..Default::default()
        };
        
        let trigger = cb.update(&state);
        
        assert!(trigger.is_some());
        assert!(cb.get_active()[0].contains("Drawdown"));
    }

    #[test]
    fn test_volatility_spike_trigger() {
        let mut cb = CircuitBreaker::default();
        cb.set_baseline_volatility(0.10); // 10% baseline
        
        let state = MarketState {
            volatility_index: 0.35, // 3.5x baseline
            ..Default::default()
        };
        
        let trigger = cb.update(&state);
        
        assert!(trigger.is_some());
        assert!(cb.get_active()[0].contains("Volatility"));
    }

    #[test]
    fn test_frequency_limit_trigger() {
        let mut cb = CircuitBreaker::new(vec![BreakerConfig::frequency_limit(5)]);
        let state = MarketState {
            orders_last_minute: 7,
            ..Default::default()
        };

        let trigger = cb.update(&state);

        assert!(trigger.is_some());
        assert_eq!(cb.get_active().len(), 1);
        assert!(cb.get_active()[0].contains("Order Frequency"));
    }

    #[test]
    fn test_rejection_spike_trigger() {
        let mut cb = CircuitBreaker::new(vec![BreakerConfig::rejection_spike(0.35)]);
        let state = MarketState {
            orders_last_minute: 20,
            rejected_orders_last_minute: 8, // 40%
            ..Default::default()
        };

        let trigger = cb.update(&state);

        assert!(trigger.is_some());
        assert_eq!(cb.get_active().len(), 1);
        assert!(cb.get_active()[0].contains("Rejection"));
    }

    #[test]
    fn test_no_trigger_within_limits() {
        let mut cb = CircuitBreaker::default();
        
        let state = MarketState {
            daily_pnl: Decimal::try_from(-0.02).unwrap(),
            current_drawdown: Decimal::try_from(0.05).unwrap(),
            ..Default::default()
        };
        
        let trigger = cb.update(&state);
        
        assert!(trigger.is_none());
        assert!(cb.check().is_ok());
    }

    #[test]
    fn test_manual_reset() {
        let mut cb = CircuitBreaker::default();
        
        let state = MarketState {
            daily_pnl: Decimal::try_from(-0.06).unwrap(),
            ..Default::default()
        };
        cb.update(&state);
        assert!(cb.check().is_err());
        
        cb.reset("Daily Loss Limit");
        assert!(cb.check().is_ok());
    }

    #[test]
    fn test_trigger_history() {
        let mut cb = CircuitBreaker::default();
        
        let state1 = MarketState {
            daily_pnl: Decimal::try_from(-0.06).unwrap(),
            ..Default::default()
        };
        cb.update(&state1);
        
        cb.reset_all();
        
        let state2 = MarketState {
            current_drawdown: Decimal::try_from(0.12).unwrap(),
            ..Default::default()
        };
        cb.update(&state2);
        
        let history = cb.get_history(10);
        assert_eq!(history.len(), 2);
    }
}
