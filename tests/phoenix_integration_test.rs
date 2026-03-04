//! End-to-End Integration Tests — Phoenix + LangGraph + Temporal
//!
//! Sprint 5-6: Full Trading Pipeline Tests

use investor_os::langgraph::{
    nodes::{Node, NodeOutput},
    MarketRegime, StateBuilder, TradingAction,
};
use investor_os::phoenix::{
    graph_integration::{DetectRegimeNode, MakeDecisionNode, PhoenixGraphEngine, RiskCheckNode},
    PhoenixConfig,
};
use investor_os::signals::{QualityScore, TickerSignals};
use rust_decimal::Decimal;

// ==================== Phoenix + LangGraph Tests ====================

#[tokio::test]
async fn test_phoenix_graph_engine_creation() {
    let config = PhoenixConfig::default();
    let engine = PhoenixGraphEngine::new(config);

    // Engine should be created successfully
    assert_eq!(engine.config.currency, "EUR");
}

#[tokio::test]
async fn test_detect_regime_node_trending() {
    let node = DetectRegimeNode;
    let state = StateBuilder::new("AAPL")
        .with_regime(MarketRegime::Trending)
        .build();

    let mut state = state;
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
async fn test_risk_check_passes_with_high_cq() {
    let node = RiskCheckNode::new(0.7);
    let state = StateBuilder::new("AAPL")
        .with_quality_score(0.8)
        .with_insider_score(0.8)
        .build();

    let mut state = state;
    // Set all CQ components
    state.sentiment_score = Some(0.8);
    state.regime_fit = Some(0.8);
    state.breakout_score = Some(0.8);
    state.atr_trend = Some(0.8);
    state.calculate_cq(); // Should be high

    let result = node.execute(state).await.unwrap();

    match result {
        NodeOutput::Continue(new_state) => {
            assert!(new_state.risk_approved);
            assert!(new_state.position_size.is_some());
            // Position size should be calculated
            let size = new_state.position_size.unwrap();
            assert!(size > Decimal::ZERO);
        }
        _ => panic!("Expected Continue"),
    }
}

#[tokio::test]
async fn test_risk_check_fails_with_low_cq() {
    let node = RiskCheckNode::new(0.7);
    let mut state = SharedState::new("AAPL");
    state.conviction_quotient = Some(0.5); // Below threshold

    let result = node.execute(state).await.unwrap();

    match result {
        NodeOutput::Continue(new_state) => {
            assert!(!new_state.risk_approved);
            assert!(new_state.position_size.is_none());
        }
        _ => panic!("Expected Continue"),
    }
}

#[tokio::test]
async fn test_make_decision_buy() {
    let node = MakeDecisionNode;
    let mut state = SharedState::new("AAPL");
    state.conviction_quotient = Some(0.85); // High CQ = Buy

    let result = node.execute(state).await.unwrap();

    match result {
        NodeOutput::End(final_state) => {
            assert_eq!(final_state.action, Some(TradingAction::Buy));
            assert_eq!(final_state.confidence, Some(0.85));
            assert!(matches!(
                final_state.execution_status,
                investor_os::langgraph::state::ExecutionStatus::Completed
            ));
        }
        _ => panic!("Expected End"),
    }
}

#[tokio::test]
async fn test_make_decision_hold() {
    let node = MakeDecisionNode;
    let mut state = SharedState::new("AAPL");
    state.conviction_quotient = Some(0.5); // Medium CQ = Hold

    let result = node.execute(state).await.unwrap();

    match result {
        NodeOutput::End(final_state) => {
            assert_eq!(final_state.action, Some(TradingAction::Hold));
        }
        _ => panic!("Expected End"),
    }
}

#[tokio::test]
async fn test_make_decision_sell() {
    let node = MakeDecisionNode;
    let mut state = SharedState::new("AAPL");
    state.conviction_quotient = Some(0.2); // Low CQ = Sell

    let result = node.execute(state).await.unwrap();

    match result {
        NodeOutput::End(final_state) => {
            assert_eq!(final_state.action, Some(TradingAction::Sell));
        }
        _ => panic!("Expected End"),
    }
}

// ==================== End-to-End Trading Flow ====================

#[tokio::test]
async fn test_full_trading_decision_flow_high_cq() {
    let config = PhoenixConfig::default();
    let engine = PhoenixGraphEngine::new(config);

    // Create high-quality signals
    let signals = TickerSignals {
        quality_score: QualityScore(80),
        insider_score: QualityScore(80),
        sentiment_score: QualityScore(80),
        regime_fit: QualityScore(80),
        breakout_score: 0.8,
        atr_trend: 0.8,
        ..Default::default()
    };

    let decision = engine
        .generate_decision("AAPL", signals, Decimal::from(150))
        .await;

    assert!(decision.is_ok());
    let decision = decision.unwrap();

    // High CQ should result in Buy action
    assert!(matches!(decision.action, investor_os::phoenix::Action::Buy));
    assert!(decision.confidence > 0.7);
    assert!(!decision.rationale.is_empty());
}

#[tokio::test]
async fn test_full_trading_decision_flow_low_cq() {
    let config = PhoenixConfig::default();
    let engine = PhoenixGraphEngine::new(config);

    // Create low-quality signals
    let signals = TickerSignals {
        quality_score: QualityScore(30),
        insider_score: QualityScore(30),
        sentiment_score: QualityScore(30),
        regime_fit: QualityScore(30),
        breakout_score: 0.2,
        atr_trend: 0.2,
        ..Default::default()
    };

    let decision = engine
        .generate_decision("AAPL", signals, Decimal::from(150))
        .await;

    // Low CQ might fail risk check or result in Sell/Hold
    // The graph should complete either way
    match decision {
        Ok(decision) => {
            // If we got a decision, it should be low confidence
            assert!(
                decision.confidence < 0.5
                    || matches!(decision.action, investor_os::phoenix::Action::Sell)
            );
        }
        Err(_) => {
            // Or risk check failed, which is also valid
        }
    }
}

// ==================== Integration with Signals ====================

use investor_os::langgraph::state::SharedState;

#[test]
fn test_ticker_signals_to_state_conversion() {
    let signals = TickerSignals {
        quality_score: QualityScore(75),
        insider_score: QualityScore(60),
        sentiment_score: QualityScore(80),
        regime_fit: QualityScore(70),
        breakout_score: 0.75,
        atr_trend: 0.6,
        ..Default::default()
    };

    let state = StateBuilder::new("TSLA")
        .with_quality_score(0.75)
        .with_insider_score(0.60)
        .build();

    let mut state = state;
    state.sentiment_score = Some(0.80);
    state.regime_fit = Some(0.70);
    state.breakout_score = Some(0.75);
    state.atr_trend = Some(0.60);

    // Calculate CQ
    let cq = state.calculate_cq();
    assert!(cq.is_some());

    // Expected: 0.75*0.20 + 0.60*0.20 + 0.80*0.15 + 0.70*0.20 + 0.75*0.15 + 0.60*0.10
    // = 0.15 + 0.12 + 0.12 + 0.14 + 0.1125 + 0.06 = 0.7025
    assert!((cq.unwrap() - 0.7025).abs() < 0.001);
}

// ==================== State Management Tests ====================

#[tokio::test]
async fn test_state_persists_through_nodes() {
    let node1 = DetectRegimeNode;
    let node2 = RiskCheckNode::new(0.5);

    let state = StateBuilder::new("MSFT")
        .with_price(Decimal::from(300))
        .build();

    let mut state = state;
    state.breakout_score = Some(0.9);
    state.conviction_quotient = Some(0.8);

    // First node
    let result1 = node1.execute(state).await.unwrap();
    state = match result1 {
        NodeOutput::Continue(s) => s,
        _ => panic!("Expected Continue"),
    };

    // Verify state persisted
    assert_eq!(state.ticker, "MSFT");
    assert!(state.current_price.is_some());
    assert!(matches!(state.market_regime, MarketRegime::Trending));

    // Second node
    let result2 = node2.execute(state).await.unwrap();
    state = match result2 {
        NodeOutput::Continue(s) => s,
        _ => panic!("Expected Continue"),
    };

    // State should still have all data
    assert_eq!(state.ticker, "MSFT");
    assert!(state.risk_approved);
}
