//! Coverage tests for Graph implementations
//!
//! Fills gaps from coverage analysis

use investor_os::langgraph::{
    state::{SharedState, MarketRegime, ExecutionStatus, RiskCheck, StateBuilder},
    edges::{Edge, EdgeCondition, ConditionalEdge, conditions},
    graph::GraphBuilder,
    GraphError,
};

#[test]
fn test_execution_status_variants() {
    use investor_os::langgraph::state::ExecutionStatus::*;
    
    assert!(matches!(Pending, Pending));
    assert!(matches!(InProgress, InProgress));
    assert!(matches!(Completed, Completed));
    assert!(matches!(Failed, Failed));
    assert!(matches!(Skipped, Skipped));
    assert!(matches!(Rejected, Rejected));
}

#[test]
fn test_shared_state_is_complete() {
    let mut state = SharedState::new("AAPL");
    
    assert!(!state.is_complete());
    
    state.execution_status = ExecutionStatus::Completed;
    assert!(state.is_complete());
    
    state.execution_status = ExecutionStatus::Skipped;
    assert!(state.is_complete());
    
    state.execution_status = ExecutionStatus::Failed;
    assert!(!state.is_complete());
}

#[test]
fn test_shared_state_add_error() {
    let mut state = SharedState::new("AAPL");
    
    state.add_error("test_node", "Something went wrong");
    
    assert_eq!(state.errors.len(), 1);
    assert_eq!(state.errors[0].node, "test_node");
    assert_eq!(state.errors[0].message, "Something went wrong");
}

#[test]
fn test_risk_check_creation() {
    let check = RiskCheck {
        check_type: "max_position_size".to_string(),
        passed: true,
        details: "Position size within limits".to_string(),
    };
    
    assert_eq!(check.check_type, "max_position_size");
    assert!(check.passed);
}

#[test]
fn test_edge_condition_always() {
    let state = SharedState::new("AAPL");
    
    assert!(EdgeCondition::Always.evaluate(&state));
}

#[test]
fn test_edge_condition_boxed() {
    let condition = EdgeCondition::Boxed(Box::new(|state| {
        state.ticker == "AAPL"
    }));
    
    let aapl_state = SharedState::new("AAPL");
    let msft_state = SharedState::new("MSFT");
    
    assert!(condition.evaluate(&aapl_state));
    assert!(!condition.evaluate(&msft_state));
}

#[test]
fn test_conditional_edge_evaluation() {
    let edge = ConditionalEdge::with_predicate(
        "node_a",
        "node_b",
        |state| state.ticker == "AAPL"
    );
    
    assert_eq!(edge.from(), "node_a");
    assert_eq!(edge.to(), "node_b");
    
    let state = SharedState::new("AAPL");
    assert!(edge.can_transition(&state));
}

#[test]
fn test_conditions_combinators() {
    let high_cq_state = SharedState {
        conviction_quotient: Some(0.8),
        risk_approved: true,
        ..SharedState::new("AAPL")
    };
    
    let low_cq_state = SharedState {
        conviction_quotient: Some(0.5),
        risk_approved: true,
        ..SharedState::new("AAPL")
    };
    
    let combined = conditions::and(
        conditions::cq_above(0.7),
        conditions::risk_approved
    );
    
    assert!(combined(&high_cq_state));
    assert!(!combined(&low_cq_state));
}

#[test]
fn test_conditions_or() {
    let state1 = SharedState {
        market_regime: MarketRegime::Trending,
        ..SharedState::new("AAPL")
    };
    
    let combined = conditions::or(
        conditions::is_trending,
        conditions::is_range_bound
    );
    
    assert!(combined(&state1));
}

#[test]
fn test_graph_builder_validation() {
    // Graph without start node should fail
    let result = GraphBuilder::new("test")
        .add_node("node1", investor_os::langgraph::nodes::StartNode)
        .build();
    
    assert!(result.is_err());
}

#[test]
fn test_graph_builder_with_start() {
    use investor_os::langgraph::nodes::StartNode;
    
    let result = GraphBuilder::new("test")
        .add_node("start", StartNode)
        .set_start("start")
        .build();
    
    assert!(result.is_ok());
}

#[test]
fn test_market_regime_is_volatile() {
    let state = StateBuilder::new("AAPL")
        .with_regime(MarketRegime::Volatile)
        .build();
    
    assert!(conditions::is_volatile(&state));
    assert!(!conditions::is_trending(&state));
    assert!(!conditions::is_range_bound(&state));
}

#[test]
fn test_has_action_condition() {
    let with_action = SharedState {
        action: Some(investor_os::langgraph::state::TradingAction::Buy),
        ..SharedState::new("AAPL")
    };
    
    let without_action = SharedState::new("AAPL");
    
    assert!(conditions::has_action(&with_action));
    assert!(!conditions::has_action(&without_action));
}

#[test]
fn test_no_errors_condition() {
    let clean_state = SharedState::new("AAPL");
    
    let mut error_state = SharedState::new("AAPL");
    error_state.add_error("node", "error");
    
    assert!(conditions::no_errors(&clean_state));
    assert!(!conditions::no_errors(&error_state));
}
