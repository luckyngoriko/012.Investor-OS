//! Integration tests for LangGraph module
//!
//! Sprint 3: LangGraph Core Tests

use investor_os::langgraph::{
    edges::conditions,
    graph::{Graph, GraphBuilder},
    nodes::{CQCalculationNode, Node, NodeOutput, StartNode},
    state::{ExecutionStatus, MarketRegime, SharedState, StateBuilder, TradingAction},
};

#[test]
fn test_shared_state_builder() {
    let state = StateBuilder::new("AAPL")
        .with_price(rust_decimal::Decimal::from(150))
        .with_regime(MarketRegime::Trending)
        .with_quality_score(0.8)
        .with_insider_score(0.7)
        .build();

    assert_eq!(state.ticker, "AAPL");
    assert_eq!(state.current_price, Some(rust_decimal::Decimal::from(150)));
    assert!(matches!(state.market_regime, MarketRegime::Trending));
    assert_eq!(state.quality_score, Some(0.8));
    assert_eq!(state.insider_score, Some(0.7));
}

#[test]
fn test_cq_calculation() {
    let mut state = StateBuilder::new("AAPL")
        .with_quality_score(0.8)
        .with_insider_score(0.7)
        .with_sentiment_score(0.6)
        .with_regime(MarketRegime::Trending)
        .build();

    // Set the scores directly
    state.regime_fit = Some(0.9);
    state.breakout_score = Some(0.75);
    state.atr_trend = Some(0.5);

    // Calculate CQ
    let cq = state.calculate_cq();

    assert!(cq.is_some());
    let cq_value = cq.unwrap();
    assert!(cq_value >= 0.0 && cq_value <= 1.0);

    // Expected: 0.8*0.20 + 0.7*0.20 + 0.6*0.15 + 0.9*0.20 + 0.75*0.15 + 0.5*0.10
    // = 0.16 + 0.14 + 0.09 + 0.18 + 0.1125 + 0.05 = 0.7325
    assert!((cq_value - 0.7325).abs() < 0.001);
}

#[test]
fn test_market_regime_conditions() {
    let trending_state = StateBuilder::new("AAPL")
        .with_regime(MarketRegime::Trending)
        .build();

    let range_state = StateBuilder::new("AAPL")
        .with_regime(MarketRegime::RangeBound)
        .build();

    assert!(conditions::is_trending(&trending_state));
    assert!(!conditions::is_trending(&range_state));

    assert!(!conditions::is_range_bound(&trending_state));
    assert!(conditions::is_range_bound(&range_state));
}

#[test]
fn test_cq_condition() {
    let high_cq_state = SharedState {
        conviction_quotient: Some(0.75),
        ..SharedState::new("AAPL")
    };

    let low_cq_state = SharedState {
        conviction_quotient: Some(0.5),
        ..SharedState::new("AAPL")
    };

    let cq_above_70 = conditions::cq_above(0.7);

    assert!(cq_above_70(&high_cq_state));
    assert!(!cq_above_70(&low_cq_state));
}

#[test]
fn test_state_snapshot() {
    let state = SharedState::new("AAPL");
    let snapshot = state.snapshot();

    assert_eq!(snapshot.current_node, "start");
    assert!(snapshot.cq.is_none());
}

#[tokio::test]
async fn test_start_node() {
    let node = StartNode;
    let state = SharedState::new("AAPL");

    match node
        .execute(state)
        .await
        .expect("StartNode execute should succeed")
    {
        NodeOutput::Continue(new_state) => {
            assert_eq!(new_state.ticker, "AAPL");
        }
        _ => panic!("Expected Continue output from StartNode"),
    }
}

#[tokio::test]
async fn test_cq_calculation_node() {
    let node = CQCalculationNode;
    let mut state = StateBuilder::new("AAPL")
        .with_quality_score(0.8)
        .with_insider_score(0.7)
        .build();

    // Set remaining scores
    state.sentiment_score = Some(0.6);
    state.regime_fit = Some(0.9);
    state.breakout_score = Some(0.75);
    state.atr_trend = Some(0.5);

    match node
        .execute(state)
        .await
        .expect("CQCalculationNode execute should succeed")
    {
        NodeOutput::Continue(new_state) => {
            let cq = new_state
                .conviction_quotient
                .expect("conviction_quotient should be set after CQ calculation");
            assert!(
                (cq - 0.7325).abs() < 0.001,
                "CQ should be ~0.7325, got {cq}"
            );
        }
        _ => panic!("Expected Continue output from CQCalculationNode"),
    }
}

#[tokio::test]
async fn test_simple_graph_execution() {
    use async_trait::async_trait;
    use investor_os::langgraph::nodes::NodeError;

    // Create a simple test node
    struct TestTransformNode;

    #[async_trait]
    impl Node for TestTransformNode {
        fn name(&self) -> &str {
            "transform"
        }

        async fn execute(&self, mut state: SharedState) -> Result<NodeOutput, NodeError> {
            state
                .metadata
                .insert("transformed".to_string(), serde_json::json!(true));
            Ok(NodeOutput::Continue(state))
        }
    }

    struct TestEndNode;

    #[async_trait]
    impl Node for TestEndNode {
        fn name(&self) -> &str {
            "end"
        }

        async fn execute(&self, state: SharedState) -> Result<NodeOutput, NodeError> {
            Ok(NodeOutput::End(state))
        }
    }

    // Build graph
    let graph = GraphBuilder::new("test_graph")
        .add_node("start", StartNode)
        .add_node("transform", TestTransformNode)
        .add_node("end", TestEndNode)
        .add_edge("start", "transform")
        .add_edge("transform", "end")
        .set_start("start")
        .set_end("end")
        .build()
        .unwrap();

    let initial_state = SharedState::new("TEST");
    let final_state = graph
        .execute(initial_state)
        .await
        .expect("graph execution should complete successfully");
    // Verify we went through the transform node
    assert!(final_state.metadata.contains_key("transformed"));
    // Graph completed successfully (End node was reached)
    assert!(!final_state.node_history.is_empty());
}

#[test]
fn test_execution_status() {
    let pending = ExecutionStatus::Pending;
    let completed = ExecutionStatus::Completed;
    let failed = ExecutionStatus::Failed;

    // This is more of a compile-time check that the enum variants exist
    assert!(!matches!(pending, ExecutionStatus::Completed));
    assert!(matches!(completed, ExecutionStatus::Completed));
    assert!(matches!(failed, ExecutionStatus::Failed));
}

#[test]
fn test_trading_action_variants() {
    let buy = TradingAction::Buy;
    let sell = TradingAction::Sell;
    let hold = TradingAction::Hold;

    assert!(matches!(buy, TradingAction::Buy));
    assert!(matches!(sell, TradingAction::Sell));
    assert!(matches!(hold, TradingAction::Hold));
}
