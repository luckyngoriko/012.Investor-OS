//! Execution Specialist Agent
//!
//! Optimizes order execution and routing.
//! Determines best venues, timing, and order splitting strategies.

use super::*;
use crate::agent::AgentMessage;
use async_trait::async_trait;
use rust_decimal::Decimal;
use std::time::Duration;
use tracing::{debug, info};

/// Execution Specialist Agent implementation
pub struct ExecutionSpecialistAgent {
    config: AgentConfig,
    status: AgentStatus,
    /// Preferred venues ranked by reliability
    venue_preferences: Vec<String>,
    /// Default TWAP slices
    default_twap_slices: u32,
    /// Slippage model parameters
    slippage_model: SlippageModel,
}

/// Slippage estimation model
#[derive(Debug, Clone)]
pub struct SlippageModel {
    /// Base slippage in bps
    pub base_slippage_bps: f64,
    /// Slippage per unit of order size
    pub size_impact_factor: f64,
    /// Volatility adjustment
    pub vol_adjustment: f64,
}

impl Default for SlippageModel {
    fn default() -> Self {
        Self {
            base_slippage_bps: 2.0,
            size_impact_factor: 0.1,
            vol_adjustment: 1.0,
        }
    }
}

impl ExecutionSpecialistAgent {
    pub fn new(config: AgentConfig) -> Self {
        Self {
            config,
            status: AgentStatus::Active,
            venue_preferences: vec![
                "smart_router".to_string(),
                "exchange_direct".to_string(),
                "dark_pool".to_string(),
            ],
            default_twap_slices: 5,
            slippage_model: SlippageModel::default(),
        }
    }
    
    /// Create execution plan for an order
    fn create_execution_plan(&self, order: &OrderInfo) -> ExecutionPlan {
        // Determine optimal venue
        let optimal_venue = self.select_venue(order);
        
        // Determine timing strategy
        let timing = self.determine_timing(order);
        
        // Calculate order splitting
        let slices = self.calculate_slices(order);
        
        // Estimate slippage
        let expected_slippage = self.estimate_slippage(order);
        
        ExecutionPlan {
            optimal_venue,
            timing,
            order_splitting: slices,
            expected_slippage_bps: expected_slippage,
        }
    }
    
    /// Select best venue for order
    fn select_venue(&self, _order: &OrderInfo) -> String {
        // In production: analyze liquidity, fees, latency per venue
        // For now, return preferred venue
        self.venue_preferences.first()
            .cloned()
            .unwrap_or_else(|| "smart_router".to_string())
    }
    
    /// Determine execution timing strategy
    fn determine_timing(&self, order: &OrderInfo) -> ExecutionTiming {
        let qty: f64 = order.quantity.try_into().unwrap_or(100.0);
        
        if qty > 10000.0 {
            // Large order: use TWAP
            ExecutionTiming::TWAP {
                duration: Duration::from_secs(300), // 5 minutes
                slices: self.default_twap_slices,
            }
        } else if qty > 1000.0 {
            // Medium order: wait for favorable conditions
            ExecutionTiming::WaitFor(Duration::from_secs(60))
        } else {
            // Small order: immediate execution
            ExecutionTiming::Immediate
        }
    }
    
    /// Calculate order slices for execution
    fn calculate_slices(&self, order: &OrderInfo) -> Vec<OrderSlice> {
        let total_qty = order.quantity;
        let base_qty = total_qty / Decimal::from(self.default_twap_slices);
        
        let mut slices = Vec::new();
        let delay_between_slices = 60000u64; // 1 minute
        
        for i in 0..self.default_twap_slices {
            // Vary slice sizes slightly (±20%) to avoid detection
            let variation = if i % 2 == 0 { 
                Decimal::from(12) / Decimal::from(10) 
            } else { 
                Decimal::from(8) / Decimal::from(10) 
            };
            
            let slice_qty = base_qty * variation;
            
            slices.push(OrderSlice {
                quantity: slice_qty.min(total_qty),
                delay_ms: i as u64 * delay_between_slices,
            });
        }
        
        slices
    }
    
    /// Estimate expected slippage in basis points
    fn estimate_slippage(&self, order: &OrderInfo) -> f64 {
        let qty: f64 = order.quantity.try_into().unwrap_or(0.0);
        let size_impact = qty * self.slippage_model.size_impact_factor / 1000.0;
        
        self.slippage_model.base_slippage_bps + size_impact
    }
}

#[async_trait]
impl Agent for ExecutionSpecialistAgent {
    fn id(&self) -> &AgentId {
        &self.config.id
    }
    
    fn role(&self) -> AgentRole {
        AgentRole::ExecutionSpecialist
    }
    
    fn status(&self) -> AgentStatus {
        self.status
    }
    
    async fn process(&mut self, task: Task) -> Result<TaskResult, AgentError> {
        let start_time = std::time::Instant::now();
        
        let output = match &task.task_type {
            super::super::TaskType::OptimizeExecution { order } => {
                debug!("ExecutionSpecialist optimizing execution for {}", order.symbol);
                
                let plan = self.create_execution_plan(order);
                
                info!(
                    "Execution plan for {}: venue={}, timing={:?}, est_slippage={:.2}bps",
                    order.symbol,
                    plan.optimal_venue,
                    plan.timing,
                    plan.expected_slippage_bps
                );
                
                TaskOutput::ExecutionPlan(plan)
            }
            _ => {
                TaskOutput::Error(format!("Unsupported task type for ExecutionSpecialist: {:?}", task.task_type))
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
    
    async fn on_message(&mut self, _msg: AgentMessage) -> Result<(), AgentError> {
        // Execution specialist primarily responds to direct task requests
        Ok(())
    }
    
    async fn pause(&mut self) {
        self.status = AgentStatus::Paused;
        info!("ExecutionSpecialist {} paused", self.config.id);
    }
    
    async fn resume(&mut self) {
        self.status = AgentStatus::Active;
        info!("ExecutionSpecialist {} resumed", self.config.id);
    }
    
    async fn shutdown(&mut self) {
        self.status = AgentStatus::Shutdown;
        info!("ExecutionSpecialist {} shutdown", self.config.id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::super::AgentConfig;

    #[tokio::test]
    async fn test_execution_specialist_creation() {
        let config = AgentConfig::new(AgentRole::ExecutionSpecialist, "Execution Agent");
        let agent = ExecutionSpecialistAgent::new(config);
        
        assert_eq!(agent.role(), AgentRole::ExecutionSpecialist);
    }

    #[test]
    fn test_execution_plan_creation() {
        let config = AgentConfig::new(AgentRole::ExecutionSpecialist, "Execution Agent");
        let agent = ExecutionSpecialistAgent::new(config);
        
        let order = OrderInfo {
            symbol: "BTC".to_string(),
            quantity: Decimal::try_from(50000.0).unwrap(), // Large order
            side: super::super::super::OrderSide::Buy,
            order_type: super::super::super::OrderType::Market,
        };
        
        let plan = agent.create_execution_plan(&order);
        
        assert!(!plan.optimal_venue.is_empty());
        assert!(plan.expected_slippage_bps > 0.0);
        
        // Large order should use TWAP
        match plan.timing {
            ExecutionTiming::TWAP { duration, slices } => {
                assert_eq!(slices, 5);
                assert!(duration.as_secs() > 0);
            }
            _ => panic!("Expected TWAP for large order"),
        }
    }

    #[test]
    fn test_small_order_immediate() {
        let config = AgentConfig::new(AgentRole::ExecutionSpecialist, "Execution Agent");
        let agent = ExecutionSpecialistAgent::new(config);
        
        let order = OrderInfo {
            symbol: "AAPL".to_string(),
            quantity: Decimal::try_from(100.0).unwrap(), // Small order
            side: super::super::super::OrderSide::Buy,
            order_type: super::super::super::OrderType::Market,
        };
        
        let timing = agent.determine_timing(&order);
        
        match timing {
            ExecutionTiming::Immediate => {}
            _ => panic!("Expected Immediate for small order"),
        }
    }

    #[test]
    fn test_slippage_estimation() {
        let config = AgentConfig::new(AgentRole::ExecutionSpecialist, "Execution Agent");
        let agent = ExecutionSpecialistAgent::new(config);
        
        let small_order = OrderInfo {
            symbol: "TSLA".to_string(),
            quantity: Decimal::try_from(100.0).unwrap(),
            side: super::super::super::OrderSide::Buy,
            order_type: super::super::super::OrderType::Market,
        };
        
        let large_order = OrderInfo {
            symbol: "TSLA".to_string(),
            quantity: Decimal::try_from(100000.0).unwrap(),
            side: super::super::super::OrderSide::Buy,
            order_type: super::super::super::OrderType::Market,
        };
        
        let small_slippage = agent.estimate_slippage(&small_order);
        let large_slippage = agent.estimate_slippage(&large_order);
        
        // Large order should have higher slippage
        assert!(large_slippage > small_slippage);
    }
}
