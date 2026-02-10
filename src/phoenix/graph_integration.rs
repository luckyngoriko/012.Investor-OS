//! Phoenix + LangGraph Integration
//!
//! Позволява на Phoenix Engine да използва LangGraph за decision flow.

use super::{PhoenixConfig, TradingDecision, Action};
use crate::langgraph::{
    Graph, GraphBuilder, SharedState, StateBuilder, MarketRegime, TradingAction,
    nodes::{Node, NodeOutput, NodeError, StartNode, CQCalculationNode},
};
use crate::signals::TickerSignals;
use rust_decimal::Decimal;
use async_trait::async_trait;
use std::sync::Arc;

/// Phoenix Graph Engine — използва LangGraph за trading decisions
pub struct PhoenixGraphEngine {
    pub config: PhoenixConfig,
    trading_graph: Arc<dyn Graph>,
}

impl PhoenixGraphEngine {
    pub fn new(config: PhoenixConfig) -> Self {
        let graph = Self::build_trading_graph();
        
        Self {
            config,
            trading_graph: Arc::new(graph),
        }
    }
    
    /// Създава trading graph с всички nodes
    fn build_trading_graph() -> impl Graph {
        GraphBuilder::new("phoenix_trading")
            .add_node("start", StartNode)
            .add_node("collect_signals", CollectSignalsNode)
            .add_node("detect_regime", DetectRegimeNode)
            .add_node("apply_strategy", ApplyStrategyNode)
            .add_node("cq_calculation", CQCalculationNode)
            .add_node("risk_check", RiskCheckNode::new(0.7))
            .add_node("make_decision", MakeDecisionNode)
            
            // Linear flow
            .add_edge("start", "collect_signals")
            .add_edge("collect_signals", "detect_regime")
            .add_edge("detect_regime", "apply_strategy")
            .add_edge("apply_strategy", "cq_calculation")
            .add_edge("cq_calculation", "risk_check")
            
            // Conditional: само ако CQ е достатъчно висок
            .add_conditional_edge(
                "risk_check",
                |state| state.risk_approved && state.conviction_quotient.is_some_and(|cq| cq >= 0.7),
                "make_decision"
            )
            
            .set_start("start")
            .build()
            .expect("Failed to build trading graph")
    }
    
    /// Генерира trading decision за даден ticker
    pub async fn generate_decision(
        &self,
        ticker: &str,
        signals: TickerSignals,
        price: Decimal,
    ) -> Result<TradingDecision, GraphError> {
        // Build initial state
        let state = StateBuilder::new(ticker)
            .with_price(price)
            .with_quality_score(signals.quality_score.inner())
            .with_insider_score(signals.insider_score.inner())
            .with_sentiment_score(signals.sentiment_score.inner())
            .build();
        
        // Set remaining scores
        let mut state = state;
        state.regime_fit = Some(signals.regime_fit.inner());
        state.breakout_score = Some(signals.breakout_score);
        state.atr_trend = Some(signals.atr_trend);
        
        // Execute graph
        let final_state = self.trading_graph.execute(state).await?;
        
        // Convert to TradingDecision
        Ok(self.state_to_decision(&final_state))
    }
    
    fn state_to_decision(&self, state: &SharedState) -> TradingDecision {
        let action = match state.action {
            Some(TradingAction::Buy) => Action::Buy,
            Some(TradingAction::Sell) => Action::Sell,
            _ => Action::Hold,
        };
        
        TradingDecision {
            action,
            ticker: state.ticker.clone(),
            quantity: state.position_size.map(|p| p.to_string().parse().unwrap_or(0)),
            confidence: state.confidence.unwrap_or(0.0),
            rationale: format!(
                "CQ: {:.2}, Risk: {}",
                state.conviction_quotient.unwrap_or(0.0),
                state.risk_approved
            ),
        }
    }
}

// ==================== Custom Nodes for Phoenix ====================

/// Collects signals from various sources
pub struct CollectSignalsNode;

#[async_trait]
impl Node for CollectSignalsNode {
    fn name(&self) -> &str { "collect_signals" }
    
    async fn execute(&self, state: SharedState) -> Result<NodeOutput, NodeError> {
        // In real implementation, this would fetch from signal services
        // For now, signals are already in state
        tracing::info!("Collected signals for {}", state.ticker);
        Ok(NodeOutput::Continue(state))
    }
}

/// Detects market regime based on data
pub struct DetectRegimeNode;

#[async_trait]
impl Node for DetectRegimeNode {
    fn name(&self) -> &str { "detect_regime" }
    
    async fn execute(&self, mut state: SharedState) -> Result<NodeOutput, NodeError> {
        // Simple regime detection based on breakout score
        state.market_regime = if state.breakout_score.map_or(0.0, |s| s) > 0.6 {
            MarketRegime::Trending
        } else if state.breakout_score.map_or(0.0, |s| s) < 0.3 {
            MarketRegime::RangeBound
        } else {
            MarketRegime::Volatile
        };
        
        tracing::info!("Detected regime: {:?} for {}", state.market_regime, state.ticker);
        Ok(NodeOutput::Continue(state))
    }
}

/// Applies strategy based on regime
pub struct ApplyStrategyNode;

#[async_trait]
impl Node for ApplyStrategyNode {
    fn name(&self) -> &str { "apply_strategy" }
    
    async fn execute(&self, mut state: SharedState) -> Result<NodeOutput, NodeError> {
        // Adjust weights based on regime
        match state.market_regime {
            MarketRegime::Trending => {
                // Increase breakout weight
                state.breakout_score = state.breakout_score.map(|s| (s * 1.2).min(1.0));
            }
            MarketRegime::RangeBound => {
                // Decrease breakout weight
                state.breakout_score = state.breakout_score.map(|s| s * 0.8);
            }
            _ => {}
        }
        
        Ok(NodeOutput::Continue(state))
    }
}

/// Checks risk limits
pub struct RiskCheckNode {
    min_cq: f64,
    max_position_size: Decimal,
}

impl RiskCheckNode {
    pub fn new(min_cq: f64) -> Self {
        Self {
            min_cq,
            max_position_size: Decimal::from(10000), // Max $10k per position
        }
    }
}

#[async_trait]
impl Node for RiskCheckNode {
    fn name(&self) -> &str { "risk_check" }
    
    async fn execute(&self, mut state: SharedState) -> Result<NodeOutput, NodeError> {
        let cq = state.conviction_quotient.unwrap_or(0.0);
        
        // Check CQ threshold
        if cq < self.min_cq {
            state.risk_approved = false;
            state.risk_checks.push(crate::langgraph::state::RiskCheck {
                check_type: "min_cq".to_string(),
                passed: false,
                details: format!("CQ {:.2} below threshold {:.2}", cq, self.min_cq),
            });
            return Ok(NodeOutput::Continue(state));
        }
        
        // Calculate position size based on CQ
        let position_pct = cq.min(0.25); // Max 25% of available capital
        state.position_size = Some(self.max_position_size * Decimal::try_from(position_pct).unwrap_or(Decimal::ZERO));
        
        state.risk_approved = true;
        state.risk_checks.push(crate::langgraph::state::RiskCheck {
            check_type: "min_cq".to_string(),
            passed: true,
            details: format!("CQ {:.2} above threshold {:.2}", cq, self.min_cq),
        });
        
        Ok(NodeOutput::Continue(state))
    }
}

/// Makes final trading decision
pub struct MakeDecisionNode;

#[async_trait]
impl Node for MakeDecisionNode {
    fn name(&self) -> &str { "make_decision" }
    
    async fn execute(&self, mut state: SharedState) -> Result<NodeOutput, NodeError> {
        let cq = state.conviction_quotient.unwrap_or(0.0);
        
        // Simple decision logic
        state.action = if cq > 0.8 {
            Some(TradingAction::Buy)
        } else if cq < 0.3 {
            Some(TradingAction::Sell)
        } else {
            Some(TradingAction::Hold)
        };
        
        state.confidence = Some(cq);
        state.execution_status = crate::langgraph::state::ExecutionStatus::Completed;
        
        tracing::info!(
            "Decision for {}: {:?} with confidence {:.2}",
            state.ticker,
            state.action,
            cq
        );
        
        Ok(NodeOutput::End(state))
    }
}

use crate::langgraph::GraphError;

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_phoenix_graph_engine_creation() {
        let config = PhoenixConfig::default();
        let engine = PhoenixGraphEngine::new(config);
        
        // Should create without panicking
        assert_eq!(engine.config.currency, "EUR");
    }
    
    #[tokio::test]
    async fn test_detect_regime_node() {
        let node = DetectRegimeNode;
        let mut state = SharedState::new("AAPL");
        state.breakout_score = Some(0.8);
        
        let result = node.execute(state).await.unwrap();
        
        match result {
            NodeOutput::Continue(new_state) => {
                assert!(matches!(new_state.market_regime, MarketRegime::Trending));
            }
            _ => panic!("Expected Continue"),
        }
    }
    
    #[tokio::test]
    async fn test_risk_check_node_pass() {
        let node = RiskCheckNode::new(0.7);
        let mut state = SharedState::new("AAPL");
        state.conviction_quotient = Some(0.75);
        
        let result = node.execute(state).await.unwrap();
        
        match result {
            NodeOutput::Continue(new_state) => {
                assert!(new_state.risk_approved);
                assert!(new_state.position_size.is_some());
            }
            _ => panic!("Expected Continue"),
        }
    }
    
    #[tokio::test]
    async fn test_risk_check_node_fail() {
        let node = RiskCheckNode::new(0.7);
        let mut state = SharedState::new("AAPL");
        state.conviction_quotient = Some(0.5);
        
        let result = node.execute(state).await.unwrap();
        
        match result {
            NodeOutput::Continue(new_state) => {
                assert!(!new_state.risk_approved);
            }
            _ => panic!("Expected Continue"),
        }
    }
}
