//! Specialized Trading Agents
//!
//! This module contains concrete agent implementations:
//! - MarketAnalyst: Technical and fundamental analysis
//! - RiskAssessor: Position sizing and risk evaluation
//! - ExecutionSpecialist: Order routing and execution
//! - Learner: Strategy optimization
//! - SentimentReader: News and social media analysis

pub mod market_analyst;
pub mod risk_assessor;
pub mod execution_specialist;
pub mod learner;
pub mod sentiment_reader;

pub use market_analyst::MarketAnalystAgent;
pub use risk_assessor::RiskAssessorAgent;
pub use execution_specialist::ExecutionSpecialistAgent;
pub use learner::LearnerAgent;
pub use sentiment_reader::SentimentReaderAgent;

pub use super::{
    Agent, AgentConfig, AgentError, AgentId, AgentMessage, AgentRole, AgentStatus,
    MarketAnalysis, RiskAssessment, ExecutionPlan, LearningUpdate, SentimentAnalysis,
    Task, TaskOutput, TaskResult, TaskStatus, TrendDirection, RecommendedAction,
    RiskLevel, Sentiment, VoteData, VoteChoice, Priority, MessageType, MessagePayload,
    ObservationData, WarningData, RequestData, TradeResult, PositionInfo, OrderInfo,
    ExecutionTiming, OrderSlice, PositionSide, OrderSide, OrderType,
};
