//! Agent Coordinator
//!
//! Orchestrates multiple agents:
//! - Lifecycle management (register, start, stop, deregister)
//! - Task distribution to appropriate agents
//! - Health monitoring
//! - Dynamic scaling

use super::communication::CommunicationHub;
use super::consensus::{ConsensusEngine, ConsensusResult, Proposal, WeightedVote};
use super::registry::AgentRegistry;
use super::supervisor::{AgentSupervisor, HealthStatus};
use super::{
    Agent, AgentConfig, AgentError, AgentId, AgentMessage, AgentRole, AgentStatus,
    MessagePayload, MessageType, Task, TaskResult,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use tokio::time::timeout;
use tracing::{debug, error, info, warn};

/// Coordinator configuration
#[derive(Debug, Clone)]
pub struct CoordinatorConfig {
    pub max_agents: usize,
    pub task_timeout_ms: u64,
    pub consensus_timeout_ms: u64,
    pub enable_auto_recovery: bool,
    pub health_check_interval_ms: u64,
}

impl Default for CoordinatorConfig {
    fn default() -> Self {
        Self {
            max_agents: 100,
            task_timeout_ms: 30000,
            consensus_timeout_ms: 60000,
            enable_auto_recovery: true,
            health_check_interval_ms: 5000,
        }
    }
}

/// Main agent coordinator
pub struct AgentCoordinator {
    config: CoordinatorConfig,
    /// Registered agents
    agents: Arc<RwLock<HashMap<AgentId, AgentHandle>>>,
    /// Communication hub
    hub: Arc<CommunicationHub>,
    /// Consensus engine
    consensus: Arc<RwLock<ConsensusEngine>>,
    /// Agent registry
    registry: Arc<RwLock<AgentRegistry>>,
    /// Supervisor for health monitoring
    supervisor: Arc<RwLock<AgentSupervisor>>,
    /// Task response channels
    task_channels: Arc<RwLock<HashMap<String, mpsc::Sender<TaskResult>>>>,
}

/// Agent handle containing agent instance and metadata
struct AgentHandle {
    #[allow(dead_code)]
    config: AgentConfig,
    sender: mpsc::Sender<AgentCommand>,
}

/// Commands sent to agent tasks
#[derive(Debug)]
enum AgentCommand {
    ProcessTask(Task, mpsc::Sender<TaskResult>),
    HandleMessage(AgentMessage),
    Pause,
    Resume,
    Shutdown,
}

impl AgentCoordinator {
    /// Create a new coordinator
    pub fn new(config: CoordinatorConfig) -> Self {
        let hub = Arc::new(CommunicationHub::new());
        
        Self {
            config,
            agents: Arc::new(RwLock::new(HashMap::new())),
            hub,
            consensus: Arc::new(RwLock::new(ConsensusEngine::new())),
            registry: Arc::new(RwLock::new(AgentRegistry::new())),
            supervisor: Arc::new(RwLock::new(AgentSupervisor::new())),
            task_channels: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Register a new agent
    pub async fn register_agent<A: Agent + 'static>(
        &self,
        config: AgentConfig,
        mut agent: A,
    ) -> Result<AgentId, AgentError> {
        let agents = self.agents.read().await;
        if agents.len() >= self.config.max_agents {
            return Err(AgentError::AgentError("Max agents reached".to_string()));
        }
        drop(agents);
        
        let agent_id = config.id.clone();
        let (cmd_tx, mut cmd_rx) = mpsc::channel(100);
        
        // Register with communication hub
        let mut msg_rx = self.hub.register_agent(agent_id.clone()).await;
        
        // Register with registry
        let mut registry = self.registry.write().await;
        registry.register(config.clone());
        drop(registry);
        
        // Register with supervisor
        let mut supervisor = self.supervisor.write().await;
        supervisor.register_agent(agent_id.clone());
        drop(supervisor);
        
        // Subscribe to relevant message types based on role
        match config.role {
            AgentRole::MarketAnalyst => {
                self.hub.subscribe(agent_id.clone(), MessageType::Observation).await;
                self.hub.subscribe(agent_id.clone(), MessageType::Request).await;
            }
            AgentRole::RiskAssessor => {
                self.hub.subscribe(agent_id.clone(), MessageType::Warning).await;
                self.hub.subscribe(agent_id.clone(), MessageType::Request).await;
            }
            AgentRole::ExecutionSpecialist => {
                self.hub.subscribe(agent_id.clone(), MessageType::Request).await;
            }
            AgentRole::SentimentReader => {
                self.hub.subscribe(agent_id.clone(), MessageType::Broadcast).await;
            }
            _ => {}
        }
        
        // Spawn agent task
        let agent_id_clone = agent_id.clone();
        tokio::spawn(async move {
            info!("Agent {} started", agent_id_clone);
            
            loop {
                tokio::select! {
                    // Handle commands
                    Some(cmd) = cmd_rx.recv() => {
                        match cmd {
                            AgentCommand::ProcessTask(task, result_tx) => {
                                debug!("Agent {} processing task {}", agent_id_clone, 
                                    match &task.id { super::TaskId(s) => s.as_str() });
                                
                                match agent.process(task).await {
                                    Ok(result) => {
                                        let _ = result_tx.send(result).await;
                                    }
                                    Err(e) => {
                                        error!("Agent {} task error: {}", agent_id_clone, e);
                                    }
                                }
                            }
                            AgentCommand::HandleMessage(msg) => {
                                if let Err(e) = agent.on_message(msg).await {
                                    error!("Agent {} message error: {}", agent_id_clone, e);
                                }
                            }
                            AgentCommand::Pause => {
                                agent.pause().await;
                            }
                            AgentCommand::Resume => {
                                agent.resume().await;
                            }
                            AgentCommand::Shutdown => {
                                agent.shutdown().await;
                                info!("Agent {} shutdown", agent_id_clone);
                                break;
                            }
                        }
                    }
                    // Handle messages from hub
                    Some(msg) = msg_rx.recv() => {
                        if let Err(e) = agent.on_message(msg).await {
                            error!("Agent {} message error: {}", agent_id_clone, e);
                        }
                    }
                }
            }
        });
        
        // Store agent handle
        let mut agents = self.agents.write().await;
        agents.insert(agent_id.clone(), AgentHandle {
            config,
            sender: cmd_tx,
        });
        
        info!("Registered agent {} with coordinator", agent_id);
        Ok(agent_id)
    }
    
    /// Deregister an agent
    pub async fn deregister_agent(&self, agent_id: &AgentId) -> Result<(), AgentError> {
        let mut agents = self.agents.write().await;
        
        if let Some(handle) = agents.remove(agent_id) {
            // Send shutdown command
            let _ = handle.sender.send(AgentCommand::Shutdown).await;
            
            // Deregister from hub
            self.hub.deregister_agent(agent_id).await;
            
            // Deregister from registry
            let mut registry = self.registry.write().await;
            registry.deregister(agent_id);
            
            // Deregister from supervisor
            let mut supervisor = self.supervisor.write().await;
            supervisor.deregister_agent(agent_id);
            
            info!("Deregistered agent {} from coordinator", agent_id);
            Ok(())
        } else {
            Err(AgentError::AgentNotFound(agent_id.clone()))
        }
    }
    
    /// Send task to specific agent
    pub async fn send_task(
        &self,
        agent_id: &AgentId,
        task: Task,
    ) -> Result<TaskResult, AgentError> {
        let task_id = task.id.clone(); // Clone before moving
        let agents = self.agents.read().await;
        
        if let Some(handle) = agents.get(agent_id) {
            let (result_tx, mut result_rx) = mpsc::channel(1);
            
            // Send task to agent
            handle.sender
                .send(AgentCommand::ProcessTask(task, result_tx))
                .await
                .map_err(|_| AgentError::CommunicationError("Agent channel closed".to_string()))?;
            
            drop(agents);
            
            // Wait for result with timeout
            let timeout_duration = Duration::from_millis(self.config.task_timeout_ms);
            match timeout(timeout_duration, result_rx.recv()).await {
                Ok(Some(result)) => Ok(result),
                Ok(None) => Err(AgentError::CommunicationError("Result channel closed".to_string())),
                Err(_) => Err(AgentError::TaskTimeout(task_id)),
            }
        } else {
            Err(AgentError::AgentNotFound(agent_id.clone()))
        }
    }
    
    /// Broadcast message to all agents
    pub async fn broadcast(&self, msg: AgentMessage) -> Result<usize, AgentError> {
        self.hub.broadcast(msg).await
    }
    
    /// Send message to specific agent
    pub async fn send_to(
        &self,
        msg: AgentMessage,
        to: &AgentId,
    ) -> Result<(), AgentError> {
        self.hub.send_to(msg, to).await
    }
    
    /// Get agents by role
    pub async fn get_agents_by_role(&self, role: AgentRole) -> Vec<AgentId> {
        let registry = self.registry.read().await;
        registry.get_by_role(role)
    }
    
    /// Get agent status
    pub async fn get_agent_status(&self, agent_id: &AgentId) -> Option<AgentStatus> {
        let supervisor = self.supervisor.read().await;
        supervisor.get_health(agent_id).map(|h| match h {
            HealthStatus::Healthy => AgentStatus::Active,
            HealthStatus::Degraded => AgentStatus::Busy,
            HealthStatus::Unhealthy => AgentStatus::Error,
            HealthStatus::Unknown => AgentStatus::Initializing,
        })
    }
    
    /// Get all agent IDs
    pub async fn get_all_agents(&self) -> Vec<AgentId> {
        let agents = self.agents.read().await;
        agents.keys().cloned().collect()
    }
    
    /// Create consensus proposal
    pub async fn propose(
        &self,
        proposal: Proposal,
    ) -> Result<ConsensusResult, AgentError> {
        let eligible_voters = self.get_all_agents().await;
        
        if eligible_voters.is_empty() {
            return Err(AgentError::AgentError("No agents available for consensus".to_string()));
        }
        
        let mut consensus = self.consensus.write().await;
        let proposal_id = consensus.create_proposal(proposal, eligible_voters.clone());
        
        // Broadcast proposal to all agents
        let proposal_msg = AgentMessage::broadcast(
            AgentId::from_string("coordinator"),
            MessageType::Request,
            MessagePayload::Request(super::RequestData {
                request_id: proposal_id.0.clone(),
                request_type: "vote".to_string(),
                parameters: [(
                    "proposal_id".to_string(),
                    proposal_id.0.clone()
                )].into_iter().collect(),
            }),
        );
        
        drop(consensus);
        self.broadcast(proposal_msg).await?;
        
        // Wait for votes (simplified - in production, use proper async waiting)
        let consensus_timeout = Duration::from_millis(self.config.consensus_timeout_ms);
        let start = std::time::Instant::now();
        
        loop {
            if start.elapsed() > consensus_timeout {
                let mut consensus = self.consensus.write().await;
                return Ok(consensus.finalize(&proposal_id));
            }
            
            let consensus = self.consensus.write().await;
            if let Some(result) = consensus.check_consensus(&proposal_id) {
                return Ok(result);
            }
            drop(consensus);
            
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
    
    /// Submit a vote for consensus
    pub async fn submit_vote(
        &self,
        proposal_id: &super::consensus::ProposalId,
        vote: WeightedVote,
    ) -> Result<(), AgentError> {
        let mut consensus = self.consensus.write().await;
        consensus.vote(proposal_id, vote)
    }
    
    /// Pause an agent
    pub async fn pause_agent(&self, agent_id: &AgentId) -> Result<(), AgentError> {
        let agents = self.agents.read().await;
        
        if let Some(handle) = agents.get(agent_id) {
            handle.sender
                .send(AgentCommand::Pause)
                .await
                .map_err(|_| AgentError::CommunicationError("Agent channel closed".to_string()))?;
            Ok(())
        } else {
            Err(AgentError::AgentNotFound(agent_id.clone()))
        }
    }
    
    /// Resume an agent
    pub async fn resume_agent(&self, agent_id: &AgentId) -> Result<(), AgentError> {
        let agents = self.agents.read().await;
        
        if let Some(handle) = agents.get(agent_id) {
            handle.sender
                .send(AgentCommand::Resume)
                .await
                .map_err(|_| AgentError::CommunicationError("Agent channel closed".to_string()))?;
            Ok(())
        } else {
            Err(AgentError::AgentNotFound(agent_id.clone()))
        }
    }
    
    /// Get coordinator statistics
    pub async fn get_stats(&self) -> CoordinatorStats {
        let agents = self.agents.read().await;
        let registry = self.registry.read().await;
        let consensus = self.consensus.read().await;
        
        CoordinatorStats {
            total_agents: agents.len(),
            active_agents: agents.len(), // Simplified
            agents_by_role: registry.get_role_distribution(),
            active_proposals: consensus.get_active_proposals().len(),
        }
    }
    
    /// Shutdown all agents
    pub async fn shutdown_all(&self) {
        let agent_ids: Vec<_> = {
            let agents = self.agents.read().await;
            agents.keys().cloned().collect()
        };
        
        for agent_id in agent_ids {
            if let Err(e) = self.deregister_agent(&agent_id).await {
                warn!("Error shutting down agent {}: {}", agent_id, e);
            }
        }
        
        info!("All agents shutdown");
    }
}

/// Coordinator statistics
#[derive(Debug, Clone)]
pub struct CoordinatorStats {
    pub total_agents: usize,
    pub active_agents: usize,
    pub agents_by_role: HashMap<AgentRole, usize>,
    pub active_proposals: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::agents::market_analyst::MarketAnalystAgent;

    #[tokio::test]
    async fn test_coordinator_creation() {
        let coordinator = AgentCoordinator::new(CoordinatorConfig::default());
        assert_eq!(coordinator.get_all_agents().await.len(), 0);
    }

    #[tokio::test]
    async fn test_agent_registration() {
        let coordinator = AgentCoordinator::new(CoordinatorConfig::default());
        
        let config = AgentConfig::new(AgentRole::MarketAnalyst, "Test Analyst")
            .with_description("Test agent");
        
        let agent = MarketAnalystAgent::new(config.clone());
        let id = coordinator.register_agent(config, agent).await.unwrap();
        
        assert_eq!(coordinator.get_all_agents().await.len(), 1);
        assert!(coordinator.get_agent_status(&id).await.is_some());
        
        // Cleanup
        coordinator.deregister_agent(&id).await.unwrap();
    }

    #[tokio::test]
    async fn test_broadcast() {
        let coordinator = AgentCoordinator::new(CoordinatorConfig::default());
        
        // Register an agent
        let config = AgentConfig::new(AgentRole::MarketAnalyst, "Analyst");
        let agent = MarketAnalystAgent::new(config.clone());
        let id = coordinator.register_agent(config, agent).await.unwrap();
        
        // Broadcast message
        let msg = AgentMessage::broadcast(
            AgentId::from_string("coordinator"),
            MessageType::Broadcast,
            MessagePayload::Broadcast("Hello".to_string()),
        );
        
        let count = coordinator.broadcast(msg).await.unwrap();
        assert_eq!(count, 1);
        
        // Cleanup
        coordinator.deregister_agent(&id).await.unwrap();
    }

    #[tokio::test]
    async fn test_get_agents_by_role() {
        let coordinator = AgentCoordinator::new(CoordinatorConfig::default());
        
        // Register agents of different roles
        let config1 = AgentConfig::new(AgentRole::MarketAnalyst, "Analyst 1");
        let agent1 = MarketAnalystAgent::new(config1.clone());
        let id1 = coordinator.register_agent(config1, agent1).await.unwrap();
        
        let config2 = AgentConfig::new(AgentRole::MarketAnalyst, "Analyst 2");
        let agent2 = MarketAnalystAgent::new(config2.clone());
        let id2 = coordinator.register_agent(config2, agent2).await.unwrap();
        
        let analysts = coordinator.get_agents_by_role(AgentRole::MarketAnalyst).await;
        assert_eq!(analysts.len(), 2);
        assert!(analysts.contains(&id1));
        assert!(analysts.contains(&id2));
        
        // Cleanup
        coordinator.deregister_agent(&id1).await.unwrap();
        coordinator.deregister_agent(&id2).await.unwrap();
    }
}
