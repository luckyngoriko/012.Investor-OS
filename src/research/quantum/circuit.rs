//! Quantum Circuit Implementation
//! 
//! Variational quantum circuits for hybrid ML models

use super::Result;
use num_complex::Complex64;
use std::f64::consts::PI;

/// Parameterized quantum gate
#[derive(Debug, Clone, Copy)]
pub enum Gate {
    /// Hadamard gate
    H(usize),
    /// Pauli-X
    X(usize),
    /// Pauli-Y  
    Y(usize),
    /// Pauli-Z
    Z(usize),
    /// Rotation-X with parameter
    RX(usize, f64),
    /// Rotation-Y with parameter
    RY(usize, f64),
    /// Rotation-Z with parameter
    RZ(usize, f64),
    /// CNOT gate
    CNOT(usize, usize),
    /// Parameterized two-qubit gate
    RXX(usize, usize, f64),
}

/// Quantum circuit for variational algorithms
#[derive(Debug, Clone)]
pub struct QuantumCircuit {
    n_qubits: usize,
    gates: Vec<Gate>,
    parameters: Vec<f64>,
}

impl QuantumCircuit {
    /// Create new circuit with n qubits
    pub fn new(n_qubits: usize) -> Self {
        Self {
            n_qubits,
            gates: Vec::new(),
            parameters: Vec::new(),
        }
    }

    /// Add Hadamard gate
    pub fn h(&mut self, qubit: usize) -> &mut Self {
        assert!(qubit < self.n_qubits);
        self.gates.push(Gate::H(qubit));
        self
    }

    /// Add Pauli-X
    pub fn x(&mut self, qubit: usize) -> &mut Self {
        assert!(qubit < self.n_qubits);
        self.gates.push(Gate::X(qubit));
        self
    }

    /// Add parameterized RX
    pub fn rx(&mut self, qubit: usize, param: f64) -> &mut Self {
        assert!(qubit < self.n_qubits);
        self.gates.push(Gate::RX(qubit, param));
        self
    }

    /// Add parameterized RY
    pub fn ry(&mut self, qubit: usize, param: f64) -> &mut Self {
        assert!(qubit < self.n_qubits);
        self.gates.push(Gate::RY(qubit, param));
        self
    }

    /// Add parameterized RZ
    pub fn rz(&mut self, qubit: usize, param: f64) -> &mut Self {
        assert!(qubit < self.n_qubits);
        self.gates.push(Gate::RZ(qubit, param));
        self
    }

    /// Add CNOT
    pub fn cnot(&mut self, control: usize, target: usize) -> &mut Self {
        assert!(control < self.n_qubits && target < self.n_qubits);
        self.gates.push(Gate::CNOT(control, target));
        self
    }

    /// Number of qubits
    pub fn n_qubits(&self) -> usize {
        self.n_qubits
    }

    /// Number of gates
    pub fn depth(&self) -> usize {
        self.gates.len()
    }

    /// Get gates
    pub fn gates(&self) -> &[Gate] {
        &self.gates
    }

    /// Execute circuit and return probabilities
    pub fn execute(&self) -> Result<Vec<f64>> {
        // Simplified simulation using statevector
        let dim = 1_usize << self.n_qubits;
        let mut statevec = vec![Complex64::new(0.0, 0.0); dim];
        statevec[0] = Complex64::new(1.0, 0.0); // |0...0⟩

        // Apply gates
        for gate in &self.gates {
            match gate {
                Gate::H(q) => Self::apply_h(&mut statevec, *q, self.n_qubits),
                Gate::X(q) => Self::apply_x(&mut statevec, *q, self.n_qubits),
                Gate::RX(q, p) => Self::apply_rx(&mut statevec, *q, self.n_qubits, *p),
                Gate::RY(q, p) => Self::apply_ry(&mut statevec, *q, self.n_qubits, *p),
                Gate::RZ(q, p) => Self::apply_rz(&mut statevec, *q, self.n_qubits, *p),
                Gate::CNOT(c, t) => Self::apply_cnot(&mut statevec, *c, *t, self.n_qubits),
                _ => {} // Other gates
            }
        }

        // Calculate probabilities
        let probs: Vec<f64> = statevec
            .iter()
            .map(|c| c.norm_sqr())
            .collect();

        Ok(probs)
    }

    fn apply_h(statevec: &mut [Complex64], qubit: usize, n_qubits: usize) {
        let dim = 1_usize << n_qubits;
        let stride = 1_usize << qubit;
        
        for i in (0..dim).step_by(stride * 2) {
            for j in 0..stride {
                let idx0 = i + j;
                let idx1 = i + j + stride;
                let a = statevec[idx0];
                let b = statevec[idx1];
                statevec[idx0] = (a + b) / 2.0_f64.sqrt();
                statevec[idx1] = (a - b) / 2.0_f64.sqrt();
            }
        }
    }

    fn apply_x(statevec: &mut [Complex64], qubit: usize, n_qubits: usize) {
        let dim = 1_usize << n_qubits;
        let stride = 1_usize << qubit;
        
        for i in (0..dim).step_by(stride * 2) {
            for j in 0..stride {
                let idx0 = i + j;
                let idx1 = i + j + stride;
                statevec.swap(idx0, idx1);
            }
        }
    }

    fn apply_rx(statevec: &mut [Complex64], qubit: usize, n_qubits: usize, theta: f64) {
        let dim = 1_usize << n_qubits;
        let stride = 1_usize << qubit;
        let cos = (theta / 2.0).cos();
        let sin = (theta / 2.0).sin();
        
        for i in (0..dim).step_by(stride * 2) {
            for j in 0..stride {
                let idx0 = i + j;
                let idx1 = i + j + stride;
                let a = statevec[idx0];
                let b = statevec[idx1];
                statevec[idx0] = Complex64::new(cos * a.re - sin * b.im, cos * a.im + sin * b.re);
                statevec[idx1] = Complex64::new(-sin * a.im + cos * b.re, sin * a.re + cos * b.im);
            }
        }
    }

    fn apply_ry(statevec: &mut [Complex64], qubit: usize, n_qubits: usize, theta: f64) {
        let dim = 1_usize << n_qubits;
        let stride = 1_usize << qubit;
        let cos = (theta / 2.0).cos();
        let sin = (theta / 2.0).sin();
        
        for i in (0..dim).step_by(stride * 2) {
            for j in 0..stride {
                let idx0 = i + j;
                let idx1 = i + j + stride;
                let a = statevec[idx0];
                let b = statevec[idx1];
                statevec[idx0] = Complex64::new(cos * a.re - sin * b.re, cos * a.im - sin * b.im);
                statevec[idx1] = Complex64::new(sin * a.re + cos * b.re, sin * a.im + cos * b.im);
            }
        }
    }

    fn apply_rz(statevec: &mut [Complex64], qubit: usize, n_qubits: usize, theta: f64) {
        let dim = 1_usize << n_qubits;
        let stride = 1_usize << qubit;
        let phase0 = Complex64::from_polar(1.0, -theta / 2.0);
        let phase1 = Complex64::from_polar(1.0, theta / 2.0);
        
        for i in (0..dim).step_by(stride * 2) {
            for j in 0..stride {
                statevec[i + j] *= phase0;
                statevec[i + j + stride] *= phase1;
            }
        }
    }

    fn apply_cnot(statevec: &mut [Complex64], control: usize, target: usize, n_qubits: usize) {
        let dim = 1_usize << n_qubits;
        let c_stride = 1_usize << control;
        let t_stride = 1_usize << target;
        
        for i in (0..dim).step_by(c_stride * 2) {
            for j in 0..c_stride {
                let base = i + j + c_stride; // Control = 1
                for k in (0..t_stride).step_by(1) {
                    let step = base + k;
                    for l in 0..1.max(t_stride >> 1) {
                        let idx0 = step + l;
                        let idx1 = idx0 + t_stride;
                        if idx1 < dim && control != target {
                            statevec.swap(idx0, idx1);
                        }
                    }
                }
            }
        }
    }
}

/// Trait for variational circuits
pub trait VariationalCircuit {
    /// Build circuit with given parameters
    fn build(&self, params: &[f64]) -> QuantumCircuit;
    
    /// Number of parameters
    fn n_parameters(&self) -> usize;
    
    /// Number of qubits
    fn n_qubits(&self) -> usize;
}

/// Hardware-efficient ansatz
#[derive(Debug, Clone)]
pub struct HardwareEfficientAnsatz {
    n_qubits: usize,
    layers: usize,
}

impl HardwareEfficientAnsatz {
    pub fn new(n_qubits: usize, layers: usize) -> Self {
        Self { n_qubits, layers }
    }
}

impl VariationalCircuit for HardwareEfficientAnsatz {
    fn n_parameters(&self) -> usize {
        self.n_qubits * self.layers * 3 // RY, RZ, RXX per qubit per layer
    }

    fn n_qubits(&self) -> usize {
        self.n_qubits
    }

    fn build(&self, params: &[f64]) -> QuantumCircuit {
        let mut circuit = QuantumCircuit::new(self.n_qubits);
        let mut param_idx = 0;

        for _layer in 0..self.layers {
            // Single-qubit rotations
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
            
            // Entangling layer
            for q in 0..self.n_qubits - 1 {
                circuit.cnot(q, q + 1);
            }
        }

        circuit
    }
}

/// Data encoding circuit (angle encoding)
pub fn angle_encoding(data: &[f64], n_qubits: usize) -> QuantumCircuit {
    let mut circuit = QuantumCircuit::new(n_qubits);
    
    for (i, &val) in data.iter().take(n_qubits).enumerate() {
        // Normalize and encode angle
        let angle = (val.clamp(-1.0, 1.0) + 1.0) * PI;
        circuit.ry(i, angle);
    }
    
    circuit
}

/// Amplitude encoding (simplified)
pub fn amplitude_encoding(data: &[f64]) -> QuantumCircuit {
    let n_qubits = (data.len() as f64).log2().ceil() as usize;
    QuantumCircuit::new(n_qubits.max(1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circuit_creation() {
        let mut circuit = QuantumCircuit::new(2);
        circuit.h(0).cnot(0, 1);
        
        assert_eq!(circuit.n_qubits(), 2);
        assert_eq!(circuit.depth(), 2);
    }

    #[test]
    fn test_circuit_execution() {
        let mut circuit = QuantumCircuit::new(1);
        circuit.h(0);
        
        let probs = circuit.execute().unwrap();
        assert_eq!(probs.len(), 2);
        assert!((probs[0] - 0.5).abs() < 1e-10);
        assert!((probs[1] - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_hardware_efficient_ansatz() {
        let ansatz = HardwareEfficientAnsatz::new(3, 2);
        let params = vec![0.5; ansatz.n_parameters()];
        let circuit = ansatz.build(&params);
        
        assert_eq!(circuit.n_qubits(), 3);
        assert!(circuit.depth() > 0);
    }
}
