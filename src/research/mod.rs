//! Experimental & Research Module - Sprint 21
//!
//! Cutting-edge research features:
//! - Quantum ML (IBM Quantum, QAOA)
//! - Federated Learning (privacy-preserving)
//! - Neuromorphic Computing (Intel Loihi)
//! - Predictive Regime Detection
//! - Market Microstructure Analysis

pub mod quantum;
pub mod federated;
pub mod neuromorphic;
pub mod predictive;
pub mod microstructure;

pub use quantum::{
    QuantumCircuit, VariationalCircuit, HardwareEfficientAnsatz, 
    angle_encoding, amplitude_encoding,
    QAOAOptimizer, OptimizationProblem, QAOAResult,
    VQE, VQEConfig, VQEResult, Hamiltonian, PauliTerm, PauliOp,
    h2_hamiltonian, ising_hamiltonian,
    QuantumBackend, SimulatorBackend, BackendFactory, ExecutionResult, Measurement,
    QuantumAdvantage,
    QuantumError, Result as QuantumResult, is_quantum_available, init as init_quantum,
};

pub use federated::{
    FederatedCoordinator, FederatedClient, FederatedConfig,
    AggregationStrategy, RoundResult, ModelUpdate, FederatedError,
    FederatedProgress, ClientState,
};

pub use neuromorphic::{
    SpikingNeuralNetwork, SnnConfig, NeuromorphicBackend,
    SnnInferenceResult, NeuromorphicInferenceEngine,
    NeuronState, Synapse, SpikeEvent, SnnStats,
    NeuromorphicError,
};

pub use predictive::{
    PredictiveRegimeDetector, RegimeDetectorConfig, MarketRegime,
    RegimeForecast, EarlyWarning, TrendDirection, MarketDataPoint,
    RegimeTransitionSignal, RegimeAction, RegimeError,
};

pub use microstructure::{
    MicrostructureAnalyzer, OrderBook, OrderBookLevel,
    TradeTick, VolumeBar, VpinCalculator, LiquidityMetrics,
    AdverseSelectionEstimator, Side, MicrostructureError,
};

use thiserror::Error;

/// Research module errors
#[derive(Error, Debug, Clone)]
pub enum ResearchError {
    #[error("Quantum error: {0}")]
    Quantum(String),
    
    #[error("Federated learning error: {0}")]
    Federated(#[from] FederatedError),
    
    #[error("Neuromorphic error: {0}")]
    Neuromorphic(#[from] NeuromorphicError),
    
    #[error("Regime detection error: {0}")]
    Regime(#[from] RegimeError),
    
    #[error("Microstructure error: {0}")]
    Microstructure(#[from] MicrostructureError),
}

impl From<QuantumError> for ResearchError {
    fn from(e: QuantumError) -> Self {
        ResearchError::Quantum(e.to_string())
    }
}

/// Research technology status
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TechnologyStatus {
    /// Theoretical/concept
    Research,
    /// Early prototype
    Prototype,
    /// Testing/validation
    Testing,
    /// Production ready
    Production,
}

/// Research area information
#[derive(Debug, Clone)]
pub struct ResearchArea {
    pub name: String,
    pub description: String,
    pub status: TechnologyStatus,
    pub readiness_level: u8, // 1-9
}

/// Get all research areas
pub fn research_areas() -> Vec<ResearchArea> {
    vec![
        ResearchArea {
            name: "Quantum ML".to_string(),
            description: "Portfolio optimization using quantum computers".to_string(),
            status: TechnologyStatus::Research,
            readiness_level: 3,
        },
        ResearchArea {
            name: "Federated Learning".to_string(),
            description: "Privacy-preserving distributed ML".to_string(),
            status: TechnologyStatus::Prototype,
            readiness_level: 5,
        },
        ResearchArea {
            name: "Neuromorphic Computing".to_string(),
            description: "Ultra-low latency inference with SNNs".to_string(),
            status: TechnologyStatus::Prototype,
            readiness_level: 4,
        },
        ResearchArea {
            name: "Predictive Regime Detection".to_string(),
            description: "Forecast market regime changes".to_string(),
            status: TechnologyStatus::Testing,
            readiness_level: 6,
        },
        ResearchArea {
            name: "Market Microstructure".to_string(),
            description: "Order book dynamics and flow toxicity".to_string(),
            status: TechnologyStatus::Testing,
            readiness_level: 7,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_research_areas() {
        let areas = research_areas();
        assert_eq!(areas.len(), 5);
        
        let quantum = areas.iter().find(|a| a.name == "Quantum ML").unwrap();
        assert_eq!(quantum.readiness_level, 3);
    }

    #[test]
    fn test_technology_status() {
        use TechnologyStatus::*;
        
        assert_ne!(Research, Production);
        assert_eq!(Research as u8, Research as u8);
    }
}
