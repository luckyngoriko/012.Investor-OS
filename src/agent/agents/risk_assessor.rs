//! Risk Assessor Agent
//!
//! Evaluates trading risks and determines position sizing.
//! Provides risk warnings and recommends maximum position sizes.

use super::*;
use crate::agent::{AgentMessage, MessagePayload, MessageType};
use async_trait::async_trait;
use rust_decimal::Decimal;
use tracing::{debug, info, warn};

/// Risk Assessor Agent implementation
pub struct RiskAssessorAgent {
    config: AgentConfig,
    status: AgentStatus,
    /// Maximum portfolio risk per trade (0.0 - 1.0)
    max_trade_risk: f64,
    /// Maximum portfolio drawdown before halt
    max_drawdown: f64,
    /// Risk-free rate for calculations
    risk_free_rate: f64,
}

impl RiskAssessorAgent {
    pub fn new(config: AgentConfig) -> Self {
        Self {
            config,
            status: AgentStatus::Active,
            max_trade_risk: 0.02, // 2% per trade
            max_drawdown: 0.10,   // 10% max drawdown
            risk_free_rate: 0.05, // 5% risk-free rate
        }
    }
    
    pub fn with_risk_params(mut self, max_trade_risk: f64, max_drawdown: f64) -> Self {
        self.max_trade_risk = max_trade_risk.clamp(0.0, 1.0);
        self.max_drawdown = max_drawdown.clamp(0.0, 1.0);
        self
    }
    
    /// Assess risk for a position
    fn assess_position_risk(&self, position: &PositionInfo) -> RiskAssessment {
        let pnl = match position.side {
            super::super::PositionSide::Long => {
                (position.current_price - position.entry_price) * position.quantity
            }
            super::super::PositionSide::Short => {
                (position.entry_price - position.current_price) * position.quantity
            }
        };
        
        let position_value = position.current_price * position.quantity;
        let unrealized_pct: f64 = if !position.entry_price.is_zero() {
            (pnl / (position.entry_price * position.quantity)).try_into().unwrap_or(0.0)
        } else {
            0.0
        };
        
        // Determine risk level
        let risk_level = if unrealized_pct.abs() > self.max_drawdown {
            RiskLevel::Critical
        } else if unrealized_pct.abs() > self.max_drawdown * 0.7 {
            RiskLevel::High
        } else if unrealized_pct.abs() > self.max_trade_risk {
            RiskLevel::Medium
        } else {
            RiskLevel::Low
        };
        
        // Calculate VaR (simplified)
        let var_95 = position_value * Decimal::try_from(self.max_trade_risk).unwrap_or(Decimal::from(2) / Decimal::from(100));
        
        // Max position size based on volatility (simplified)
        let max_position_size = position_value * Decimal::try_from(1.0 + self.max_trade_risk).unwrap_or(Decimal::from(1));
        
        let mut warnings = Vec::new();
        
        if matches!(risk_level, RiskLevel::High | RiskLevel::Critical) {
            warnings.push(format!(
                "Position shows {:.2}% unrealized {}",
                unrealized_pct.abs() * 100.0,
                if unrealized_pct < 0.0 { "loss" } else { "gain" }
            ));
        }
        
        if position.quantity > max_position_size {
            warnings.push("Position size exceeds recommended maximum".to_string());
        }
        
        RiskAssessment {
            max_position_size,
            var_95,
            risk_level,
            warnings,
        }
    }
    
    /// Calculate Kelly Criterion position size
    fn kelly_criterion(&self, win_rate: f64, avg_win: f64, avg_loss: f64) -> f64 {
        if avg_loss == 0.0 {
            return 0.0;
        }
        
        let kelly = win_rate - ((1.0 - win_rate) / (avg_win / avg_loss.abs()));
        kelly.max(0.0).min(self.max_trade_risk * 2.0) // Cap at 2x max trade risk
    }
}

#[async_trait]
impl Agent for RiskAssessorAgent {
    fn id(&self) -> &AgentId {
        &self.config.id
    }
    
    fn role(&self) -> AgentRole {
        AgentRole::RiskAssessor
    }
    
    fn status(&self) -> AgentStatus {
        self.status
    }
    
    async fn process(&mut self, task: Task) -> Result<TaskResult, AgentError> {
        let start_time = std::time::Instant::now();
        
        let output = match &task.task_type {
            super::super::TaskType::AssessRisk { position } => {
                debug!("RiskAssessor assessing risk for {}", position.symbol);
                
                let assessment = self.assess_position_risk(position);
                
                // Log warnings
                for warning in &assessment.warnings {
                    warn!("Risk warning for {}: {}", position.symbol, warning);
                }
                
                TaskOutput::RiskAssessment(assessment)
            }
            _ => {
                TaskOutput::Error(format!("Unsupported task type for RiskAssessor: {:?}", task.task_type))
            }
        };
        
        let execution_time = start_time.elapsed().as_millis() as u64;
        
        Ok(TaskResult {
            task_id: task.id,
            agent_id: self.config.id.clone(),
            status: TaskStatus::Success,
            output,
            execution_time_ms: execution_time,
        })
    }
    
    async fn on_message(&mut self, msg: AgentMessage) -> Result<(), AgentError> {
        match msg.msg_type {
            MessageType::Warning => {
                // Process risk warnings from other agents
                if let MessagePayload::Warning(warning) = &msg.payload {
                    warn!("RiskAssessor received warning: {}", warning.message);
                }
            }
            MessageType::Request => {
                // Handle risk assessment requests
                debug!("RiskAssessor received request");
            }
            _ => {}
        }
        Ok(())
    }
    
    async fn pause(&mut self) {
        self.status = AgentStatus::Paused;
        info!("RiskAssessor {} paused", self.config.id);
    }
    
    async fn resume(&mut self) {
        self.status = AgentStatus::Active;
        info!("RiskAssessor {} resumed", self.config.id);
    }
    
    async fn shutdown(&mut self) {
        self.status = AgentStatus::Shutdown;
        info!("RiskAssessor {} shutdown", self.config.id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::super::AgentConfig;

    #[tokio::test]
    async fn test_risk_assessor_creation() {
        let config = AgentConfig::new(AgentRole::RiskAssessor, "Risk Agent");
        let agent = RiskAssessorAgent::new(config);
        
        assert_eq!(agent.role(), AgentRole::RiskAssessor);
    }

    #[test]
    fn test_position_risk_assessment() {
        let config = AgentConfig::new(AgentRole::RiskAssessor, "Risk Agent");
        let agent = RiskAssessorAgent::new(config);
        
        let position = PositionInfo {
            symbol: "AAPL".to_string(),
            quantity: Decimal::from(100),
            entry_price: Decimal::try_from(150.0).unwrap(),
            current_price: Decimal::try_from(140.0).unwrap(), // 10% loss
            side: super::super::super::PositionSide::Long,
        };
        
        let assessment = agent.assess_position_risk(&position);
        
        assert!(!assessment.var_95.is_zero());
        // 10% loss should trigger at least Medium risk
        assert!(matches!(assessment.risk_level, RiskLevel::Medium | RiskLevel::High | RiskLevel::Critical));
        // Warnings should be present for significant loss
        if matches!(assessment.risk_level, RiskLevel::High | RiskLevel::Critical) {
            assert!(!assessment.warnings.is_empty());
        }
    }

    #[test]
    fn test_kelly_criterion() {
        let config = AgentConfig::new(AgentRole::RiskAssessor, "Risk Agent");
        let agent = RiskAssessorAgent::new(config);
        
        // 60% win rate, avg win $100, avg loss $50
        let kelly = agent.kelly_criterion(0.6, 100.0, 50.0);
        
        // Kelly = 0.6 - (0.4 / 2) = 0.4
        assert!(kelly > 0.0);
        assert!(kelly <= 0.4);
    }
}
