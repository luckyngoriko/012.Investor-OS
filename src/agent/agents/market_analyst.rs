//! Market Analyst Agent
//!
//! Performs technical and fundamental analysis on market data.
//! Provides trading signals based on price action, indicators, and trends.

use super::*;
use crate::agent::{AgentMessage, MessagePayload, MessageType};
use async_trait::async_trait;
use rust_decimal::Decimal;
use std::collections::HashMap;
use tracing::{debug, info};

/// Market Analyst Agent implementation
pub struct MarketAnalystAgent {
    config: AgentConfig,
    status: AgentStatus,
    /// Technical indicator configurations
    indicator_config: IndicatorConfig,
    /// Recent analyses cache
    analysis_cache: HashMap<String, MarketAnalysis>,
    /// Track record for weighting
    successful_calls: u32,
    total_calls: u32,
}

/// Technical indicator configuration
#[derive(Debug, Clone)]
pub struct IndicatorConfig {
    pub rsi_period: usize,
    pub macd_fast: usize,
    pub macd_slow: usize,
    pub macd_signal: usize,
    pub bb_period: usize,
    pub bb_std_dev: f64,
    pub sma_periods: Vec<usize>,
}

impl Default for IndicatorConfig {
    fn default() -> Self {
        Self {
            rsi_period: 14,
            macd_fast: 12,
            macd_slow: 26,
            macd_signal: 9,
            bb_period: 20,
            bb_std_dev: 2.0,
            sma_periods: vec![20, 50, 200],
        }
    }
}

impl MarketAnalystAgent {
    pub fn new(config: AgentConfig) -> Self {
        Self {
            config,
            status: AgentStatus::Active,
            indicator_config: IndicatorConfig::default(),
            analysis_cache: HashMap::new(),
            successful_calls: 0,
            total_calls: 0,
        }
    }
    
    pub fn with_indicators(mut self, config: IndicatorConfig) -> Self {
        self.indicator_config = config;
        self
    }
    
    /// Perform technical analysis on a symbol
    fn analyze_technical(&self, symbol: &str) -> MarketAnalysis {
        // In production, this would fetch real price data and calculate indicators
        // For now, generate deterministic analysis based on symbol hash
        
        let symbol_hash = symbol.bytes().fold(0u32, |acc, b| acc.wrapping_add(b as u32));
        let trend_value = symbol_hash % 5;
        
        let trend = match trend_value {
            0 => TrendDirection::StrongUp,
            1 => TrendDirection::Up,
            2 => TrendDirection::Sideways,
            3 => TrendDirection::Down,
            _ => TrendDirection::StrongDown,
        };
        
        let (recommendation, confidence) = match trend {
            TrendDirection::StrongUp => (RecommendedAction::StrongBuy, 0.85),
            TrendDirection::Up => (RecommendedAction::Buy, 0.70),
            TrendDirection::Sideways => (RecommendedAction::Hold, 0.50),
            TrendDirection::Down => (RecommendedAction::Sell, 0.70),
            TrendDirection::StrongDown => (RecommendedAction::StrongSell, 0.85),
        };
        
        // Generate support/resistance levels
        let base_price = 100.0 + (symbol_hash % 200) as f64;
        let support_levels = vec![
            Decimal::try_from(base_price * 0.95).unwrap_or(Decimal::from(95)),
            Decimal::try_from(base_price * 0.90).unwrap_or(Decimal::from(90)),
        ];
        let resistance_levels = vec![
            Decimal::try_from(base_price * 1.05).unwrap_or(Decimal::from(105)),
            Decimal::try_from(base_price * 1.10).unwrap_or(Decimal::from(110)),
        ];
        
        let rationale = format!(
            "Trend analysis shows {:?} with RSI({}) at neutral level. \
             MACD({},{},{}) indicates {} momentum. \
             Price is near {} support/resistance levels.",
            trend,
            self.indicator_config.rsi_period,
            self.indicator_config.macd_fast,
            self.indicator_config.macd_slow,
            self.indicator_config.macd_signal,
            if matches!(trend, TrendDirection::Up | TrendDirection::StrongUp) { "bullish" } else { "bearish" },
            if matches!(recommendation, RecommendedAction::Buy | RecommendedAction::StrongBuy) { "above" } else { "below" }
        );
        
        MarketAnalysis {
            symbol: symbol.to_string(),
            trend,
            confidence,
            support_levels,
            resistance_levels,
            recommended_action: recommendation,
            rationale,
        }
    }
    
    /// Get agent weight based on track record
    fn get_weight(&self) -> f64 {
        if self.total_calls == 0 {
            1.0 // Default weight
        } else {
            let accuracy = self.successful_calls as f64 / self.total_calls as f64;
            0.5 + accuracy // Weight between 0.5 and 1.5 based on accuracy
        }
    }
}

#[async_trait]
impl Agent for MarketAnalystAgent {
    fn id(&self) -> &AgentId {
        &self.config.id
    }
    
    fn role(&self) -> AgentRole {
        AgentRole::MarketAnalyst
    }
    
    fn status(&self) -> AgentStatus {
        self.status
    }
    
    async fn process(&mut self, task: Task) -> Result<TaskResult, AgentError> {
        let start_time = std::time::Instant::now();
        
        let output = match &task.task_type {
            super::super::TaskType::AnalyzeMarket { symbol } => {
                debug!("MarketAnalyst analyzing {}", symbol);
                
                let analysis = self.analyze_technical(symbol);
                self.analysis_cache.insert(symbol.clone(), analysis.clone());
                
                TaskOutput::MarketAnalysis(analysis)
            }
            _ => {
                TaskOutput::Error(format!("Unsupported task type for MarketAnalyst: {:?}", task.task_type))
            }
        };
        
        self.total_calls += 1;
        
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
            MessageType::Request => {
                // Handle vote requests
                if let MessagePayload::Request(req) = &msg.payload {
                    if req.request_type == "vote" {
                        info!("MarketAnalyst received vote request");
                        // Would respond with vote in real implementation
                    }
                }
            }
            MessageType::Observation => {
                // Process market observations
                debug!("MarketAnalyst received observation");
            }
            _ => {}
        }
        Ok(())
    }
    
    async fn pause(&mut self) {
        self.status = AgentStatus::Paused;
        info!("MarketAnalyst {} paused", self.config.id);
    }
    
    async fn resume(&mut self) {
        self.status = AgentStatus::Active;
        info!("MarketAnalyst {} resumed", self.config.id);
    }
    
    async fn shutdown(&mut self) {
        self.status = AgentStatus::Shutdown;
        info!("MarketAnalyst {} shutdown", self.config.id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::super::AgentConfig;

    #[tokio::test]
    async fn test_market_analyst_creation() {
        let config = AgentConfig::new(AgentRole::MarketAnalyst, "Test Analyst");
        let agent = MarketAnalystAgent::new(config);
        
        assert_eq!(agent.role(), AgentRole::MarketAnalyst);
        assert!(matches!(agent.status(), AgentStatus::Active));
    }

    #[tokio::test]
    async fn test_technical_analysis() {
        let config = AgentConfig::new(AgentRole::MarketAnalyst, "Test Analyst");
        let agent = MarketAnalystAgent::new(config);
        
        let analysis = agent.analyze_technical("AAPL");
        
        assert_eq!(analysis.symbol, "AAPL");
        assert!(analysis.confidence > 0.0 && analysis.confidence <= 1.0);
        assert!(!analysis.support_levels.is_empty());
        assert!(!analysis.resistance_levels.is_empty());
        assert!(!analysis.rationale.is_empty());
    }

    #[tokio::test]
    async fn test_task_processing() {
        let config = AgentConfig::new(AgentRole::MarketAnalyst, "Test Analyst");
        let mut agent = MarketAnalystAgent::new(config.clone());
        
        let task = Task {
            id: super::super::super::TaskId::new(),
            task_type: super::super::super::TaskType::AnalyzeMarket { symbol: "TSLA".to_string() },
            payload: serde_json::Value::Null,
            deadline: None,
            priority: Priority::Normal,
        };
        
        let result = agent.process(task).await.unwrap();
        
        assert_eq!(result.agent_id, config.id);
        assert!(matches!(result.status, TaskStatus::Success));
        
        match result.output {
            TaskOutput::MarketAnalysis(analysis) => {
                assert_eq!(analysis.symbol, "TSLA");
            }
            _ => panic!("Expected MarketAnalysis output"),
        }
    }

    #[test]
    fn test_weight_calculation() {
        let config = AgentConfig::new(AgentRole::MarketAnalyst, "Test");
        let mut agent = MarketAnalystAgent::new(config);
        
        // Default weight
        assert_eq!(agent.get_weight(), 1.0);
        
        // After some successful calls
        agent.total_calls = 10;
        agent.successful_calls = 8; // 80% accuracy
        
        let weight = agent.get_weight();
        assert!(weight > 1.0); // Should be above 1.0 for good accuracy
    }
}
