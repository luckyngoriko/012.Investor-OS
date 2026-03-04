//! Sprint 25: Multi-Agent Systems - Golden Path Tests
//!
//! Tests for:
//! - Agent registration and lifecycle
//! - Inter-agent communication
//! - Consensus decision making
//! - Specialized agent functionality

use investor_os::agent::agents::*;
use investor_os::agent::communication::CommunicationHub;
use investor_os::agent::consensus::{
    ConsensusEngine, ConsensusThreshold, Proposal, TradingDecision, WeightedVote,
};
use investor_os::agent::*;
use rust_decimal::Decimal;
use std::time::Duration;

/// Test 1: Agent registration and deregistration
#[tokio::test]
async fn test_agent_lifecycle() {
    let coordinator = coordinator::AgentCoordinator::new(coordinator::CoordinatorConfig::default());

    // Register a market analyst agent
    let config = AgentConfig::new(AgentRole::MarketAnalyst, "Test Analyst")
        .with_description("Test market analyst");

    let agent = MarketAnalystAgent::new(config.clone());
    let agent_id = coordinator.register_agent(config, agent).await.unwrap();

    // Verify agent is registered
    assert_eq!(coordinator.get_all_agents().await.len(), 1);

    // Deregister agent
    coordinator.deregister_agent(&agent_id).await.unwrap();
    assert_eq!(coordinator.get_all_agents().await.len(), 0);
}

/// Test 2: Multiple specialized agents
#[tokio::test]
async fn test_specialized_agents() {
    let coordinator = coordinator::AgentCoordinator::new(coordinator::CoordinatorConfig::default());

    // Register agents of different types
    let analyst_config = AgentConfig::new(AgentRole::MarketAnalyst, "Analyst");
    let analyst = MarketAnalystAgent::new(analyst_config.clone());
    let analyst_id = coordinator
        .register_agent(analyst_config, analyst)
        .await
        .unwrap();

    let risk_config = AgentConfig::new(AgentRole::RiskAssessor, "Risk Manager");
    let risk_agent = RiskAssessorAgent::new(risk_config.clone());
    let risk_id = coordinator
        .register_agent(risk_config, risk_agent)
        .await
        .unwrap();

    let exec_config = AgentConfig::new(AgentRole::ExecutionSpecialist, "Execution");
    let exec_agent = ExecutionSpecialistAgent::new(exec_config.clone());
    let exec_id = coordinator
        .register_agent(exec_config, exec_agent)
        .await
        .unwrap();

    // Verify all agents are registered
    assert_eq!(coordinator.get_all_agents().await.len(), 3);

    // Verify agents by role
    let analysts = coordinator
        .get_agents_by_role(AgentRole::MarketAnalyst)
        .await;
    assert_eq!(analysts.len(), 1);
    assert!(analysts.contains(&analyst_id));

    let risk = coordinator
        .get_agents_by_role(AgentRole::RiskAssessor)
        .await;
    assert_eq!(risk.len(), 1);

    // Cleanup
    coordinator.deregister_agent(&analyst_id).await.unwrap();
    coordinator.deregister_agent(&risk_id).await.unwrap();
    coordinator.deregister_agent(&exec_id).await.unwrap();
}

/// Test 3: Market analysis task
#[tokio::test]
async fn test_market_analysis_task() {
    let coordinator = coordinator::AgentCoordinator::new(coordinator::CoordinatorConfig::default());

    let config = AgentConfig::new(AgentRole::MarketAnalyst, "Analyst");
    let agent = MarketAnalystAgent::new(config.clone());
    let agent_id = coordinator.register_agent(config, agent).await.unwrap();

    // Send analysis task
    let task = Task {
        id: TaskId::new(),
        task_type: TaskType::AnalyzeMarket {
            symbol: "AAPL".to_string(),
        },
        payload: serde_json::Value::Null,
        deadline: None,
        priority: Priority::Normal,
    };

    let result = coordinator.send_task(&agent_id, task).await.unwrap();

    assert!(matches!(result.status, TaskStatus::Success));
    match result.output {
        TaskOutput::MarketAnalysis(analysis) => {
            assert_eq!(analysis.symbol, "AAPL");
            assert!(analysis.confidence > 0.0);
        }
        _ => panic!("Expected MarketAnalysis output"),
    }

    // Cleanup
    coordinator.deregister_agent(&agent_id).await.unwrap();
}

/// Test 4: Risk assessment task
#[tokio::test]
async fn test_risk_assessment_task() {
    let coordinator = coordinator::AgentCoordinator::new(coordinator::CoordinatorConfig::default());

    let config = AgentConfig::new(AgentRole::RiskAssessor, "Risk Agent");
    let agent = RiskAssessorAgent::new(config.clone());
    let agent_id = coordinator.register_agent(config, agent).await.unwrap();

    // Send risk assessment task
    let position = PositionInfo {
        symbol: "TSLA".to_string(),
        quantity: Decimal::from(100),
        entry_price: Decimal::try_from(200.0).unwrap(),
        current_price: Decimal::try_from(180.0).unwrap(), // 10% loss
        side: PositionSide::Long,
    };

    let task = Task {
        id: TaskId::new(),
        task_type: TaskType::AssessRisk { position },
        payload: serde_json::Value::Null,
        deadline: None,
        priority: Priority::High,
    };

    let result = coordinator.send_task(&agent_id, task).await.unwrap();

    assert!(matches!(result.status, TaskStatus::Success));
    match result.output {
        TaskOutput::RiskAssessment(assessment) => {
            assert!(!assessment.var_95.is_zero());
            assert!(!assessment.warnings.is_empty()); // Should warn about loss
        }
        _ => panic!("Expected RiskAssessment output"),
    }

    // Cleanup
    coordinator.deregister_agent(&agent_id).await.unwrap();
}

/// Test 5: Consensus simple majority
#[test]
fn test_simple_majority_consensus() {
    let mut engine = ConsensusEngine::new();

    let voters = vec![
        AgentId::from_string("agent1"),
        AgentId::from_string("agent2"),
        AgentId::from_string("agent3"),
        AgentId::from_string("agent4"),
        AgentId::from_string("agent5"),
    ];

    let proposal = Proposal::new(
        "Buy AAPL",
        "Proposal to buy 100 shares of AAPL",
        TradingDecision::Buy {
            symbol: "AAPL".to_string(),
            quantity: Decimal::from(100),
            max_price: None,
        },
        Duration::from_secs(60),
        ConsensusThreshold::SimpleMajority,
    );

    let proposal_id = engine.create_proposal(proposal, voters.clone());

    // 3 out of 5 vote for (60% - meets simple majority)
    engine
        .vote(
            &proposal_id,
            WeightedVote::new(voters[0].clone(), VoteChoice::For, 1.0, 0.9, "Good setup"),
        )
        .unwrap();

    engine
        .vote(
            &proposal_id,
            WeightedVote::new(voters[1].clone(), VoteChoice::For, 1.0, 0.8, "Agree"),
        )
        .unwrap();

    engine
        .vote(
            &proposal_id,
            WeightedVote::new(voters[2].clone(), VoteChoice::For, 1.0, 0.85, "Buy signal"),
        )
        .unwrap();

    engine
        .vote(
            &proposal_id,
            WeightedVote::new(
                voters[3].clone(),
                VoteChoice::Against,
                1.0,
                0.5,
                "Too risky",
            ),
        )
        .unwrap();

    engine
        .vote(
            &proposal_id,
            WeightedVote::new(voters[4].clone(), VoteChoice::Against, 1.0, 0.6, "Wait"),
        )
        .unwrap();

    let result = engine.finalize(&proposal_id);

    match result {
        consensus::ConsensusResult::Approved(decision) => {
            assert_eq!(decision.votes_for.len(), 3);
            assert_eq!(decision.votes_against.len(), 2);
            assert!(decision.agreement_level > 0.5);
        }
        _ => panic!("Expected Approved consensus"),
    }
}

/// Test 6: Consensus weighted voting
#[test]
fn test_weighted_consensus() {
    let mut engine = ConsensusEngine::new();

    let voters = vec![
        AgentId::from_string("expert"),
        AgentId::from_string("novice1"),
        AgentId::from_string("novice2"),
    ];

    let proposal = Proposal::new(
        "Trade Decision",
        "Should we enter this trade?",
        TradingDecision::Buy {
            symbol: "BTC".to_string(),
            quantity: Decimal::from(1),
            max_price: None,
        },
        Duration::from_secs(60),
        ConsensusThreshold::SimpleMajority,
    );

    let proposal_id = engine.create_proposal(proposal, voters.clone());

    // Expert votes against with weight 3.0
    engine
        .vote(
            &proposal_id,
            WeightedVote::new(
                voters[0].clone(),
                VoteChoice::Against,
                3.0,
                0.95,
                "Strong rejection",
            ),
        )
        .unwrap();

    // Novices vote for with weight 1.0 each
    engine
        .vote(
            &proposal_id,
            WeightedVote::new(voters[1].clone(), VoteChoice::For, 1.0, 0.5, "Looks good"),
        )
        .unwrap();

    engine
        .vote(
            &proposal_id,
            WeightedVote::new(voters[2].clone(), VoteChoice::For, 1.0, 0.5, "I agree"),
        )
        .unwrap();

    let result = engine.finalize(&proposal_id);

    // Should be rejected: 3.0 against vs 2.0 for
    match result {
        consensus::ConsensusResult::Rejected(_) => {}
        _ => panic!("Expected Rejected consensus due to expert weight"),
    }
}

/// Test 7: Communication hub message passing
#[tokio::test]
async fn test_inter_agent_communication() {
    let hub = CommunicationHub::new();

    let agent1 = AgentId::from_string("agent1");
    let agent2 = AgentId::from_string("agent2");

    let _rx1 = hub.register_agent(agent1.clone()).await;
    let mut rx2 = hub.register_agent(agent2.clone()).await;

    // Send message from agent1 to agent2
    let msg = AgentMessage::new(
        agent1.clone(),
        Some(agent2.clone()),
        MessageType::Observation,
        MessagePayload::Observation(ObservationData {
            symbol: "AAPL".to_string(),
            observation_type: "price_spike".to_string(),
            value: 5.0,
            metadata: [("source".to_string(), "market".to_string())].into(),
        }),
    );

    hub.send_to(msg.clone(), &agent2).await.unwrap();

    // Verify agent2 receives the message
    let received = rx2.recv().await;
    assert!(received.is_some());
    assert_eq!(received.unwrap().from, agent1);
}

/// Test 8: Sentiment analysis agent
#[tokio::test]
async fn test_sentiment_reader_agent() {
    let coordinator = coordinator::AgentCoordinator::new(coordinator::CoordinatorConfig::default());

    let config = AgentConfig::new(AgentRole::SentimentReader, "Sentiment Agent");
    let agent = SentimentReaderAgent::new(config.clone());
    let agent_id = coordinator.register_agent(config, agent).await.unwrap();

    // Send sentiment analysis task
    let task = Task {
        id: TaskId::new(),
        task_type: TaskType::AnalyzeSentiment {
            symbol: "TSLA".to_string(),
            sources: vec!["news".to_string(), "twitter".to_string()],
        },
        payload: serde_json::Value::Null,
        deadline: None,
        priority: Priority::Normal,
    };

    let result = coordinator.send_task(&agent_id, task).await.unwrap();

    assert!(matches!(result.status, TaskStatus::Success));
    match result.output {
        TaskOutput::SentimentAnalysis(sentiment) => {
            assert!(!sentiment.key_topics.is_empty());
            assert!(sentiment.news_sentiment >= -1.0 && sentiment.news_sentiment <= 1.0);
        }
        _ => panic!("Expected SentimentAnalysis output"),
    }

    // Cleanup
    coordinator.deregister_agent(&agent_id).await.unwrap();
}

/// Test 9: Learner agent from trade
#[tokio::test]
async fn test_learner_agent() {
    let coordinator = coordinator::AgentCoordinator::new(coordinator::CoordinatorConfig::default());

    let config = AgentConfig::new(AgentRole::Learner, "Learning Agent");
    let agent = LearnerAgent::new(config.clone());
    let agent_id = coordinator.register_agent(config, agent).await.unwrap();

    // Send learning task
    let trade = TradeResult {
        symbol: "AAPL".to_string(),
        entry_price: Decimal::try_from(150.0).unwrap(),
        exit_price: Decimal::try_from(160.0).unwrap(),
        quantity: Decimal::from(100),
        pnl: Decimal::try_from(1000.0).unwrap(),
        duration_secs: 3600,
        exit_reason: "take_profit".to_string(),
    };

    let task = Task {
        id: TaskId::new(),
        task_type: TaskType::LearnFromTrade { trade },
        payload: serde_json::Value::Null,
        deadline: None,
        priority: Priority::Normal,
    };

    let result = coordinator.send_task(&agent_id, task).await.unwrap();

    assert!(matches!(result.status, TaskStatus::Success));
    match result.output {
        TaskOutput::LearningUpdate(update) => {
            assert!(!update.insights.is_empty());
        }
        _ => panic!("Expected LearningUpdate output"),
    }

    // Cleanup
    coordinator.deregister_agent(&agent_id).await.unwrap();
}

/// Test 10: Execution planning
#[tokio::test]
async fn test_execution_specialist() {
    let coordinator = coordinator::AgentCoordinator::new(coordinator::CoordinatorConfig::default());

    let config = AgentConfig::new(AgentRole::ExecutionSpecialist, "Execution Agent");
    let agent = ExecutionSpecialistAgent::new(config.clone());
    let agent_id = coordinator.register_agent(config, agent).await.unwrap();

    // Send execution optimization task
    let order = OrderInfo {
        symbol: "BTC".to_string(),
        quantity: Decimal::try_from(50000.0).unwrap(),
        side: OrderSide::Buy,
        order_type: OrderType::Market,
    };

    let task = Task {
        id: TaskId::new(),
        task_type: TaskType::OptimizeExecution { order },
        payload: serde_json::Value::Null,
        deadline: None,
        priority: Priority::High,
    };

    let result = coordinator.send_task(&agent_id, task).await.unwrap();

    assert!(matches!(result.status, TaskStatus::Success));
    match result.output {
        TaskOutput::ExecutionPlan(plan) => {
            assert!(!plan.optimal_venue.is_empty());
            assert!(plan.expected_slippage_bps > 0.0);
        }
        _ => panic!("Expected ExecutionPlan output"),
    }

    // Cleanup
    coordinator.deregister_agent(&agent_id).await.unwrap();
}
