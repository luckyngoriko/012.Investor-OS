//! Explainable AI - Decision Auditing
//!
//! Records and explains AI decisions for transparency and compliance

use chrono::{DateTime, Utc};
use std::collections::HashMap;
use tracing::info;
use uuid::Uuid;

use super::Action;

/// Decision auditor maintains a log of all AI decisions
#[derive(Debug)]
pub struct DecisionAuditor {
    decisions: Vec<DecisionExplanation>,
    emergency_events: Vec<EmergencyEvent>,
    system_events: Vec<SystemEvent>,
    max_history: usize,
}

/// Explanation of an AI decision
#[derive(Debug, Clone)]
pub struct DecisionExplanation {
    pub decision_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub action: Action,
    pub reasoning: String,
    pub factors: Vec<String>,
    pub confidence: f64,
    pub metadata: HashMap<String, String>,
}

/// Trait for explainable decisions
pub trait ExplainableDecision {
    fn explain(&self) -> DecisionExplanation;
}

/// Emergency event record
#[derive(Debug, Clone)]
pub struct EmergencyEvent {
    pub event_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub event_type: String,
    pub description: String,
    pub action_taken: String,
}

/// System event record
#[derive(Debug, Clone)]
pub struct SystemEvent {
    pub timestamp: DateTime<Utc>,
    pub event_type: String,
    pub details: String,
}

impl DecisionAuditor {
    /// Create new auditor with default history size
    pub fn new() -> Self {
        Self::with_capacity(10000)
    }

    /// Create with specific history capacity
    pub fn with_capacity(max_history: usize) -> Self {
        Self {
            decisions: Vec::with_capacity(max_history),
            emergency_events: Vec::new(),
            system_events: Vec::with_capacity(1000),
            max_history,
        }
    }

    /// Log a decision
    pub fn log_decision(&mut self, explanation: DecisionExplanation) {
        info!(
            "AI Decision [{}]: {:?} {} {} - Reason: {}",
            explanation.decision_id,
            explanation.action.side,
            explanation.action.quantity,
            explanation.action.symbol,
            explanation.reasoning
        );

        // Add to history, maintaining max size
        if self.decisions.len() >= self.max_history {
            self.decisions.remove(0);
        }
        self.decisions.push(explanation);
    }

    /// Log an emergency event
    pub fn log_emergency(&mut self, event_type: &str, description: &str) {
        let event = EmergencyEvent {
            event_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            event_type: event_type.to_string(),
            description: description.to_string(),
            action_taken: "Kill switch activated".to_string(),
        };

        info!(
            "EMERGENCY EVENT [{}]: {} - {}",
            event.event_id, event.event_type, event.description
        );

        self.emergency_events.push(event);
    }

    /// Log a system event
    pub fn log_event(&mut self, event_type: &str, details: &str) {
        let event = SystemEvent {
            timestamp: Utc::now(),
            event_type: event_type.to_string(),
            details: details.to_string(),
        };

        info!("System Event: {} - {}", event.event_type, event.details);

        // Maintain max size
        if self.system_events.len() >= 1000 {
            self.system_events.remove(0);
        }
        self.system_events.push(event);
    }

    /// Get recent decisions
    pub fn get_recent(&self, count: usize) -> Vec<DecisionExplanation> {
        let start = self.decisions.len().saturating_sub(count);
        self.decisions[start..].to_vec()
    }

    /// Get decisions by time range
    pub fn get_by_time_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Vec<&DecisionExplanation> {
        self.decisions.iter()
            .filter(|d| d.timestamp >= start && d.timestamp <= end)
            .collect()
    }

    /// Get decisions for a specific symbol
    pub fn get_by_symbol(&self, symbol: &str) -> Vec<&DecisionExplanation> {
        self.decisions.iter()
            .filter(|d| d.action.symbol == symbol)
            .collect()
    }

    /// Get last decision time
    pub fn last_decision_time(&self) -> Option<DateTime<Utc>> {
        self.decisions.last().map(|d| d.timestamp)
    }

    /// Get all emergency events
    pub fn emergency_events(&self) -> &[EmergencyEvent] {
        &self.emergency_events
    }

    /// Get recent system events
    pub fn recent_events(&self, count: usize) -> Vec<&SystemEvent> {
        let start = self.system_events.len().saturating_sub(count);
        self.system_events.iter().skip(start).collect()
    }

    /// Generate summary report
    pub fn generate_report(&self, since: DateTime<Utc>) -> AuditReport {
        let recent_decisions: Vec<_> = self.decisions.iter()
            .filter(|d| d.timestamp >= since)
            .collect();

        let total = recent_decisions.len();
        let by_symbol = self.count_by_symbol(&recent_decisions);
        let avg_confidence = if total > 0 {
            recent_decisions.iter().map(|d| d.confidence).sum::<f64>() / total as f64
        } else {
            0.0
        };

        AuditReport {
            period_start: since,
            period_end: Utc::now(),
            total_decisions: total,
            decisions_by_symbol: by_symbol,
            average_confidence: avg_confidence,
            emergency_count: self.emergency_events.len(),
        }
    }

    fn count_by_symbol(&self, decisions: &[&DecisionExplanation]) -> HashMap<String, usize> {
        let mut counts = HashMap::new();
        for d in decisions {
            *counts.entry(d.action.symbol.clone()).or_insert(0) += 1;
        }
        counts
    }

    /// Export to JSON format
    pub fn export_json(&self) -> String {
        // Simplified export - in production, use serde_json
        format!(
            "{{\"total_decisions\": {}, \"emergencies\": {}}}",
            self.decisions.len(),
            self.emergency_events.len()
        )
    }
}

impl Default for DecisionAuditor {
    fn default() -> Self {
        Self::new()
    }
}

/// Audit report summary
#[derive(Debug, Clone)]
pub struct AuditReport {
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub total_decisions: usize,
    pub decisions_by_symbol: HashMap<String, usize>,
    pub average_confidence: f64,
    pub emergency_count: usize,
}

impl DecisionExplanation {
    /// Create new explanation
    pub fn new(
        action: Action,
        reasoning: String,
        factors: Vec<String>,
        confidence: f64,
    ) -> Self {
        Self {
            decision_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            action,
            reasoning,
            factors,
            confidence,
            metadata: HashMap::new(),
        }
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: &str, value: &str) -> Self {
        self.metadata.insert(key.to_string(), value.to_string());
        self
    }

    /// Format as human-readable text
    pub fn to_human_readable(&self) -> String {
        format!(
            "Decision {} at {}:\n\
             Action: {:?} {} {} @ {:?}\n\
             Reasoning: {}\n\
             Factors: {}\n\
             Confidence: {:.2}%",
            self.decision_id,
            self.timestamp,
            self.action.side,
            self.action.quantity,
            self.action.symbol,
            self.action.price,
            self.reasoning,
            self.factors.join(", "),
            self.confidence * 100.0
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;

    fn create_test_action() -> Action {
        Action {
            action_type: super::super::ActionType::PlaceOrder,
            symbol: "AAPL".to_string(),
            quantity: Decimal::from(100),
            price: Some(Decimal::from(150)),
            side: crate::broker::OrderSide::Buy,
            strategy: "test".to_string(),
            confidence: 0.85,
        }
    }

    #[test]
    fn test_log_decision() {
        let mut auditor = DecisionAuditor::new();
        
        let explanation = DecisionExplanation::new(
            create_test_action(),
            "Strong momentum signal".to_string(),
            vec!["RSI breakout".to_string(), "Volume spike".to_string()],
            0.85,
        );

        auditor.log_decision(explanation);
        
        assert_eq!(auditor.get_recent(10).len(), 1);
    }

    #[test]
    fn test_log_emergency() {
        let mut auditor = DecisionAuditor::new();
        
        auditor.log_emergency("Kill Switch", "Large drawdown detected");
        
        assert_eq!(auditor.emergency_events().len(), 1);
    }

    #[test]
    fn test_get_by_symbol() {
        let mut auditor = DecisionAuditor::new();
        
        let mut action = create_test_action();
        auditor.log_decision(DecisionExplanation::new(
            action.clone(),
            "Test".to_string(),
            vec![],
            0.5,
        ));

        action.symbol = "GOOGL".to_string();
        auditor.log_decision(DecisionExplanation::new(
            action,
            "Test".to_string(),
            vec![],
            0.5,
        ));

        let aapl_decisions = auditor.get_by_symbol("AAPL");
        assert_eq!(aapl_decisions.len(), 1);
    }

    #[test]
    fn test_max_history() {
        let mut auditor = DecisionAuditor::with_capacity(5);
        
        for i in 0..10 {
            let mut action = create_test_action();
            action.symbol = format!("SYM{}", i);
            auditor.log_decision(DecisionExplanation::new(
                action,
                "Test".to_string(),
                vec![],
                0.5,
            ));
        }

        // Should only keep last 5
        assert_eq!(auditor.get_recent(100).len(), 5);
    }

    #[test]
    fn test_human_readable() {
        let explanation = DecisionExplanation::new(
            create_test_action(),
            "Test reasoning".to_string(),
            vec!["Factor 1".to_string()],
            0.85,
        );

        let text = explanation.to_human_readable();
        assert!(text.contains("AAPL"));
        assert!(text.contains("Test reasoning"));
        assert!(text.contains("85.00%"));
    }
}
