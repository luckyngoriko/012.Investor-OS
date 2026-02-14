//! Agent Supervisor
//!
//! Monitors agent health and manages recovery

use super::AgentId;
use chrono::{DateTime, Utc};
use std::collections::HashMap;

/// Health status of an agent
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

/// Health check result
#[derive(Debug, Clone)]
pub struct HealthCheck {
    pub agent_id: AgentId,
    pub status: HealthStatus,
    pub last_check: DateTime<Utc>,
    pub response_time_ms: u64,
    pub consecutive_failures: u32,
    pub message_count: u64,
    pub error_count: u64,
}

impl HealthCheck {
    pub fn new(agent_id: AgentId) -> Self {
        Self {
            agent_id,
            status: HealthStatus::Unknown,
            last_check: Utc::now(),
            response_time_ms: 0,
            consecutive_failures: 0,
            message_count: 0,
            error_count: 0,
        }
    }
}

/// Supervisor configuration
#[derive(Debug, Clone)]
pub struct SupervisorConfig {
    pub check_interval_ms: u64,
    pub failure_threshold: u32,
    pub recovery_enabled: bool,
    pub max_recovery_attempts: u32,
}

impl Default for SupervisorConfig {
    fn default() -> Self {
        Self {
            check_interval_ms: 5000,
            failure_threshold: 3,
            recovery_enabled: true,
            max_recovery_attempts: 3,
        }
    }
}

/// Agent supervisor for health monitoring
pub struct AgentSupervisor {
    config: SupervisorConfig,
    health_checks: HashMap<AgentId, HealthCheck>,
    recovery_attempts: HashMap<AgentId, u32>,
}

impl AgentSupervisor {
    pub fn new() -> Self {
        Self::with_config(SupervisorConfig::default())
    }
    
    pub fn with_config(config: SupervisorConfig) -> Self {
        Self {
            config,
            health_checks: HashMap::new(),
            recovery_attempts: HashMap::new(),
        }
    }
    
    /// Register an agent for monitoring
    pub fn register_agent(&mut self, agent_id: AgentId) {
        self.health_checks.insert(agent_id.clone(), HealthCheck::new(agent_id.clone()));
        self.recovery_attempts.insert(agent_id, 0);
    }
    
    /// Deregister an agent
    pub fn deregister_agent(&mut self, agent_id: &AgentId) {
        self.health_checks.remove(agent_id);
        self.recovery_attempts.remove(agent_id);
    }
    
    /// Update health check for an agent
    pub fn update_health(&mut self, agent_id: AgentId, check: HealthCheck) {
        self.health_checks.insert(agent_id, check);
    }
    
    /// Record a successful health check
    pub fn record_success(&mut self, agent_id: &AgentId, response_time_ms: u64) {
        if let Some(check) = self.health_checks.get_mut(agent_id) {
            check.status = HealthStatus::Healthy;
            check.last_check = Utc::now();
            check.response_time_ms = response_time_ms;
            check.consecutive_failures = 0;
        }
    }
    
    /// Record a failed health check
    pub fn record_failure(&mut self, agent_id: &AgentId) {
        if let Some(check) = self.health_checks.get_mut(agent_id) {
            check.consecutive_failures += 1;
            check.last_check = Utc::now();
            
            if check.consecutive_failures >= self.config.failure_threshold {
                check.status = HealthStatus::Unhealthy;
            } else if check.consecutive_failures > 0 {
                check.status = HealthStatus::Degraded;
            }
        }
    }
    
    /// Record message processed
    pub fn record_message(&mut self, agent_id: &AgentId) {
        if let Some(check) = self.health_checks.get_mut(agent_id) {
            check.message_count += 1;
        }
    }
    
    /// Record error
    pub fn record_error(&mut self, agent_id: &AgentId) {
        if let Some(check) = self.health_checks.get_mut(agent_id) {
            check.error_count += 1;
        }
    }
    
    /// Get health status for an agent
    pub fn get_health(&self, agent_id: &AgentId) -> Option<HealthStatus> {
        self.health_checks.get(agent_id).map(|c| c.status)
    }
    
    /// Get full health check data
    pub fn get_health_check(&self, agent_id: &AgentId) -> Option<HealthCheck> {
        self.health_checks.get(agent_id).cloned()
    }
    
    /// Get all unhealthy agents
    pub fn get_unhealthy_agents(&self) -> Vec<AgentId> {
        self.health_checks
            .iter()
            .filter(|(_, check)| check.status == HealthStatus::Unhealthy)
            .map(|(id, _)| id.clone())
            .collect()
    }
    
    /// Get all agents health status
    pub fn get_all_health(&self) -> Vec<HealthCheck> {
        self.health_checks.values().cloned().collect()
    }
    
    /// Check if agent needs recovery
    pub fn needs_recovery(&self, agent_id: &AgentId) -> bool {
        if let Some(check) = self.health_checks.get(agent_id) {
            if check.status == HealthStatus::Unhealthy {
                let attempts = self.recovery_attempts.get(agent_id).copied().unwrap_or(0);
                return attempts < self.config.max_recovery_attempts;
            }
        }
        false
    }
    
    /// Record recovery attempt
    pub fn record_recovery_attempt(&mut self, agent_id: &AgentId) {
        *self.recovery_attempts.entry(agent_id.clone()).or_insert(0) += 1;
    }
    
    /// Reset recovery attempts (after successful recovery)
    pub fn reset_recovery_attempts(&mut self, agent_id: &AgentId) {
        self.recovery_attempts.insert(agent_id.clone(), 0);
    }
    
    /// Get supervisor statistics
    pub fn get_stats(&self) -> SupervisorStats {
        let total = self.health_checks.len();
        let healthy = self.health_checks.values()
            .filter(|c| c.status == HealthStatus::Healthy)
            .count();
        let degraded = self.health_checks.values()
            .filter(|c| c.status == HealthStatus::Degraded)
            .count();
        let unhealthy = self.health_checks.values()
            .filter(|c| c.status == HealthStatus::Unhealthy)
            .count();
        
        SupervisorStats {
            total_agents: total,
            healthy,
            degraded,
            unhealthy,
            unknown: total - healthy - degraded - unhealthy,
        }
    }
    
}

impl Default for AgentSupervisor {
    fn default() -> Self {
        Self::new()
    }
}

/// Supervisor statistics
#[derive(Debug, Clone)]
pub struct SupervisorStats {
    pub total_agents: usize,
    pub healthy: usize,
    pub degraded: usize,
    pub unhealthy: usize,
    pub unknown: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_tracking() {
        let mut supervisor = AgentSupervisor::new();
        let agent_id = AgentId::from_string("test_agent");
        
        supervisor.register_agent(agent_id.clone());
        
        // Initially unknown
        assert_eq!(supervisor.get_health(&agent_id), Some(HealthStatus::Unknown));
        
        // Record success
        supervisor.record_success(&agent_id, 50);
        assert_eq!(supervisor.get_health(&agent_id), Some(HealthStatus::Healthy));
        
        // Record failures
        supervisor.record_failure(&agent_id);
        supervisor.record_failure(&agent_id);
        assert_eq!(supervisor.get_health(&agent_id), Some(HealthStatus::Degraded));
        
        // More failures
        supervisor.record_failure(&agent_id);
        assert_eq!(supervisor.get_health(&agent_id), Some(HealthStatus::Unhealthy));
    }

    #[test]
    fn test_unhealthy_agents() {
        let mut supervisor = AgentSupervisor::new();
        
        let agent1 = AgentId::from_string("agent1");
        let agent2 = AgentId::from_string("agent2");
        let agent3 = AgentId::from_string("agent3");
        
        supervisor.register_agent(agent1.clone());
        supervisor.register_agent(agent2.clone());
        supervisor.register_agent(agent3.clone());
        
        // Make agent1 unhealthy
        for _ in 0..5 {
            supervisor.record_failure(&agent1);
        }
        
        // agent2 is healthy
        supervisor.record_success(&agent2, 100);
        
        // agent3 is degraded
        supervisor.record_failure(&agent3);
        
        let unhealthy = supervisor.get_unhealthy_agents();
        assert_eq!(unhealthy.len(), 1);
        assert!(unhealthy.contains(&agent1));
    }

    #[test]
    fn test_recovery_tracking() {
        let mut supervisor = AgentSupervisor::new();
        let agent_id = AgentId::from_string("agent");
        
        supervisor.register_agent(agent_id.clone());
        
        // Make unhealthy
        for _ in 0..3 {
            supervisor.record_failure(&agent_id);
        }
        
        assert!(supervisor.needs_recovery(&agent_id));
        
        // Record recovery attempts
        supervisor.record_recovery_attempt(&agent_id);
        supervisor.record_recovery_attempt(&agent_id);
        supervisor.record_recovery_attempt(&agent_id);
        
        // Max attempts reached
        assert!(!supervisor.needs_recovery(&agent_id));
    }

    #[test]
    fn test_supervisor_stats() {
        let mut supervisor = AgentSupervisor::new();
        
        supervisor.register_agent(AgentId::from_string("h1"));
        supervisor.register_agent(AgentId::from_string("h2"));
        supervisor.register_agent(AgentId::from_string("d1"));
        supervisor.register_agent(AgentId::from_string("u1"));
        
        supervisor.record_success(&AgentId::from_string("h1"), 100);
        supervisor.record_success(&AgentId::from_string("h2"), 100);
        
        supervisor.record_failure(&AgentId::from_string("d1"));
        
        for _ in 0..3 {
            supervisor.record_failure(&AgentId::from_string("u1"));
        }
        
        let stats = supervisor.get_stats();
        assert_eq!(stats.total_agents, 4);
        assert_eq!(stats.healthy, 2);
        assert_eq!(stats.degraded, 1);
        assert_eq!(stats.unhealthy, 1);
    }
}
