//! Learner Agent
//!
//! Analyzes past trades and optimizes strategies.
//! Continuously improves based on performance feedback.

use super::*;
use crate::agent::{AgentMessage, MessageType};
use async_trait::async_trait;
use rust_decimal::Decimal;
use std::collections::HashMap;
use tracing::{debug, info};

/// Learner Agent implementation
pub struct LearnerAgent {
    config: AgentConfig,
    status: AgentStatus,
    /// Strategy performance tracking
    strategy_performance: HashMap<String, StrategyStats>,
    /// Trade history
    trade_history: Vec<TradeResult>,
    /// Learning rate for adjustments
    learning_rate: f64,
}

/// Strategy performance statistics
#[derive(Debug, Clone, Default)]
struct StrategyStats {
    wins: u32,
    losses: u32,
    total_pnl: Decimal,
    avg_win: Decimal,
    avg_loss: Decimal,
    max_consecutive_wins: u32,
    max_consecutive_losses: u32,
}

impl LearnerAgent {
    pub fn new(config: AgentConfig) -> Self {
        Self {
            config,
            status: AgentStatus::Active,
            strategy_performance: HashMap::new(),
            trade_history: Vec::new(),
            learning_rate: 0.1,
        }
    }
    
    pub fn with_learning_rate(mut self, rate: f64) -> Self {
        self.learning_rate = rate.clamp(0.0, 1.0);
        self
    }
    
    /// Learn from a trade result
    fn learn_from_trade(&mut self, trade: &TradeResult) {
        self.trade_history.push(trade.clone());
        
        // Update strategy performance (using exit_reason as strategy identifier)
        let strategy_key = trade.exit_reason.clone();
        let stats = self.strategy_performance
            .entry(strategy_key.clone())
            .or_default();
        
        if trade.pnl > Decimal::ZERO {
            stats.wins += 1;
            stats.avg_win = if stats.wins == 1 {
                trade.pnl
            } else {
                (stats.avg_win * Decimal::from(stats.wins - 1) + trade.pnl) 
                    / Decimal::from(stats.wins)
            };
        } else {
            stats.losses += 1;
            stats.avg_loss = if stats.losses == 1 {
                trade.pnl
            } else {
                (stats.avg_loss * Decimal::from(stats.losses - 1) + trade.pnl) 
                    / Decimal::from(stats.losses)
            };
        }
        
        stats.total_pnl += trade.pnl;
        
        debug!(
            "Learner recorded trade for strategy '{}': PnL={}",
            strategy_key, trade.pnl
        );
    }
    
    /// Generate strategy adjustments
    fn generate_adjustments(&self) -> LearningUpdate {
        let mut adjustments = HashMap::new();
        let mut insights = Vec::new();
        let mut total_performance_delta = 0.0;
        
        for (strategy, stats) in &self.strategy_performance {
            let total_trades = stats.wins + stats.losses;
            if total_trades == 0 {
                continue;
            }
            
            let win_rate = stats.wins as f64 / total_trades as f64;
            let profit_factor = if !stats.avg_loss.is_zero() {
                let avg_win_f64: f64 = (stats.avg_win * Decimal::from(stats.wins)).try_into().unwrap_or(0.0);
                let avg_loss_f64: f64 = (stats.avg_loss.abs() * Decimal::from(stats.losses)).try_into().unwrap_or(1.0);
                avg_win_f64 / avg_loss_f64
            } else {
                999.0 // No losses yet
            };
            
            // Calculate adjustment based on performance
            let adjustment = if win_rate > 0.6 && profit_factor > 1.5 {
                // Strategy is working well, increase allocation
                self.learning_rate
            } else if win_rate < 0.4 || profit_factor < 0.8 {
                // Strategy is underperforming, decrease allocation
                -self.learning_rate
            } else {
                // Neutral, small positive adjustment for exploration
                self.learning_rate * 0.1
            };
            
            adjustments.insert(strategy.clone(), adjustment);
            total_performance_delta += adjustment;
            
            // Generate insight
            let insight = format!(
                "Strategy '{}': Win rate {:.1}%, Profit factor {:.2}, Total PnL {}",
                strategy,
                win_rate * 100.0,
                profit_factor,
                stats.total_pnl
            );
            insights.push(insight);
            
            if win_rate < 0.4 {
                insights.push(format!(
                    "WARNING: Strategy '{}' has low win rate, consider review",
                    strategy
                ));
            }
        }
        
        LearningUpdate {
            strategy_adjustments: adjustments,
            insights,
            performance_delta: total_performance_delta,
        }
    }
    
    /// Get best performing strategy
    fn get_best_strategy(&self) -> Option<(String, StrategyStats)> {
        self.strategy_performance
            .iter()
            .max_by(|(_, a), (_, b)| {
                let a_score = if a.losses == 0 {
                    a.wins as f64 * 100.0
                } else {
                    (a.wins as f64) / (a.losses as f64)
                };
                let b_score = if b.losses == 0 {
                    b.wins as f64 * 100.0
                } else {
                    (b.wins as f64) / (b.losses as f64)
                };
                a_score.partial_cmp(&b_score).unwrap()
            })
            .map(|(k, v)| (k.clone(), v.clone()))
    }
    
    /// Calculate overall win rate
    fn overall_win_rate(&self) -> f64 {
        let (wins, losses) = self.strategy_performance.values()
            .fold((0, 0), |(w, l), stats| {
                (w + stats.wins, l + stats.losses)
            });
        
        let total = wins + losses;
        if total == 0 {
            0.0
        } else {
            wins as f64 / total as f64
        }
    }
}

#[async_trait]
impl Agent for LearnerAgent {
    fn id(&self) -> &AgentId {
        &self.config.id
    }
    
    fn role(&self) -> AgentRole {
        AgentRole::Learner
    }
    
    fn status(&self) -> AgentStatus {
        self.status
    }
    
    async fn process(&mut self, task: Task) -> Result<TaskResult, AgentError> {
        let start_time = std::time::Instant::now();
        
        let output = match &task.task_type {
            super::super::TaskType::LearnFromTrade { trade } => {
                debug!("Learner processing trade result for {}", trade.symbol);
                
                self.learn_from_trade(trade);
                
                // Generate insights
                let update = self.generate_adjustments();
                
                info!(
                    "Learner generated {} insights, overall win rate: {:.1}%",
                    update.insights.len(),
                    self.overall_win_rate() * 100.0
                );
                
                for insight in &update.insights {
                    info!("  - {}", insight);
                }
                
                TaskOutput::LearningUpdate(update)
            }
            _ => {
                TaskOutput::Error(format!("Unsupported task type for Learner: {:?}", task.task_type))
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
        if msg.msg_type == MessageType::Observation {
            // Could process market observations for pattern learning
            debug!("Learner received observation");
        }
        Ok(())
    }
    
    async fn pause(&mut self) {
        self.status = AgentStatus::Paused;
        info!("Learner {} paused", self.config.id);
    }
    
    async fn resume(&mut self) {
        self.status = AgentStatus::Active;
        info!("Learner {} resumed", self.config.id);
    }
    
    async fn shutdown(&mut self) {
        self.status = AgentStatus::Shutdown;
        info!("Learner {} shutdown", self.config.id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::super::AgentConfig;

    #[tokio::test]
    async fn test_learner_creation() {
        let config = AgentConfig::new(AgentRole::Learner, "Learning Agent");
        let agent = LearnerAgent::new(config);
        
        assert_eq!(agent.role(), AgentRole::Learner);
    }

    #[test]
    fn test_learning_from_trades() {
        let config = AgentConfig::new(AgentRole::Learner, "Learning Agent");
        let mut agent = LearnerAgent::new(config);
        
        // Add some winning trades
        for _ in 0..7 {
            agent.learn_from_trade(&TradeResult {
                symbol: "AAPL".to_string(),
                entry_price: Decimal::try_from(100.0).unwrap(),
                exit_price: Decimal::try_from(105.0).unwrap(),
                quantity: Decimal::from(10),
                pnl: Decimal::try_from(50.0).unwrap(),
                duration_secs: 3600,
                exit_reason: "momentum".to_string(),
            });
        }
        
        // Add some losing trades
        for _ in 0..3 {
            agent.learn_from_trade(&TradeResult {
                symbol: "TSLA".to_string(),
                entry_price: Decimal::try_from(200.0).unwrap(),
                exit_price: Decimal::try_from(195.0).unwrap(),
                quantity: Decimal::from(10),
                pnl: Decimal::try_from(-50.0).unwrap(),
                duration_secs: 3600,
                exit_reason: "stop_loss".to_string(),
            });
        }
        
        let win_rate = agent.overall_win_rate();
        assert!((win_rate - 0.7).abs() < 0.01); // 70% win rate
        
        let update = agent.generate_adjustments();
        assert!(!update.insights.is_empty());
        assert!(!update.strategy_adjustments.is_empty());
    }

    #[test]
    fn test_best_strategy_selection() {
        let config = AgentConfig::new(AgentRole::Learner, "Learning Agent");
        let mut agent = LearnerAgent::new(config);
        
        // Strategy 1: Good performance
        for _ in 0..8 {
            agent.learn_from_trade(&TradeResult {
                symbol: "A".to_string(),
                entry_price: Decimal::from(100),
                exit_price: Decimal::from(105),
                quantity: Decimal::from(1),
                pnl: Decimal::from(5),
                duration_secs: 100,
                exit_reason: "strategy_a".to_string(),
            });
        }
        
        // Strategy 2: Poor performance
        for _ in 0..5 {
            agent.learn_from_trade(&TradeResult {
                symbol: "B".to_string(),
                entry_price: Decimal::from(100),
                exit_price: Decimal::from(95),
                quantity: Decimal::from(1),
                pnl: Decimal::from(-5),
                duration_secs: 100,
                exit_reason: "strategy_b".to_string(),
            });
        }
        
        let best = agent.get_best_strategy();
        assert!(best.is_some());
        let (name, _) = best.unwrap();
        assert_eq!(name, "strategy_a");
    }

    #[test]
    fn test_strategy_adjustments() {
        let config = AgentConfig::new(AgentRole::Learner, "Learning Agent");
        let mut agent = LearnerAgent::new(config).with_learning_rate(0.1);
        
        // Excellent strategy
        for _ in 0..10 {
            agent.learn_from_trade(&TradeResult {
                symbol: "BTC".to_string(),
                entry_price: Decimal::from(100),
                exit_price: Decimal::from(110),
                quantity: Decimal::from(1),
                pnl: Decimal::from(10),
                duration_secs: 100,
                exit_reason: "excellent".to_string(),
            });
        }
        
        let update = agent.generate_adjustments();
        
        // Good strategy should get positive adjustment
        let excellent_adj = update.strategy_adjustments.get("excellent").unwrap();
        assert!(*excellent_adj > 0.0);
    }
}
