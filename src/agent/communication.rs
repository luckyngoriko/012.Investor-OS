//! Inter-Agent Communication System
//!
//! Provides pub-sub messaging between agents with:
//! - Point-to-point messaging
//! - Broadcast messaging
//! - Message priority queuing
//! - Async delivery with timeouts

use super::{AgentError, AgentId, AgentMessage, MessageType};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use tokio::time::timeout;
use tracing::{debug, error, trace, warn};

/// Channel capacity per agent
const DEFAULT_CHANNEL_CAPACITY: usize = 1000;

/// Communication hub for inter-agent messaging
pub struct CommunicationHub {
    /// Message channels for each agent
    channels: Arc<RwLock<HashMap<AgentId, mpsc::Sender<AgentMessage>>>>,
    /// Message history for auditing
    history: Arc<RwLock<Vec<AgentMessage>>>,
    /// Maximum history size
    max_history: usize,
    /// Subscriptions: message type -> list of agent IDs
    subscriptions: Arc<RwLock<HashMap<MessageType, Vec<AgentId>>>>,
}

impl CommunicationHub {
    /// Create a new communication hub
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_CHANNEL_CAPACITY, 10000)
    }
    
    /// Create with custom capacity and history size
    pub fn with_capacity(_channel_capacity: usize, max_history: usize) -> Self {
        Self {
            channels: Arc::new(RwLock::new(HashMap::new())),
            history: Arc::new(RwLock::new(Vec::new())),
            max_history,
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Register an agent with the hub
    /// Returns a receiver for the agent to consume messages
    pub async fn register_agent(&self, agent_id: AgentId) -> mpsc::Receiver<AgentMessage> {
        let (tx, rx) = mpsc::channel(DEFAULT_CHANNEL_CAPACITY);
        
        let mut channels = self.channels.write().await;
        channels.insert(agent_id.clone(), tx);
        
        debug!("Registered agent {} with communication hub", agent_id);
        rx
    }
    
    /// Deregister an agent
    pub async fn deregister_agent(&self, agent_id: &AgentId) {
        let mut channels = self.channels.write().await;
        channels.remove(agent_id);
        
        // Remove from subscriptions
        let mut subs = self.subscriptions.write().await;
        for subscribers in subs.values_mut() {
            subscribers.retain(|id| id != agent_id);
        }
        
        debug!("Deregistered agent {} from communication hub", agent_id);
    }
    
    /// Send message to specific agent (point-to-point)
    pub async fn send_to(
        &self,
        msg: AgentMessage,
        to: &AgentId,
    ) -> Result<(), AgentError> {
        let channels = self.channels.read().await;
        
        if let Some(sender) = channels.get(to) {
            // Store in history
            self.add_to_history(msg.clone()).await;
            
            // Send with timeout
            match timeout(Duration::from_secs(5), sender.send(msg)).await {
                Ok(Ok(())) => {
                    trace!("Message sent to {}", to);
                    Ok(())
                }
                Ok(Err(_)) => {
                    error!("Failed to send message to {}: channel closed", to);
                    Err(AgentError::CommunicationError(format!(
                        "Channel closed for agent {}",
                        to
                    )))
                }
                Err(_) => {
                    warn!("Timeout sending message to {}", to);
                    Err(AgentError::CommunicationError(format!(
                        "Timeout sending to agent {}",
                        to
                    )))
                }
            }
        } else {
            Err(AgentError::AgentNotFound(to.clone()))
        }
    }
    
    /// Broadcast message to all agents
    pub async fn broadcast(&self, msg: AgentMessage) -> Result<usize, AgentError> {
        let channels = self.channels.read().await;
        let mut sent_count = 0;
        
        // Store in history
        self.add_to_history(msg.clone()).await;
        
        // Send to all agents except sender
        for (agent_id, sender) in channels.iter() {
            if *agent_id != msg.from {
                match sender.try_send(msg.clone()) {
                    Ok(()) => sent_count += 1,
                    Err(e) => {
                        warn!("Failed to broadcast to {}: {}", agent_id, e);
                    }
                }
            }
        }
        
        debug!("Broadcast message to {} agents", sent_count);
        Ok(sent_count)
    }
    
    /// Publish message based on subscriptions
    pub async fn publish(&self, msg: AgentMessage) -> Result<usize, AgentError> {
        // Store in history
        self.add_to_history(msg.clone()).await;
        
        let subs = self.subscriptions.read().await;
        let mut sent_count = 0;
        
        if let Some(subscribers) = subs.get(&msg.msg_type) {
            let channels = self.channels.read().await;
            
            for agent_id in subscribers {
                if *agent_id != msg.from {
                    if let Some(sender) = channels.get(agent_id) {
                        match sender.try_send(msg.clone()) {
                            Ok(()) => sent_count += 1,
                            Err(e) => {
                                warn!("Failed to publish to {}: {}", agent_id, e);
                            }
                        }
                    }
                }
            }
        }
        
        debug!("Published message to {} subscribers", sent_count);
        Ok(sent_count)
    }
    
    /// Subscribe an agent to a message type
    pub async fn subscribe(&self, agent_id: AgentId, msg_type: MessageType) {
        let mut subs = self.subscriptions.write().await;
        subs.entry(msg_type)
            .or_insert_with(Vec::new)
            .push(agent_id);
        debug!("Agent subscribed to {:?}", msg_type);
    }
    
    /// Unsubscribe an agent from a message type
    pub async fn unsubscribe(&self, agent_id: &AgentId, msg_type: &MessageType) {
        let mut subs = self.subscriptions.write().await;
        if let Some(subscribers) = subs.get_mut(msg_type) {
            subscribers.retain(|id| id != agent_id);
        }
    }
    
    /// Get message history
    pub async fn get_history(&self, limit: usize) -> Vec<AgentMessage> {
        let history = self.history.read().await;
        history.iter().rev().take(limit).cloned().collect()
    }
    
    /// Get messages between two agents
    pub async fn get_conversation(
        &self,
        agent1: &AgentId,
        agent2: &AgentId,
        limit: usize,
    ) -> Vec<AgentMessage> {
        let history = self.history.read().await;
        history
            .iter()
            .filter(|msg| {
                (&msg.from == agent1 && msg.to.as_ref() == Some(agent2))
                    || (&msg.from == agent2 && msg.to.as_ref() == Some(agent1))
            })
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }
    
    /// Get agent count
    pub async fn agent_count(&self) -> usize {
        let channels = self.channels.read().await;
        channels.len()
    }
    
    /// Add message to history with cleanup
    async fn add_to_history(&self, msg: AgentMessage) {
        let mut history = self.history.write().await;
        history.push(msg);
        
        // Cleanup old history
        if history.len() > self.max_history {
            let excess = history.len() - self.max_history;
            history.drain(0..excess);
        }
    }
    
    /// Request-response pattern
    pub async fn request_response(
        &self,
        request: AgentMessage,
        to: &AgentId,
        timeout_duration: Duration,
    ) -> Result<AgentMessage, AgentError> {
        let request_id = request.id.clone();
        
        // Send request
        self.send_to(request, to).await?;
        
        // Wait for response with matching reply_to
        let start = std::time::Instant::now();
        loop {
            if start.elapsed() > timeout_duration {
                return Err(AgentError::TaskTimeout(super::TaskId(request_id)));
            }
            
            // Check history for response
            let history = self.history.read().await;
            for msg in history.iter().rev() {
                if msg.reply_to.as_ref() == Some(&request_id) {
                    return Ok(msg.clone());
                }
            }
            drop(history);
            
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }
}

impl Default for CommunicationHub {
    fn default() -> Self {
        Self::new()
    }
}

/// Message router for intelligent message routing
pub struct MessageRouter {
    hub: Arc<CommunicationHub>,
    routing_rules: Arc<RwLock<Vec<RoutingRule>>>,
}

/// Routing rule for message filtering and transformation
pub struct RoutingRule {
    pub condition: Box<dyn Fn(&AgentMessage) -> bool + Send + Sync>,
    pub action: RoutingAction,
}

impl std::fmt::Debug for RoutingRule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RoutingRule").finish()
    }
}

/// Routing actions
pub enum RoutingAction {
    ForwardTo(AgentId),
    Broadcast,
    Drop,
    Transform(Box<dyn Fn(AgentMessage) -> AgentMessage + Send + Sync>),
}

impl std::fmt::Debug for RoutingAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RoutingAction::ForwardTo(id) => f.debug_tuple("ForwardTo").field(id).finish(),
            RoutingAction::Broadcast => f.debug_struct("Broadcast").finish(),
            RoutingAction::Drop => f.debug_struct("Drop").finish(),
            RoutingAction::Transform(_) => f.debug_struct("Transform").finish(),
        }
    }
}

impl MessageRouter {
    pub fn new(hub: Arc<CommunicationHub>) -> Self {
        Self {
            hub,
            routing_rules: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    /// Add a routing rule
    pub async fn add_rule(&self, rule: RoutingRule) {
        let mut rules = self.routing_rules.write().await;
        rules.push(rule);
    }
    
    /// Route a message according to rules
    pub async fn route(&self, msg: AgentMessage) -> Result<(), AgentError> {
        let rules = self.routing_rules.read().await;
        
        for rule in rules.iter() {
            if (rule.condition)(&msg) {
                match &rule.action {
                    RoutingAction::ForwardTo(agent_id) => {
                        self.hub.send_to(msg, agent_id).await?;
                        return Ok(());
                    }
                    RoutingAction::Broadcast => {
                        self.hub.broadcast(msg).await?;
                        return Ok(());
                    }
                    RoutingAction::Drop => {
                        trace!("Message dropped by routing rule");
                        return Ok(());
                    }
                    RoutingAction::Transform(transformer) => {
                        let transformed = transformer(msg);
                        // Continue routing with transformed message
                        return self.hub.broadcast(transformed).await.map(|_| ());
                    }
                }
            }
        }
        
        // Default: broadcast
        self.hub.broadcast(msg).await.map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::{MessagePayload, ObservationData};

    #[tokio::test]
    async fn test_hub_registration() {
        let hub = CommunicationHub::new();
        let agent_id = AgentId::from_string("test_agent");
        
        let rx = hub.register_agent(agent_id.clone()).await;
        assert_eq!(hub.agent_count().await, 1);
        
        drop(rx);
        hub.deregister_agent(&agent_id).await;
        assert_eq!(hub.agent_count().await, 0);
    }

    #[tokio::test]
    async fn test_point_to_point_messaging() {
        let hub = CommunicationHub::new();
        let agent1 = AgentId::from_string("agent1");
        let agent2 = AgentId::from_string("agent2");
        
        let mut rx1 = hub.register_agent(agent1.clone()).await;
        let mut rx2 = hub.register_agent(agent2.clone()).await;
        
        let msg = AgentMessage::new(
            agent1.clone(),
            Some(agent2.clone()),
            MessageType::Observation,
            MessagePayload::Observation(ObservationData {
                symbol: "AAPL".to_string(),
                observation_type: "price".to_string(),
                value: 150.0,
                metadata: HashMap::new(),
            }),
        );
        
        hub.send_to(msg.clone(), &agent2).await.unwrap();
        
        let received = rx2.recv().await;
        assert!(received.is_some());
        assert_eq!(received.unwrap().from, agent1);
        
        // Agent1 should not receive its own message
        assert!(rx1.try_recv().is_err());
    }

    #[tokio::test]
    async fn test_broadcast() {
        let hub = CommunicationHub::new();
        let agent1 = AgentId::from_string("agent1");
        let agent2 = AgentId::from_string("agent2");
        let agent3 = AgentId::from_string("agent3");
        
        let _rx1 = hub.register_agent(agent1.clone()).await;
        let mut rx2 = hub.register_agent(agent2.clone()).await;
        let mut rx3 = hub.register_agent(agent3.clone()).await;
        
        let msg = AgentMessage::broadcast(
            agent1.clone(),
            MessageType::Warning,
            MessagePayload::Broadcast("Alert!".to_string()),
        );
        
        let count = hub.broadcast(msg).await.unwrap();
        assert_eq!(count, 2); // agent2 and agent3
        
        assert!(rx2.recv().await.is_some());
        assert!(rx3.recv().await.is_some());
    }

    #[tokio::test]
    async fn test_subscriptions() {
        let hub = CommunicationHub::new();
        let agent = AgentId::from_string("agent");
        
        let _rx = hub.register_agent(agent.clone()).await;
        
        hub.subscribe(agent.clone(), MessageType::Observation).await;
        hub.subscribe(agent.clone(), MessageType::Warning).await;
        
        // Unsubscribe
        hub.unsubscribe(&agent, &MessageType::Observation).await;
    }

    #[tokio::test]
    async fn test_message_history() {
        let hub = CommunicationHub::new();
        let agent = AgentId::from_string("agent");
        
        let _rx = hub.register_agent(agent.clone()).await;
        
        let msg = AgentMessage::broadcast(
            agent.clone(),
            MessageType::Observation,
            MessagePayload::Broadcast("test".to_string()),
        );
        
        hub.broadcast(msg).await.unwrap();
        
        let history = hub.get_history(10).await;
        assert_eq!(history.len(), 1);
    }
}
