//! LangGraph-inspired State Machine Framework for Rust
//!
//! Контролира decision flow като граф:
//! - Nodes: Агенти/функции
//! - Edges: Правила за преход
//! - State: Споделено състояние
//! - Conditional edges: Адаптивна логика

pub mod graph;
pub mod nodes;
pub mod edges;
pub mod state;

pub use graph::{Graph, GraphBuilder, GraphExecutor};
pub use nodes::{Node, NodeOutput, NodeError};
pub use edges::{Edge, EdgeCondition, ConditionalEdge};
pub use state::{SharedState, StateSnapshot, StateBuilder, MarketRegime, TradingAction, ExecutionStatus, RiskCheck};

use std::sync::Arc;

/// Главен executor за графове
pub struct LangGraphEngine {
    graphs: HashMap<String, Arc<dyn Graph>>,
    telemetry: Option<Telemetry>,
}

use std::collections::HashMap;

impl LangGraphEngine {
    pub fn new() -> Self {
        Self {
            graphs: HashMap::new(),
            telemetry: None,
        }
    }
    
    pub fn register_graph(&mut self, name: impl Into<String>, graph: Arc<dyn Graph>) {
        self.graphs.insert(name.into(), graph);
    }
    
    pub async fn execute(
        &self, 
        graph_name: &str, 
        initial_state: SharedState
    ) -> Result<SharedState, GraphError> {
        let graph = self.graphs.get(graph_name)
            .ok_or(GraphError::GraphNotFound(graph_name.to_string()))?;
        
        let executor = GraphExecutor::new(graph.clone(), self.telemetry.clone());
        executor.run(initial_state).await
    }
}

impl Default for LangGraphEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct Telemetry;

#[derive(Debug, thiserror::Error)]
pub enum GraphError {
    #[error("Graph not found: {0}")]
    GraphNotFound(String),
    #[error("Node error: {0}")]
    NodeError(String),
    #[error("Edge error: {0}")]
    EdgeError(String),
    #[error("Execution error: {0}")]
    ExecutionError(String),
    #[error("Max iterations exceeded")]
    MaxIterationsExceeded,
    #[error("Loop detected")]
    LoopDetected,
}

// Пример за Investor OS Trading Graph:
//
// let trading_graph = GraphBuilder::new("trading_decision")
//     .add_node("start", StartNode)
//     .add_node("collect_data", DataCollectionNode::new(market_service))
//     .add_node("detect_regime", RegimeDetectionNode::new(ml_service))
//     .add_node("breakout", BreakoutStrategyNode)
//     .add_node("mean_rev", MeanReversionStrategyNode)
//     .add_node("cq_calc", CQCalculationNode)
//     .add_node("risk_check", RiskCheckNode::new(risk_service))
//     .add_node("execute", ExecutionNode::new(broker_service))
//     
//     // Linear edges
//     .add_edge("start", "collect_data")
//     .add_edge("collect_data", "detect_regime")
//     
//     // Conditional based on regime
//     .add_conditional_edge(
//         "detect_regime",
//         |state| matches!(state.regime, MarketRegime::Trending),
//         "breakout"
//     )
//     .add_conditional_edge(
//         "detect_regime", 
//         |state| matches!(state.regime, MarketRegime::RangeBound),
//         "mean_rev"
//     )
//     .add_conditional_edge(
//         "detect_regime",
//         |state| matches!(state.regime, MarketRegime::Volatile),
//         "risk_check"  // Skip to risk check in volatile
//     )
//     
//     // Merge back
//     .add_edge("breakout", "cq_calc")
//     .add_edge("mean_rev", "cq_calc")
//     
//     // Risk gate
//     .add_edge("cq_calc", "risk_check")
//     
//     .add_conditional_edge(
//         "risk_check",
//         |state| state.risk_approved && state.cq.value() >= 0.7,
//         "execute"
//     )
//     
//     // Loop for re-evaluation
//     .add_loop(
//         "execute",
//         |state| state.should_recheck(),
//         "collect_data"
//     )
//     
//     .build()?;
