//! Agent Registry
//!
//! Manages agent metadata and discovery

use super::{AgentConfig, AgentId, AgentRole};
use std::collections::HashMap;

/// Registry of agent configurations and metadata
pub struct AgentRegistry {
    configs: HashMap<AgentId, AgentConfig>,
}

impl AgentRegistry {
    pub fn new() -> Self {
        Self {
            configs: HashMap::new(),
        }
    }
    
    /// Register an agent configuration
    pub fn register(&mut self, config: AgentConfig) {
        self.configs.insert(config.id.clone(), config);
    }
    
    /// Deregister an agent
    pub fn deregister(&mut self, agent_id: &AgentId) {
        self.configs.remove(agent_id);
    }
    
    /// Get agent configuration
    pub fn get_config(&self, agent_id: &AgentId) -> Option<AgentConfig> {
        self.configs.get(agent_id).cloned()
    }
    
    /// Get all agents
    pub fn get_all(&self) -> Vec<AgentConfig> {
        self.configs.values().cloned().collect()
    }
    
    /// Get agents by role
    pub fn get_by_role(&self, role: AgentRole) -> Vec<AgentId> {
        self.configs
            .iter()
            .filter(|(_, config)| config.role == role)
            .map(|(id, _)| id.clone())
            .collect()
    }
    
    /// Get role distribution
    pub fn get_role_distribution(&self) -> HashMap<AgentRole, usize> {
        let mut distribution: HashMap<AgentRole, usize> = HashMap::new();
        
        for config in self.configs.values() {
            *distribution.entry(config.role).or_insert(0) += 1;
        }
        
        distribution
    }
    
    /// Search agents by name pattern
    pub fn search_by_name(&self, pattern: &str) -> Vec<AgentConfig> {
        self.configs
            .values()
            .filter(|config| config.name.to_lowercase().contains(&pattern.to_lowercase()))
            .cloned()
            .collect()
    }
    
    /// Count agents
    pub fn count(&self) -> usize {
        self.configs.len()
    }
    
    /// Count agents by role
    pub fn count_by_role(&self, role: AgentRole) -> usize {
        self.configs
            .values()
            .filter(|config| config.role == role)
            .count()
    }
}

impl Default for AgentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_registration() {
        let mut registry = AgentRegistry::new();
        
        let config = AgentConfig::new(AgentRole::MarketAnalyst, "Test Agent")
            .with_description("A test agent");
        
        registry.register(config.clone());
        assert_eq!(registry.count(), 1);
        
        let retrieved = registry.get_config(&config.id);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "Test Agent");
    }

    #[test]
    fn test_get_by_role() {
        let mut registry = AgentRegistry::new();
        
        let config1 = AgentConfig::new(AgentRole::MarketAnalyst, "Analyst 1");
        let config2 = AgentConfig::new(AgentRole::MarketAnalyst, "Analyst 2");
        let config3 = AgentConfig::new(AgentRole::RiskAssessor, "Risk Agent");
        
        registry.register(config1);
        registry.register(config2);
        registry.register(config3);
        
        let analysts = registry.get_by_role(AgentRole::MarketAnalyst);
        assert_eq!(analysts.len(), 2);
        
        let risk = registry.get_by_role(AgentRole::RiskAssessor);
        assert_eq!(risk.len(), 1);
    }

    #[test]
    fn test_role_distribution() {
        let mut registry = AgentRegistry::new();
        
        registry.register(AgentConfig::new(AgentRole::MarketAnalyst, "A1"));
        registry.register(AgentConfig::new(AgentRole::MarketAnalyst, "A2"));
        registry.register(AgentConfig::new(AgentRole::RiskAssessor, "R1"));
        registry.register(AgentConfig::new(AgentRole::Learner, "L1"));
        
        let dist = registry.get_role_distribution();
        assert_eq!(dist.get(&AgentRole::MarketAnalyst), Some(&2));
        assert_eq!(dist.get(&AgentRole::RiskAssessor), Some(&1));
        assert_eq!(dist.get(&AgentRole::Learner), Some(&1));
    }

    #[test]
    fn test_search_by_name() {
        let mut registry = AgentRegistry::new();
        
        registry.register(AgentConfig::new(AgentRole::MarketAnalyst, "Alpha Trader"));
        registry.register(AgentConfig::new(AgentRole::RiskAssessor, "Beta Risk"));
        registry.register(AgentConfig::new(AgentRole::Learner, "Gamma Learner"));
        
        let results = registry.search_by_name("alpha");
        assert_eq!(results.len(), 1);
        
        let results = registry.search_by_name("trader");
        assert_eq!(results.len(), 1);
    }
}
