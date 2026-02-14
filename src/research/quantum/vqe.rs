//! Variational Quantum Eigensolver (VQE)
//!
//! For quantum chemistry simulations and ground state preparation

use super::{QuantumCircuit, VariationalCircuit, Result};
use super::backend::QuantumBackend;

/// VQE configuration
#[derive(Debug, Clone)]
pub struct VQEConfig {
    /// Number of qubits
    pub n_qubits: usize,
    /// Number of variational layers
    pub n_layers: usize,
    /// Maximum optimization iterations
    pub max_iterations: usize,
    /// Convergence tolerance
    pub tolerance: f64,
    /// Learning rate
    pub learning_rate: f64,
}

impl Default for VQEConfig {
    fn default() -> Self {
        Self {
            n_qubits: 4,
            n_layers: 2,
            max_iterations: 100,
            tolerance: 1e-6,
            learning_rate: 0.01,
        }
    }
}

/// VQE for finding ground state energy
pub struct VQE {
    config: VQEConfig,
    backend: Box<dyn QuantumBackend>,
    ansatz: HardwareEfficientAnsatz,
}

/// Hamiltonian term
#[derive(Debug, Clone)]
pub struct PauliTerm {
    pub coefficient: f64,
    pub operators: Vec<(usize, PauliOp)>, // (qubit, operator)
}

/// Pauli operators
#[derive(Debug, Clone, Copy)]
pub enum PauliOp {
    I,
    X,
    Y,
    Z,
}

/// Molecular Hamiltonian
#[derive(Debug, Clone)]
pub struct Hamiltonian {
    pub terms: Vec<PauliTerm>,
}

/// VQE optimization result
#[derive(Debug, Clone)]
pub struct VQEResult {
    pub ground_state_energy: f64,
    pub optimal_params: Vec<f64>,
    pub iterations: usize,
    pub energy_history: Vec<f64>,
    pub converged: bool,
}

impl VQE {
    /// Create new VQE instance
    pub fn new(config: VQEConfig, backend: Box<dyn QuantumBackend>) -> Self {
        let ansatz = HardwareEfficientAnsatz::new(
            config.n_qubits,
            config.n_layers
        );
        
        Self {
            config,
            backend,
            ansatz,
        }
    }

    /// Find ground state energy
    pub fn optimize(&self, hamiltonian: &Hamiltonian) -> Result<VQEResult> {
        let n_params = self.ansatz.n_parameters();
        let mut params: Vec<f64> = (0..n_params)
            .map(|_| fastrand::f64() * 2.0 * std::f64::consts::PI)
            .collect();

        let mut energy_history = Vec::with_capacity(self.config.max_iterations);
        let mut converged = false;
        let mut best_energy = f64::INFINITY;

        for iter in 0..self.config.max_iterations {
            let energy = self.evaluate_energy(&params, hamiltonian)?;
            energy_history.push(energy);

            if energy < best_energy {
                best_energy = energy;
            }

            // Check convergence
            if iter > 10 {
                let recent: Vec<f64> = energy_history.iter().rev().take(5).copied().collect();
                let variance = Self::variance(&recent);
                if variance < self.config.tolerance {
                    converged = true;
                    break;
                }
            }

            // Update parameters
            params = self.gradient_descent_step(&params, hamiltonian)?;
        }

        Ok(VQEResult {
            ground_state_energy: best_energy,
            optimal_params: params,
            iterations: energy_history.len(),
            energy_history,
            converged,
        })
    }

    fn evaluate_energy(&self, params: &[f64], hamiltonian: &Hamiltonian) -> Result<f64> {
        let circuit = self.ansatz.build(params);
        
        let mut energy = 0.0;
        for term in &hamiltonian.terms {
            let exp_val = self.measure_pauli(&circuit, &term.operators)?;
            energy += term.coefficient * exp_val;
        }
        
        Ok(energy)
    }

    fn measure_pauli(
        &self,
        circuit: &QuantumCircuit,
        operators: &[(usize, PauliOp)]
    ) -> Result<f64> {
        // Apply basis rotations for X and Y measurements
        let mut meas_circuit = circuit.clone();
        
        for (qubit, op) in operators {
            match op {
                PauliOp::X => {
                    // Rotate to X basis: H
                    meas_circuit.ry(*qubit, -std::f64::consts::FRAC_PI_2);
                }
                PauliOp::Y => {
                    // Rotate to Y basis: H * S†
                    meas_circuit.rx(*qubit, std::f64::consts::FRAC_PI_2);
                }
                _ => {} // Z or I: no rotation needed
            }
        }

        let result = self.backend.execute(&meas_circuit, 1024)?;
        
        // Calculate expectation from measurements
        let mut exp = 0.0;
        for m in &result.measurements {
            let mut parity = 1.0;
            for (qubit, op) in operators {
                if matches!(op, PauliOp::I) {
                    continue;
                }
                let bit = m.bitstring.chars().nth(*qubit).map(|c| c == '1').unwrap_or(false);
                if bit {
                    parity *= -1.0;
                }
            }
            exp += parity * m.probability;
        }
        
        Ok(exp)
    }

    fn gradient_descent_step(&self, params: &[f64], hamiltonian: &Hamiltonian) -> Result<Vec<f64>> {
        let eps = 0.005;
        let current_energy = self.evaluate_energy(params, hamiltonian)?;
        
        let mut new_params = params.to_vec();
        
        for i in 0..params.len() {
            let mut perturbed = params.to_vec();
            perturbed[i] += eps;
            let perturbed_energy = self.evaluate_energy(&perturbed, hamiltonian)?;
            
            let gradient = (perturbed_energy - current_energy) / eps;
            new_params[i] -= self.config.learning_rate * gradient;
        }
        
        Ok(new_params)
    }

    fn variance(values: &[f64]) -> f64 {
        if values.is_empty() {
            return 0.0;
        }
        let mean: f64 = values.iter().sum::<f64>() / values.len() as f64;
        values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / values.len() as f64
    }
}

/// Hardware-efficient ansatz for VQE
#[derive(Debug, Clone)]
pub struct HardwareEfficientAnsatz {
    n_qubits: usize,
    n_layers: usize,
}

impl HardwareEfficientAnsatz {
    pub fn new(n_qubits: usize, n_layers: usize) -> Self {
        Self { n_qubits, n_layers }
    }

    pub fn n_parameters(&self) -> usize {
        self.n_qubits * self.n_layers * 2 // RY and RZ per qubit per layer
    }

    pub fn build(&self, params: &[f64]) -> QuantumCircuit {
        let mut circuit = QuantumCircuit::new(self.n_qubits);
        let mut param_idx = 0;

        for _layer in 0..self.n_layers {
            // Rotation layer
            for q in 0..self.n_qubits {
                if param_idx < params.len() {
                    circuit.ry(q, params[param_idx]);
                    param_idx += 1;
                }
                if param_idx < params.len() {
                    circuit.rz(q, params[param_idx]);
                    param_idx += 1;
                }
            }

            // Entanglement layer (nearest neighbor CNOTs)
            for q in 0..self.n_qubits - 1 {
                circuit.cnot(q, q + 1);
            }
            
            // Optional: connect ends for ring topology
            if self.n_qubits > 2 {
                circuit.cnot(self.n_qubits - 1, 0);
            }
        }

        circuit
    }
}

/// Create molecular hydrogen Hamiltonian (H2)
pub fn h2_hamiltonian() -> Hamiltonian {
    // Simplified H2 Hamiltonian in STO-3G basis
    // This is a toy model - real implementation would use PySCF/OpenFermion
    Hamiltonian {
        terms: vec![
            PauliTerm {
                coefficient: -0.5,
                operators: vec![(0, PauliOp::Z)],
            },
            PauliTerm {
                coefficient: -0.5,
                operators: vec![(1, PauliOp::Z)],
            },
            PauliTerm {
                coefficient: 0.25,
                operators: vec![(0, PauliOp::Z), (1, PauliOp::Z)],
            },
            PauliTerm {
                coefficient: 0.25,
                operators: vec![(0, PauliOp::X), (1, PauliOp::X)],
            },
        ],
    }
}

/// Create Ising model Hamiltonian
pub fn ising_hamiltonian(n_qubits: usize, j: f64, h: f64) -> Hamiltonian {
    let mut terms = Vec::new();
    
    // Z-Z interactions
    for i in 0..n_qubits - 1 {
        terms.push(PauliTerm {
            coefficient: j,
            operators: vec![(i, PauliOp::Z), (i + 1, PauliOp::Z)],
        });
    }
    
    // X fields
    for i in 0..n_qubits {
        terms.push(PauliTerm {
            coefficient: h,
            operators: vec![(i, PauliOp::X)],
        });
    }
    
    Hamiltonian { terms }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::backend::SimulatorBackend;

    #[test]
    fn test_hardware_efficient_ansatz() {
        let ansatz = HardwareEfficientAnsatz::new(4, 2);
        let params = vec![0.1; ansatz.n_parameters()];
        let circuit = ansatz.build(&params);
        
        assert_eq!(circuit.n_qubits(), 4);
        assert!(circuit.depth() > 0);
    }

    #[test]
    fn test_h2_hamiltonian() {
        let hamiltonian = h2_hamiltonian();
        assert_eq!(hamiltonian.terms.len(), 4);
    }

    #[test]
    fn test_vqe_optimization() {
        let config = VQEConfig {
            n_qubits: 2,
            n_layers: 1,
            max_iterations: 10,
            tolerance: 1e-4,
            learning_rate: 0.1,
        };
        
        let backend = Box::new(SimulatorBackend::with_qubits(2));
        let vqe = VQE::new(config, backend);
        let hamiltonian = h2_hamiltonian();
        
        let result = vqe.optimize(&hamiltonian);
        assert!(result.is_ok());
        
        let result = result.unwrap();
        assert!(result.ground_state_energy < 0.0); // Bound state
    }
}
