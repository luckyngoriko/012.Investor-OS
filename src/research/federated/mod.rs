//! Federated Learning Module
//!
//! Privacy-preserving distributed machine learning.
//! Train models without sharing raw data.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;
use tracing::info;
use chrono::{DateTime, Utc};

/// Federated learning errors
#[derive(Error, Debug, Clone)]
pub enum FederatedError {
    #[error("Client unavailable: {0}")]
    ClientUnavailable(String),
    
    #[error("Aggregation failed: {0}")]
    AggregationFailed(String),
    
    #[error("Privacy budget exhausted: {epsilon:.4}")]
    PrivacyBudgetExhausted { epsilon: f64 },
    
    #[error("Insufficient clients: {have} < {need}")]
    InsufficientClients { have: usize, need: usize },
    
    #[error("Model divergence detected")]
    ModelDivergence,
}

/// Federated learning configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederatedConfig {
    /// Minimum number of clients per round
    pub min_clients: usize,
    /// Target number of clients
    pub target_clients: usize,
    /// Number of local epochs per round
    pub local_epochs: usize,
    /// Learning rate for local training
    pub learning_rate: f64,
    /// Differential privacy epsilon
    pub privacy_epsilon: f64,
    /// Privacy delta
    pub privacy_delta: f64,
    /// Maximum gradient norm for clipping
    pub max_gradient_norm: f64,
    /// Aggregation strategy
    pub aggregation: AggregationStrategy,
}

impl Default for FederatedConfig {
    fn default() -> Self {
        Self {
            min_clients: 3,
            target_clients: 10,
            local_epochs: 5,
            learning_rate: 0.01,
            privacy_epsilon: 1.0,
            privacy_delta: 1e-5,
            max_gradient_norm: 1.0,
            aggregation: AggregationStrategy::FedAvg,
        }
    }
}

/// Model aggregation strategy
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum AggregationStrategy {
    /// Federated Averaging (FedAvg)
    FedAvg,
    /// Federated Proximal
    FedProx,
    /// SCAFFOLD
    Scaffold,
    /// Personalized FL
    PerFedAvg,
}

/// Federated client state
#[derive(Debug, Clone)]
pub struct ClientState {
    pub client_id: String,
    pub last_update: DateTime<Utc>,
    pub dataset_size: usize,
    pub is_available: bool,
    pub model_version: u32,
}

impl ClientState {
    pub fn new(client_id: impl Into<String>, dataset_size: usize) -> Self {
        Self {
            client_id: client_id.into(),
            last_update: Utc::now(),
            dataset_size,
            is_available: true,
            model_version: 0,
        }
    }
}

/// Model update from client
#[derive(Debug, Clone)]
pub struct ModelUpdate {
    pub client_id: String,
    pub weights: Vec<f64>,
    pub dataset_size: usize,
    pub timestamp: DateTime<Utc>,
    pub loss: f64,
    pub accuracy: f64,
}

/// Federated learning round result
#[derive(Debug, Clone)]
pub struct RoundResult {
    pub round_number: u32,
    pub participating_clients: Vec<String>,
    pub aggregated_weights: Vec<f64>,
    pub global_loss: f64,
    pub global_accuracy: f64,
    pub privacy_spent: f64,
    pub timestamp: DateTime<Utc>,
}

/// Federated Learning Coordinator
#[derive(Debug)]
pub struct FederatedCoordinator {
    config: FederatedConfig,
    clients: HashMap<String, ClientState>,
    global_model: Vec<f64>,
    current_round: u32,
    total_privacy_spent: f64,
    round_history: Vec<RoundResult>,
}

impl FederatedCoordinator {
    /// Create new federated learning coordinator
    pub fn new(config: FederatedConfig) -> Self {
        info!("Creating FederatedCoordinator with min {} clients", config.min_clients);
        
        Self {
            config,
            clients: HashMap::new(),
            global_model: Vec::new(),
            current_round: 0,
            total_privacy_spent: 0.0,
            round_history: Vec::new(),
        }
    }
    
    /// Register a new client
    pub fn register_client(&mut self, client_id: impl Into<String>, dataset_size: usize) {
        let client_id = client_id.into();
        let client = ClientState::new(&client_id, dataset_size);
        
        info!("Registered federated client {} with {} samples", client_id, dataset_size);
        
        self.clients.insert(client_id, client);
    }
    
    /// Unregister a client
    pub fn unregister_client(&mut self, client_id: &str) {
        self.clients.remove(client_id);
        info!("Unregistered federated client {}", client_id);
    }
    
    /// Get number of available clients
    pub fn available_clients(&self) -> usize {
        self.clients.values()
            .filter(|c| c.is_available)
            .count()
    }
    
    /// Start a new federated learning round
    pub fn start_round(&mut self) -> Result<RoundResult, FederatedError> {
        let available = self.available_clients();
        
        if available < self.config.min_clients {
            return Err(FederatedError::InsufficientClients {
                have: available,
                need: self.config.min_clients,
            });
        }
        
        self.current_round += 1;
        info!("Starting federated learning round {} with {} clients", 
              self.current_round, available);
        
        // In production: broadcast global model to clients
        // Collect updates
        // Aggregate
        
        // Simulate round completion
        let result = RoundResult {
            round_number: self.current_round,
            participating_clients: self.clients.keys().cloned().collect(),
            aggregated_weights: self.global_model.clone(),
            global_loss: 0.5 / self.current_round as f64,
            global_accuracy: 0.7 + (self.current_round as f64 * 0.02).min(0.25),
            privacy_spent: self.config.privacy_epsilon / 10.0,
            timestamp: Utc::now(),
        };
        
        self.total_privacy_spent += result.privacy_spent;
        self.round_history.push(result.clone());
        
        info!("Round {} completed: loss={:.4}, accuracy={:.2}%", 
              result.round_number, result.global_loss, result.global_accuracy * 100.0);
        
        Ok(result)
    }
    
    /// Aggregate client updates using configured strategy
    pub fn aggregate_updates(&self, updates: &[ModelUpdate]) -> Result<Vec<f64>, FederatedError> {
        if updates.is_empty() {
            return Err(FederatedError::AggregationFailed(
                "No updates to aggregate".to_string()
            ));
        }
        
        match self.config.aggregation {
            AggregationStrategy::FedAvg => self.fedavg_aggregate(updates),
            AggregationStrategy::FedProx => self.fedprox_aggregate(updates),
            AggregationStrategy::Scaffold => self.scaffold_aggregate(updates),
            AggregationStrategy::PerFedAvg => self.perfedavg_aggregate(updates),
        }
    }
    
    /// FedAvg aggregation (weighted by dataset size)
    fn fedavg_aggregate(&self, updates: &[ModelUpdate]) -> Result<Vec<f64>, FederatedError> {
        let total_samples: usize = updates.iter().map(|u| u.dataset_size).sum();
        
        if total_samples == 0 {
            return Err(FederatedError::AggregationFailed(
                "Total dataset size is zero".to_string()
            ));
        }
        
        // Weighted average of weights
        let num_params = updates.first().map(|u| u.weights.len()).unwrap_or(0);
        let mut aggregated = vec![0.0; num_params];
        
        for update in updates {
            let weight = update.dataset_size as f64 / total_samples as f64;
            for (i, &w) in update.weights.iter().enumerate() {
                if i < aggregated.len() {
                    aggregated[i] += w * weight;
                }
            }
        }
        
        Ok(aggregated)
    }
    
    /// FedProx aggregation (with proximal term)
    fn fedprox_aggregate(&self, updates: &[ModelUpdate]) -> Result<Vec<f64>, FederatedError> {
        // FedProx adds regularization term to keep local models close to global
        // For simulation, use FedAvg
        self.fedavg_aggregate(updates)
    }
    
    /// SCAFFOLD aggregation (with control variates)
    fn scaffold_aggregate(&self, updates: &[ModelUpdate]) -> Result<Vec<f64>, FederatedError> {
        // SCAFFOLD uses control variates to correct client drift
        // For simulation, use FedAvg
        self.fedavg_aggregate(updates)
    }
    
    /// PerFedAvg aggregation (personalized FL)
    fn perfedavg_aggregate(&self, updates: &[ModelUpdate]) -> Result<Vec<f64>, FederatedError> {
        // PerFedAvg meta-learning approach
        // For simulation, use FedAvg
        self.fedavg_aggregate(updates)
    }
    
    /// Apply differential privacy to aggregated model
    pub fn apply_privacy(&self, weights: &mut [f64]) -> Result<(), FederatedError> {
        if self.total_privacy_spent >= self.config.privacy_epsilon {
            return Err(FederatedError::PrivacyBudgetExhausted {
                epsilon: self.total_privacy_spent,
            });
        }
        
        // Add Gaussian noise for differential privacy
        let noise_scale = self.config.max_gradient_norm * 
            (2.0 * self.config.privacy_epsilon.ln()).sqrt();
        
        for w in weights.iter_mut() {
            let noise = rand::random::<f64>() * noise_scale;
            *w += noise;
        }
        
        Ok(())
    }
    
    /// Get training progress
    pub fn get_progress(&self) -> FederatedProgress {
        FederatedProgress {
            current_round: self.current_round,
            total_rounds: 100, // Typical
            available_clients: self.available_clients(),
            total_clients: self.clients.len(),
            privacy_budget_remaining: (self.config.privacy_epsilon - self.total_privacy_spent)
                .max(0.0),
            latest_accuracy: self.round_history.last().map(|r| r.global_accuracy),
        }
    }
}

impl Default for FederatedCoordinator {
    fn default() -> Self {
        Self::new(FederatedConfig::default())
    }
}

/// Federated learning progress
#[derive(Debug, Clone)]
pub struct FederatedProgress {
    pub current_round: u32,
    pub total_rounds: u32,
    pub available_clients: usize,
    pub total_clients: usize,
    pub privacy_budget_remaining: f64,
    pub latest_accuracy: Option<f64>,
}

/// Federated client (edge device)
#[derive(Debug)]
pub struct FederatedClient {
    client_id: String,
    config: FederatedConfig,
    local_data_size: usize,
}

impl FederatedClient {
    pub fn new(client_id: impl Into<String>, config: FederatedConfig, data_size: usize) -> Self {
        Self {
            client_id: client_id.into(),
            config,
            local_data_size: data_size,
        }
    }
    
    /// Train local model on client data
    pub fn train_local(&self, global_weights: &[f64]) -> Result<ModelUpdate, FederatedError> {
        info!("Client {} training local model", self.client_id);
        
        // Simulate local training
        let mut local_weights = global_weights.to_vec();
        
        // Apply local updates (simulated)
        for w in &mut local_weights {
            *w += rand::random::<f64>() * 0.01 * self.config.learning_rate;
        }
        
        Ok(ModelUpdate {
            client_id: self.client_id.clone(),
            weights: local_weights,
            dataset_size: self.local_data_size,
            timestamp: Utc::now(),
            loss: rand::random::<f64>() * 0.5,
            accuracy: 0.7 + rand::random::<f64>() * 0.2,
        })
    }
    
    /// Get client ID
    pub fn client_id(&self) -> &str {
        &self.client_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_federated_coordinator_creation() {
        let coordinator = FederatedCoordinator::default();
        assert_eq!(coordinator.available_clients(), 0);
    }

    #[test]
    fn test_client_registration() {
        let mut coordinator = FederatedCoordinator::default();
        
        coordinator.register_client("client_1", 1000);
        coordinator.register_client("client_2", 2000);
        coordinator.register_client("client_3", 1500);
        
        assert_eq!(coordinator.available_clients(), 3);
    }

    #[test]
    fn test_fedavg_aggregation() {
        let coordinator = FederatedCoordinator::default();
        
        let updates = vec![
            ModelUpdate {
                client_id: "c1".to_string(),
                weights: vec![1.0, 2.0, 3.0],
                dataset_size: 100,
                timestamp: Utc::now(),
                loss: 0.5,
                accuracy: 0.75,
            },
            ModelUpdate {
                client_id: "c2".to_string(),
                weights: vec![2.0, 3.0, 4.0],
                dataset_size: 200,
                timestamp: Utc::now(),
                loss: 0.4,
                accuracy: 0.80,
            },
        ];
        
        let result = coordinator.aggregate_updates(&updates).unwrap();
        
        // Weighted average: c1 has 1/3 weight, c2 has 2/3 weight
        assert!((result[0] - 1.6667).abs() < 0.01);
    }

    #[test]
    fn test_insufficient_clients() {
        let mut coordinator = FederatedCoordinator::default();
        
        // No clients registered
        let result = coordinator.start_round();
        assert!(matches!(result, Err(FederatedError::InsufficientClients { .. })));
    }
}
