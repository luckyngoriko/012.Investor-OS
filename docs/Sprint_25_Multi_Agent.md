# Sprint 25: Multi-Agent Systems

## Goal
Build a multi-agent orchestration system where specialized AI agents collaborate to make trading decisions, share knowledge, and reach consensus on complex market scenarios.

## User Stories

### Story 1: Agent Orchestration
**As a** system architect
**I want** a coordinator that manages multiple specialized agents
**So that** each agent can focus on its domain expertise

**Acceptance Criteria:**
- Agent registration and lifecycle management
- Task distribution to appropriate agents
- Agent health monitoring
- Dynamic agent scaling

### Story 2: Inter-Agent Communication
**As a** trader
**I want** agents to share insights and collaborate
**So that** decisions benefit from multiple perspectives

**Acceptance Criteria:**
- Publish-subscribe messaging between agents
- Structured message formats (observations, predictions, warnings)
- Message priority and routing
- Async communication with timeouts

### Story 3: Consensus Decision Making
**As a** risk manager
**I want** multiple agents to vote on important decisions
**So that** no single agent has unilateral control

**Acceptance Criteria:**
- Weighted voting based on agent expertise
- Configurable consensus thresholds (simple majority, 2/3, unanimous)
- Tie-breaking mechanisms
- Decision audit trails

### Story 4: Specialized Trading Agents
**As a** portfolio manager
**I want** domain-specific agents for different tasks
**So that** we have deep expertise in each area

**Acceptance Criteria:**
- MarketAnalysisAgent - technical/fundamental analysis
- RiskAssessmentAgent - position sizing, limits
- ExecutionAgent - order routing, timing
- LearningAgent - strategy optimization
- SentimentAgent - news/social analysis

## Technical Design

### New Components

```
src/agent/
├── mod.rs              # Core types and traits
├── coordinator.rs      # Agent orchestration
├── communication.rs    # Message passing
├── consensus.rs        # Voting mechanisms
├── registry.rs         # Agent discovery
├── supervisor.rs       # Health monitoring
└── agents/
    ├── mod.rs
    ├── market_analyst.rs
    ├── risk_assessor.rs
    ├── execution_specialist.rs
    ├── learner.rs
    └── sentiment_reader.rs
```

### Core Types

```rust
/// Base trait for all agents
#[async_trait]
pub trait Agent: Send + Sync {
    fn id(&self) -> &AgentId;
    fn role(&self) -> AgentRole;
    async fn process(&self, task: Task) -> Result<TaskResult, AgentError>;
    async fn on_message(&self, msg: AgentMessage) -> Result<(), AgentError>;
}

/// Agent roles
pub enum AgentRole {
    MarketAnalyst,
    RiskAssessor,
    ExecutionSpecialist,
    Learner,
    SentimentReader,
    Coordinator,
}

/// Inter-agent message
pub struct AgentMessage {
    pub from: AgentId,
    pub to: Option<AgentId>, // None = broadcast
    pub msg_type: MessageType,
    pub payload: MessagePayload,
    pub timestamp: DateTime<Utc>,
    pub priority: Priority,
}
```

### Consensus Algorithm

```rust
/// Consensus for trading decisions
pub struct ConsensusVote {
    pub agent_id: AgentId,
    pub decision: TradingDecision,
    pub confidence: f64,
    pub weight: f64, // Based on agent track record
}

pub struct ConsensusResult {
    pub decision: TradingDecision,
    pub agreement_level: f64, // 0.0 - 1.0
    pub votes_for: Vec<AgentId>,
    pub votes_against: Vec<AgentId>,
    pub abstentions: Vec<AgentId>,
}
```

## Test Plan

1. **Agent Lifecycle Tests**
   - Registration/deregistration
   - Health checks
   - Failure recovery

2. **Communication Tests**
   - Point-to-point messaging
   - Broadcast messaging
   - Message ordering
   - Timeout handling

3. **Consensus Tests**
   - Simple majority (3/5 agents)
   - Super majority (4/5 agents)
   - Unanimous consent
   - Tie breaking

4. **Integration Tests**
   - End-to-end: market signal → consensus → execution
   - Agent disagreement scenarios
   - Performance under load

## Definition of Done

- [ ] 5 specialized agent implementations
- [ ] Coordinator with lifecycle management
- [ ] Pub-sub messaging system
- [ ] Consensus voting (majority, supermajority)
- [ ] 15+ new tests passing
- [ ] Golden Path: `test_consensus_execution` passes
