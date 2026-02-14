//! AI Guardrails System
//!
//! Ethical and safety guardrails for AI trading decisions:
//! - Pattern detection for suspicious behavior
//! - Ethical constraint enforcement
//! - Market manipulation prevention
//! - Prohibited symbol filtering

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use tracing::warn;

/// Guardrails configuration
#[derive(Debug, Clone)]
pub struct Guardrails {
    enabled: bool,
    prohibited_symbols: Vec<String>,
    suspicious_patterns: Vec<PatternRule>,
    ethical_violations: VecDeque<EthicalViolation>,
    pattern_history: VecDeque<TradingPattern>,
    max_history_size: usize,
}

impl Guardrails {
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            prohibited_symbols: Self::default_prohibited_symbols(),
            suspicious_patterns: Self::default_suspicious_patterns(),
            ethical_violations: VecDeque::new(),
            pattern_history: VecDeque::new(),
            max_history_size: 1000,
        }
    }
    
    pub fn default() -> Self {
        Self::new(true)
    }
    
    /// Check order against guardrails
    pub fn check_pattern(&mut self, order: &OrderCheckRequest) -> Result<(), GuardrailError> {
        if !self.enabled {
            return Ok(());
        }
        
        // Check prohibited symbols
        if self.prohibited_symbols.contains(&order.symbol.to_uppercase()) {
            let violation = EthicalViolation {
                violation_type: "Prohibited Symbol".to_string(),
                symbol: Some(order.symbol.clone()),
                description: format!("Trading {} is prohibited", order.symbol),
                timestamp: Utc::now(),
                severity: ViolationSeverity::Critical,
            };
            self.record_violation(violation.clone());
            
            return Err(GuardrailError::ProhibitedSymbol(order.symbol.clone()));
        }
        
        // Check suspicious patterns
        for rule in &self.suspicious_patterns.clone() {
            if let Some(violation) = self.check_rule(rule, order) {
                self.record_violation(violation.clone());
                
                if matches!(violation.severity, ViolationSeverity::Critical | ViolationSeverity::High) {
                    return Err(GuardrailError::SuspiciousPattern(violation.description));
                }
            }
        }
        
        // Record pattern
        let pattern = TradingPattern {
            symbol: order.symbol.clone(),
            side: format!("{:?}", order.side),
            quantity: order.quantity,
            timestamp: Utc::now(),
        };
        self.record_pattern(pattern);
        
        Ok(())
    }
    
    /// Check a specific rule
    fn check_rule(&self, rule: &PatternRule, order: &OrderCheckRequest) -> Option<EthicalViolation> {
        match rule.rule_type {
            PatternRuleType::RapidFireOrders => {
                let recent: Vec<_> = self.pattern_history
                    .iter()
                    .filter(|p| p.symbol == order.symbol)
                    .filter(|p| (Utc::now() - p.timestamp).num_seconds() < rule.window_seconds as i64)
                    .collect();
                
                if recent.len() as u32 >= rule.threshold {
                    Some(EthicalViolation {
                        violation_type: "Rapid Fire Orders".to_string(),
                        symbol: Some(order.symbol.clone()),
                        description: format!(
                            "{} orders for {} in {} seconds",
                            recent.len(), order.symbol, rule.window_seconds
                        ),
                        timestamp: Utc::now(),
                        severity: rule.severity.clone(),
                    })
                } else {
                    None
                }
            }
            PatternRuleType::WashTrading => {
                let recent = self.pattern_history
                    .iter()
                    .filter(|p| p.symbol == order.symbol)
                    .filter(|p| (Utc::now() - p.timestamp).num_seconds() < rule.window_seconds as i64)
                    .count();
                
                if recent > 1 {
                    Some(EthicalViolation {
                        violation_type: "Potential Wash Trading".to_string(),
                        symbol: Some(order.symbol.clone()),
                        description: format!(
                            "Rapid buy/sell pattern detected for {}",
                            order.symbol
                        ),
                        timestamp: Utc::now(),
                        severity: rule.severity.clone(),
                    })
                } else {
                    None
                }
            }
            PatternRuleType::Layering => {
                let recent = self.pattern_history
                    .iter()
                    .filter(|p| p.symbol == order.symbol)
                    .filter(|p| (Utc::now() - p.timestamp).num_seconds() < rule.window_seconds as i64)
                    .count();
                
                if recent as u32 >= rule.threshold {
                    Some(EthicalViolation {
                        violation_type: "Potential Layering".to_string(),
                        symbol: Some(order.symbol.clone()),
                        description: format!(
                            "Multiple price levels for {} detected",
                            order.symbol
                        ),
                        timestamp: Utc::now(),
                        severity: rule.severity.clone(),
                    })
                } else {
                    None
                }
            }
            PatternRuleType::ExcessiveSize => {
                if order.quantity > Decimal::from(rule.threshold) {
                    Some(EthicalViolation {
                        violation_type: "Excessive Order Size".to_string(),
                        symbol: Some(order.symbol.clone()),
                        description: format!(
                            "Order size {} exceeds threshold for {}",
                            order.quantity, order.symbol
                        ),
                        timestamp: Utc::now(),
                        severity: rule.severity.clone(),
                    })
                } else {
                    None
                }
            }
        }
    }
    
    fn record_pattern(&mut self, pattern: TradingPattern) {
        self.pattern_history.push_back(pattern);
        if self.pattern_history.len() > self.max_history_size {
            self.pattern_history.pop_front();
        }
    }
    
    fn record_violation(&mut self, violation: EthicalViolation) {
        warn!(
            "🚨 Ethical violation detected: {} - {}",
            violation.violation_type, violation.description
        );
        
        self.ethical_violations.push_back(violation);
        if self.ethical_violations.len() > self.max_history_size {
            self.ethical_violations.pop_front();
        }
    }
    
    pub fn get_recent_violations(&self, limit: usize) -> Vec<EthicalViolation> {
        self.ethical_violations
            .iter()
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }
    
    pub fn get_symbol_patterns(&self, symbol: &str, window_seconds: u64) -> Vec<&TradingPattern> {
        self.pattern_history
            .iter()
            .filter(|p| p.symbol == symbol)
            .filter(|p| (Utc::now() - p.timestamp).num_seconds() < window_seconds as i64)
            .collect()
    }
    
    pub fn add_prohibited_symbol(&mut self, symbol: &str) {
        let upper = symbol.to_uppercase();
        if !self.prohibited_symbols.contains(&upper) {
            self.prohibited_symbols.push(upper);
        }
    }
    
    pub fn remove_prohibited_symbol(&mut self, symbol: &str) {
        self.prohibited_symbols.retain(|s| s != &symbol.to_uppercase());
    }
    
    fn default_prohibited_symbols() -> Vec<String> {
        vec![]
    }
    
    fn default_suspicious_patterns() -> Vec<PatternRule> {
        vec![
            PatternRule {
                rule_type: PatternRuleType::RapidFireOrders,
                threshold: 10,
                window_seconds: 60,
                severity: ViolationSeverity::High,
            },
            PatternRule {
                rule_type: PatternRuleType::WashTrading,
                threshold: 2,
                window_seconds: 300,
                severity: ViolationSeverity::Critical,
            },
            PatternRule {
                rule_type: PatternRuleType::ExcessiveSize,
                threshold: 100000,
                window_seconds: 0,
                severity: ViolationSeverity::Medium,
            },
        ]
    }
}

/// Order check request
#[derive(Debug, Clone)]
pub struct OrderCheckRequest {
    pub symbol: String,
    pub side: OrderSide,
    pub quantity: Decimal,
    pub price: Option<Decimal>,
}

#[derive(Debug, Clone, Copy)]
pub enum OrderSide {
    Buy,
    Sell,
}

/// Trading pattern entry
#[derive(Debug, Clone)]
pub struct TradingPattern {
    pub symbol: String,
    pub side: String,
    pub quantity: Decimal,
    pub timestamp: DateTime<Utc>,
}

/// Ethical violation record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EthicalViolation {
    pub violation_type: String,
    pub symbol: Option<String>,
    pub description: String,
    pub timestamp: DateTime<Utc>,
    pub severity: ViolationSeverity,
}

/// Violation severity levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ViolationSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Pattern rule configuration
#[derive(Debug, Clone)]
struct PatternRule {
    rule_type: PatternRuleType,
    threshold: u32,
    window_seconds: u64,
    severity: ViolationSeverity,
}

/// Pattern rule types
#[derive(Debug, Clone)]
enum PatternRuleType {
    RapidFireOrders,
    WashTrading,
    Layering,
    ExcessiveSize,
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum GuardrailError {
    #[error("Prohibited symbol: {0}")]
    ProhibitedSymbol(String),
    
    #[error("Suspicious pattern detected: {0}")]
    SuspiciousPattern(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_order(symbol: &str) -> OrderCheckRequest {
        OrderCheckRequest {
            symbol: symbol.to_string(),
            side: OrderSide::Buy,
            quantity: Decimal::ONE,
            price: Some(Decimal::ONE),
        }
    }

    #[test]
    fn test_guardrails_enabled() {
        let mut guardrails = Guardrails::new(true);
        let order = create_test_order("AAPL");
        
        assert!(guardrails.check_pattern(&order).is_ok());
    }

    #[test]
    fn test_guardrails_disabled() {
        let mut guardrails = Guardrails::new(false);
        guardrails.add_prohibited_symbol("WEAPONS");
        let order = create_test_order("WEAPONS");
        
        assert!(guardrails.check_pattern(&order).is_ok());
    }

    #[test]
    fn test_prohibited_symbol() {
        let mut guardrails = Guardrails::new(true);
        guardrails.add_prohibited_symbol("WEAPONS");
        
        let order = create_test_order("WEAPONS");
        let result = guardrails.check_pattern(&order);
        
        assert!(result.is_err());
    }

    #[test]
    fn test_rapid_fire_detection() {
        let mut guardrails = Guardrails::new(true);
        
        for _ in 0..10 {
            let order = create_test_order("AAPL");
            guardrails.check_pattern(&order).ok();
        }
        
        let order = create_test_order("AAPL");
        let result = guardrails.check_pattern(&order);
        
        assert!(result.is_err());
    }

    #[test]
    fn test_violation_history() {
        let mut guardrails = Guardrails::new(true);
        guardrails.add_prohibited_symbol("BAD");
        
        let order = create_test_order("BAD");
        guardrails.check_pattern(&order).ok();
        
        let violations = guardrails.get_recent_violations(10);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].violation_type, "Prohibited Symbol");
    }
}
