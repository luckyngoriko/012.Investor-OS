//! Quantum Backend Abstraction
//!
//! Provides unified interface for simulator and hardware backends

use super::{QuantumError, Result};
use crate::research::quantum::QuantumCircuit;

/// Measurement result
#[derive(Debug, Clone)]
pub struct Measurement {
    pub bitstring: String,
    pub count: usize,
    pub probability: f64,
}

/// Execution result
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub measurements: Vec<Measurement>,
    pub expectation_values: Vec<f64>,
    pub shots: usize,
    pub backend_time_ms: f64,
}

/// Quantum backend trait
pub trait QuantumBackend: Send + Sync {
    /// Execute circuit and return measurements
    fn execute(&self, circuit: &QuantumCircuit, shots: usize) -> Result<ExecutionResult>;
    
    /// Get expectation value of observable
    fn expectation_value(
        &self,
        circuit: &QuantumCircuit,
        qubits: &[usize],
    ) -> Result<Vec<f64>>;
    
    /// Backend name
    fn name(&self) -> &str;
    
    /// Maximum available qubits
    fn max_qubits(&self) -> usize;
    
    /// Check if backend is available
    fn is_available(&self) -> bool;
}

/// Local simulator backend
pub struct SimulatorBackend {
    name: String,
    max_qubits: usize,
}

impl SimulatorBackend {
    pub fn new() -> Result<Self> {
        Ok(Self {
            name: "QuestSimulator".to_string(),
            max_qubits: 20, // Practical limit for simulation
        })
    }

    pub fn with_qubits(n: usize) -> Self {
        Self {
            name: "QuestSimulator".to_string(),
            max_qubits: n,
        }
    }
}

impl Default for SimulatorBackend {
    fn default() -> Self {
        Self::with_qubits(20)
    }
}

impl QuantumBackend for SimulatorBackend {
    fn name(&self) -> &str {
        &self.name
    }

    fn max_qubits(&self) -> usize {
        self.max_qubits
    }

    fn is_available(&self) -> bool {
        true
    }

    fn execute(&self, circuit: &QuantumCircuit, shots: usize) -> Result<ExecutionResult> {
        let start = std::time::Instant::now();
        
        // Get statevector probabilities
        let probs = circuit.execute()?;
        
        // Sample from distribution
        let mut measurements: std::collections::HashMap<String, usize> = 
            std::collections::HashMap::new();
        
        let mut rng = fastrand::Rng::new();
        for _ in 0..shots {
            let r: f64 = rng.f64();
            let mut cumsum = 0.0;
            for (idx, &p) in probs.iter().enumerate() {
                cumsum += p;
                if r <= cumsum {
                    let bitstring = format!("{:0width$b}", idx, width = circuit.n_qubits());
                    *measurements.entry(bitstring).or_insert(0) += 1;
                    break;
                }
            }
        }

        let total: usize = measurements.values().sum();
        let measurements: Vec<Measurement> = measurements
            .into_iter()
            .map(|(bitstring, count)| {
                Measurement {
                    bitstring,
                    count,
                    probability: count as f64 / total as f64,
                }
            })
            .collect();

        // Calculate expectation values (Z for each qubit)
        let mut expectation_values = vec![0.0; circuit.n_qubits()];
        for qubit in 0..circuit.n_qubits() {
            let mut exp = 0.0;
            for (idx, &p) in probs.iter().enumerate() {
                let bit = (idx >> qubit) & 1;
                let sign = if bit == 0 { 1.0 } else { -1.0 };
                exp += sign * p;
            }
            expectation_values[qubit] = exp;
        }

        Ok(ExecutionResult {
            measurements,
            expectation_values,
            shots,
            backend_time_ms: start.elapsed().as_secs_f64() * 1000.0,
        })
    }

    fn expectation_value(
        &self,
        circuit: &QuantumCircuit,
        qubits: &[usize],
    ) -> Result<Vec<f64>> {
        let probs = circuit.execute()?;
        
        let mut expectations = Vec::with_capacity(qubits.len());
        for &qubit in qubits {
            let mut exp = 0.0;
            for (idx, &p) in probs.iter().enumerate() {
                let bit = (idx >> qubit) & 1;
                let sign = if bit == 0 { 1.0 } else { -1.0 };
                exp += sign * p;
            }
            expectations.push(exp);
        }
        
        Ok(expectations)
    }
}

/// Quantum hardware backend (IBM Quantum, etc.)
pub struct HardwareBackend {
    name: String,
    n_qubits: usize,
    backend_url: Option<String>,
    api_token: Option<String>,
}

impl HardwareBackend {
    pub fn new(name: &str, n_qubits: usize) -> Self {
        Self {
            name: name.to_string(),
            n_qubits,
            backend_url: None,
            api_token: None,
        }
    }

    pub fn with_credentials(mut self, url: &str, token: &str) -> Self {
        self.backend_url = Some(url.to_string());
        self.api_token = Some(token.to_string());
        self
    }
}

impl QuantumBackend for HardwareBackend {
    fn name(&self) -> &str {
        &self.name
    }

    fn max_qubits(&self) -> usize {
        self.n_qubits
    }

    fn is_available(&self) -> bool {
        self.backend_url.is_some() && self.api_token.is_some()
    }

    fn execute(&self, _circuit: &QuantumCircuit, _shots: usize) -> Result<ExecutionResult> {
        if !self.is_available() {
            return Err(QuantumError::BackendUnavailable(
                "Hardware backend not configured".to_string()
            ));
        }
        
        // TODO: Implement actual hardware API calls
        // For now, use simulator as fallback
        let sim = SimulatorBackend::with_qubits(self.n_qubits);
        sim.execute(_circuit, _shots)
    }

    fn expectation_value(
        &self,
        circuit: &QuantumCircuit,
        qubits: &[usize],
    ) -> Result<Vec<f64>> {
        if !self.is_available() {
            return Err(QuantumError::BackendUnavailable(
                "Hardware backend not configured".to_string()
            ));
        }
        
        let sim = SimulatorBackend::with_qubits(self.n_qubits);
        sim.expectation_value(circuit, qubits)
    }
}

/// Backend factory
pub struct BackendFactory;

impl BackendFactory {
    /// Create simulator backend
    pub fn simulator() -> Box<dyn QuantumBackend> {
        Box::new(SimulatorBackend::default())
    }

    /// Create IBM Quantum backend
    pub fn ibm_quantum(api_token: &str) -> Box<dyn QuantumBackend> {
        Box::new(
            HardwareBackend::new("ibm_quantum", 127)
                .with_credentials("https://quantum-computing.ibm.com", api_token)
        )
    }

    /// Create best available backend
    pub fn best_available() -> Box<dyn QuantumBackend> {
        Self::simulator()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simulator_backend() {
        let backend = SimulatorBackend::new().unwrap();
        assert!(backend.is_available());
        assert_eq!(backend.max_qubits(), 20);
    }

    #[test]
    fn test_backend_factory() {
        let backend = BackendFactory::simulator();
        assert!(backend.is_available());
    }
}
