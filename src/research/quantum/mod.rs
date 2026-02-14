//! Quantum Machine Learning Module
//! Sprint 51: Quantum Computing for HRM
//!
//! Provides quantum computing integration using roqoqo framework.
//! Supports quantum optimization (QAOA), variational circuits (VQE),
//! and hybrid quantum-classical neural networks.

pub mod circuit;
pub mod qaoa;
pub mod vqe;
pub mod backend;

pub use circuit::{QuantumCircuit, VariationalCircuit, HardwareEfficientAnsatz, angle_encoding, amplitude_encoding};
pub use qaoa::{QAOAOptimizer, OptimizationProblem, QAOAResult};
pub use vqe::{VQE, VQEConfig, VQEResult, Hamiltonian, PauliTerm, PauliOp, h2_hamiltonian, ising_hamiltonian};
pub use backend::{QuantumBackend, SimulatorBackend, BackendFactory, ExecutionResult, Measurement};

use thiserror::Error;

/// Quantum computing errors
#[derive(Error, Debug)]
pub enum QuantumError {
    #[error("Circuit execution failed: {0}")]
    ExecutionError(String),
    
    #[error("Invalid circuit: {0}")]
    InvalidCircuit(String),
    
    #[error("Backend unavailable: {0}")]
    BackendUnavailable(String),
    
    #[error("Measurement error: {0}")]
    MeasurementError(String),
    
    #[error("Parameter optimization failed: {0}")]
    OptimizationError(String),
}

/// Result type for quantum operations
pub type Result<T> = std::result::Result<T, QuantumError>;

/// Quantum advantage assessment
#[derive(Debug, Clone)]
pub struct QuantumAdvantage {
    pub quantum_time_ms: f64,
    pub classical_time_ms: f64,
    pub speedup: f64,
    pub circuit_depth: usize,
    pub n_qubits: usize,
}

impl QuantumAdvantage {
    /// Calculate speedup
    pub fn new(quantum: f64, classical: f64, depth: usize, qubits: usize) -> Self {
        Self {
            quantum_time_ms: quantum,
            classical_time_ms: classical,
            speedup: if quantum > 0.0 { classical / quantum } else { 0.0 },
            circuit_depth: depth,
            n_qubits: qubits,
        }
    }
}

/// Check if quantum backend is available
pub fn is_quantum_available() -> bool {
    // Check if we can create a simulator backend
    SimulatorBackend::new().is_ok()
}

/// Initialize quantum module
pub fn init() {
    tracing::info!("Initializing Quantum ML module (Sprint 51)");
    
    if is_quantum_available() {
        tracing::info!("✅ Quantum simulator backend available");
    } else {
        tracing::warn!("⚠️ Quantum backend not available");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quantum_advantage() {
        let advantage = QuantumAdvantage::new(
            100.0,   // quantum: 100ms
            500.0,   // classical: 500ms
            12,      // depth
            5,       // qubits
        );
        
        assert_eq!(advantage.speedup, 5.0);
    }

    #[test]
    fn test_error_display() {
        let err = QuantumError::ExecutionError("test".to_string());
        assert!(err.to_string().contains("test"));
    }
}
