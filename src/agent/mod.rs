//! Multi-Agent System for Trading
//!
//! Specialized agents collaborate to make trading decisions:
//! - MarketAnalyst: Technical/fundamental analysis
//! - RiskAssessor: Position sizing, risk limits
//! - ExecutionSpecialist: Order routing, timing
//! - Learner: Strategy optimization from results
//! - SentimentReader: News/social media analysis

pub mod coordinator;
pub mod communication;
pub mod consensus;
pub mod registry;
pub mod supervisor;
pub mod agents;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use thiserror::Error;
use uuid::Uuid;

/// Unique agent identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentId(pub String);

impl AgentId {
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }
    
    pub fn from_string(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

impl Default for AgentId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for AgentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Agent roles/specializations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AgentRole {
    /// Market analysis (technical/fundamental)
    MarketAnalyst,
    /// Risk assessment and position sizing
    RiskAssessor,
    /// Order execution and routing
    ExecutionSpecialist,
    /// Learning from past trades
    Learner,
    /// Sentiment analysis (news/social)
    SentimentReader,
    /// Orchestrates other agents
    Coordinator,
}

impl fmt::Display for AgentRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AgentRole::MarketAnalyst => write!(f, "MarketAnalyst"),
            AgentRole::RiskAssessor => write!(f, "RiskAssessor"),
            AgentRole::ExecutionSpecialist => write!(f, "ExecutionSpecialist"),
            AgentRole::Learner => write!(f, "Learner"),
            AgentRole::SentimentReader => write!(f, "SentimentReader"),
            AgentRole::Coordinator => write!(f, "Coordinator"),
        }
    }
}

/// Agent status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentStatus {
    Initializing,
    Active,
    Busy,
    Paused,
    Error,
    Shutdown,
}

/// Agent errors
#[derive(Error, Debug, Clone)]
pub enum AgentError {
    #[error("Agent not found: {0}")]
    AgentNotFound(AgentId),
    
    #[error("Agent already registered: {0}")]
    AlreadyRegistered(AgentId),
    
    #[error("Communication error: {0}")]
    CommunicationError(String),
    
    #[error("Task timeout: {0}")]
    TaskTimeout(TaskId),
    
    #[error("Consensus not reached: {0}")]
    ConsensusNotReached(String),
    
    #[error("Agent error: {0}")]
    AgentError(String),
    
    #[error("Invalid message: {0}")]
    InvalidMessage(String),
}

/// Task identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TaskId(pub String);

impl TaskId {
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }
}

impl Default for TaskId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for TaskId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Task for an agent to process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: TaskId,
    pub task_type: TaskType,
    pub payload: TaskPayload,
    pub deadline: Option<DateTime<Utc>>,
    pub priority: Priority,
}

/// Task types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskType {
    AnalyzeMarket { symbol: String },
    AssessRisk { position: PositionInfo },
    OptimizeExecution { order: OrderInfo },
    LearnFromTrade { trade: TradeResult },
    AnalyzeSentiment { symbol: String, sources: Vec<String> },
}

/// Task payload (dynamic data)
pub type TaskPayload = serde_json::Value;

/// Position information for risk assessment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionInfo {
    pub symbol: String,
    pub quantity: Decimal,
    pub entry_price: Decimal,
    pub current_price: Decimal,
    pub side: PositionSide,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum PositionSide {
    Long,
    Short,
}

/// Order information for execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderInfo {
    pub symbol: String,
    pub quantity: Decimal,
    pub side: OrderSide,
    pub order_type: OrderType,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum OrderSide {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum OrderType {
    Market,
    Limit(Decimal),
}

/// Trade result for learning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeResult {
    pub symbol: String,
    pub entry_price: Decimal,
    pub exit_price: Decimal,
    pub quantity: Decimal,
    pub pnl: Decimal,
    pub duration_secs: u64,
    pub exit_reason: String,
}

/// Task processing result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub task_id: TaskId,
    pub agent_id: AgentId,
    pub status: TaskStatus,
    pub output: TaskOutput,
    pub execution_time_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    Success,
    PartialSuccess,
    Failed,
    Cancelled,
}

/// Task output data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskOutput {
    MarketAnalysis(MarketAnalysis),
    RiskAssessment(RiskAssessment),
    ExecutionPlan(ExecutionPlan),
    LearningUpdate(LearningUpdate),
    SentimentAnalysis(SentimentAnalysis),
    Error(String),
}

/// Market analysis output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketAnalysis {
    pub symbol: String,
    pub trend: TrendDirection,
    pub confidence: f64,
    pub support_levels: Vec<Decimal>,
    pub resistance_levels: Vec<Decimal>,
    pub recommended_action: RecommendedAction,
    pub rationale: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrendDirection {
    StrongUp,
    Up,
    Sideways,
    Down,
    StrongDown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecommendedAction {
    StrongBuy,
    Buy,
    Hold,
    Sell,
    StrongSell,
}

/// Risk assessment output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAssessment {
    pub max_position_size: Decimal,
    pub var_95: Decimal,
    pub risk_level: RiskLevel,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

/// Execution plan output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPlan {
    pub optimal_venue: String,
    pub timing: ExecutionTiming,
    pub order_splitting: Vec<OrderSlice>,
    pub expected_slippage_bps: f64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ExecutionTiming {
    Immediate,
    WaitFor(Duration),
    TWAP { duration: Duration, slices: u32 },
}

use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderSlice {
    pub quantity: Decimal,
    pub delay_ms: u64,
}

/// Learning update output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningUpdate {
    pub strategy_adjustments: HashMap<String, f64>,
    pub insights: Vec<String>,
    pub performance_delta: f64,
}

/// Sentiment analysis output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentimentAnalysis {
    pub overall_sentiment: Sentiment,
    pub news_sentiment: f64,
    pub social_sentiment: f64,
    pub key_topics: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Sentiment {
    VeryBullish,
    Bullish,
    Neutral,
    Bearish,
    VeryBearish,
}

/// Message priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[derive(Default)]
pub enum Priority {
    Low = 0,
    #[default]
    Normal = 1,
    High = 2,
    Critical = 3,
}


/// Base trait for all agents
#[async_trait]
pub trait Agent: Send + Sync {
    /// Get agent ID
    fn id(&self) -> &AgentId;
    
    /// Get agent role
    fn role(&self) -> AgentRole;
    
    /// Get agent status
    fn status(&self) -> AgentStatus;
    
    /// Process a task
    async fn process(&mut self, task: Task) -> Result<TaskResult, AgentError>;
    
    /// Handle incoming message
    async fn on_message(&mut self, msg: AgentMessage) -> Result<(), AgentError>;
    
    /// Pause agent
    async fn pause(&mut self);
    
    /// Resume agent
    async fn resume(&mut self);
    
    /// Shutdown agent
    async fn shutdown(&mut self);
}

/// Agent trait object type
pub type AgentBox = Box<dyn Agent>;

/// Message types for inter-agent communication
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MessageType {
    Observation,
    Prediction,
    Warning,
    Request,      // Request for information/action
    Response,     // Response to a request
    Vote,         // Consensus vote
    Broadcast,    // General broadcast
}

/// Message payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessagePayload {
    Observation(ObservationData),
    Prediction(PredictionData),
    Warning(WarningData),
    Request(RequestData),
    Response(ResponseData),
    Vote(VoteData),
    Broadcast(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservationData {
    pub symbol: String,
    pub observation_type: String,
    pub value: f64,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictionData {
    pub symbol: String,
    pub prediction_type: String,
    pub predicted_value: f64,
    pub confidence: f64,
    pub timeframe: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WarningData {
    pub severity: WarningSeverity,
    pub category: String,
    pub message: String,
    pub affected_symbols: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WarningSeverity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestData {
    pub request_id: String,
    pub request_type: String,
    pub parameters: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseData {
    pub request_id: String,
    pub success: bool,
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoteData {
    pub proposal_id: String,
    pub vote: VoteChoice,
    pub weight: f64,
    pub rationale: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VoteChoice {
    For,
    Against,
    Abstain,
}

/// Inter-agent message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    pub id: String,
    pub from: AgentId,
    /// None = broadcast to all
    pub to: Option<AgentId>,
    pub msg_type: MessageType,
    pub payload: MessagePayload,
    pub timestamp: DateTime<Utc>,
    pub priority: Priority,
    pub reply_to: Option<String>, // Message ID to reply to
}

impl AgentMessage {
    pub fn new(
        from: AgentId,
        to: Option<AgentId>,
        msg_type: MessageType,
        payload: MessagePayload,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            from,
            to,
            msg_type,
            payload,
            timestamp: Utc::now(),
            priority: Priority::Normal,
            reply_to: None,
        }
    }
    
    pub fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }
    
    pub fn broadcast(from: AgentId, msg_type: MessageType, payload: MessagePayload) -> Self {
        Self::new(from, None, msg_type, payload)
    }
}

/// Agent configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub id: AgentId,
    pub role: AgentRole,
    pub name: String,
    pub description: String,
    pub max_concurrent_tasks: usize,
    pub task_timeout_ms: u64,
    pub enabled: bool,
}

impl AgentConfig {
    pub fn new(role: AgentRole, name: impl Into<String>) -> Self {
        Self {
            id: AgentId::new(),
            role,
            name: name.into(),
            description: String::new(),
            max_concurrent_tasks: 5,
            task_timeout_ms: 30000,
            enabled: true,
        }
    }
    
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_id_creation() {
        let id1 = AgentId::new();
        let id2 = AgentId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_agent_role_display() {
        assert_eq!(AgentRole::MarketAnalyst.to_string(), "MarketAnalyst");
        assert_eq!(AgentRole::RiskAssessor.to_string(), "RiskAssessor");
    }

    #[test]
    fn test_priority_ordering() {
        assert!(Priority::Critical > Priority::High);
        assert!(Priority::High > Priority::Normal);
        assert!(Priority::Normal > Priority::Low);
    }

    #[test]
    fn test_agent_message_creation() {
        let from = AgentId::from_string("agent1");
        let to = AgentId::from_string("agent2");
        
        let msg = AgentMessage::new(
            from,
            Some(to),
            MessageType::Observation,
            MessagePayload::Broadcast("test".to_string()),
        );
        
        assert_eq!(msg.from.0, "agent1");
        assert_eq!(msg.to.unwrap().0, "agent2");
    }

    #[test]
    fn test_task_output_variants() {
        let analysis = MarketAnalysis {
            symbol: "AAPL".to_string(),
            trend: TrendDirection::Up,
            confidence: 0.85,
            support_levels: vec![Decimal::from(100)],
            resistance_levels: vec![Decimal::from(110)],
            recommended_action: RecommendedAction::Buy,
            rationale: "Strong momentum".to_string(),
        };
        
        let output = TaskOutput::MarketAnalysis(analysis);
        match output {
            TaskOutput::MarketAnalysis(a) => assert_eq!(a.symbol, "AAPL"),
            _ => panic!("Wrong variant"),
        }
    }
}
