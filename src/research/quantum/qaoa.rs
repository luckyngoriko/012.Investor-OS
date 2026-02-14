//! Quantum Approximate Optimization Algorithm (QAOA)
//!
//! For portfolio optimization and combinatorial problems

use super::{QuantumCircuit, VariationalCircuit, Result};
use super::backend::{QuantumBackend, ExecutionResult};

/// QAOA optimizer for combinatorial problems
pub struct QAOAOptimizer {
    layers: usize,
    n_qubits: usize,
    backend: Box<dyn QuantumBackend>,
}

/// Optimization problem type
#[derive(Debug, Clone)]
pub enum OptimizationProblem {
    /// Max-Cut problem (graph partitioning)
    MaxCut { edges: Vec<(usize, usize, f64)> },
    
    /// Portfolio optimization
    Portfolio {
        expected_returns: Vec<f64>,
        covariance: Vec<Vec<f64>>,
        budget: usize,
        risk_aversion: f64,
    },
    
    /// Knapsack problem
    Knapsack {
        values: Vec<f64>,
        weights: Vec<f64>,
        capacity: f64,
    },
}

/// QAOA result
#[derive(Debug, Clone)]
pub struct QAOAResult {
    pub best_solution: Vec<bool>,
    pub best_cost: f64,
    pub cost_history: Vec<f64>,
    pub optimal_params: Vec<f64>,
    pub approximation_ratio: f64,
}

impl QAOAOptimizer {
    /// Create new QAOA optimizer
    pub fn new(
        layers: usize,
        n_qubits: usize,
        backend: Box<dyn QuantumBackend>,
    ) -> Self {
        Self {
            layers,
            n_qubits,
            backend,
        }
    }

    /// Solve optimization problem
    pub fn solve(&self, problem: &OptimizationProblem, max_iter: usize) -> Result<QAOAResult> {
        let n_params = self.layers * 2; // gamma and beta for each layer
        
        // Initialize parameters
        let mut params: Vec<f64> = (0..n_params)
            .map(|i| if i % 2 == 0 { 0.1 } else { 0.5 }) // gamma=0.1, beta=0.5
            .collect();

        let mut cost_history = Vec::with_capacity(max_iter);
        let mut best_cost = f64::INFINITY;
        let mut best_params = params.clone();

        // Classical optimization loop
        for _iter in 0..max_iter {
            let cost = self.evaluate(&params, problem)?;
            cost_history.push(cost);

            if cost < best_cost {
                best_cost = cost;
                best_params = params.clone();
            }

            // Simple gradient-free update
            params = self.update_params(&params, problem)?;
        }

        // Final execution with best parameters
        let circuit = self.build_circuit(&best_params);
        let result = self.backend.execute(&circuit, 1024)?;
        
        let best_solution = self.extract_solution(&result);
        let final_cost = problem.cost(&best_solution);

        // Calculate approximation ratio
        let classical_best = problem.solve_classical();
        let approximation_ratio = if classical_best.1 != 0.0 {
            final_cost.abs() / classical_best.1.abs()
        } else {
            1.0
        };

        Ok(QAOAResult {
            best_solution,
            best_cost: final_cost,
            cost_history,
            optimal_params: best_params,
            approximation_ratio,
        })
    }

    fn evaluate(&self, params: &[f64], problem: &OptimizationProblem) -> Result<f64> {
        let circuit = self.build_circuit(params);
        let result = self.backend.execute(&circuit, 1024)?;
        
        // Calculate expectation value of cost Hamiltonian
        let mut cost = 0.0;
        for measurement in &result.measurements {
            let bits: Vec<bool> = measurement
                .bitstring
                .chars()
                .map(|c| c == '1')
                .collect();
            cost += problem.cost(&bits) * measurement.probability;
        }
        
        Ok(cost)
    }

    fn update_params(&self, params: &[f64], problem: &OptimizationProblem) -> Result<Vec<f64>> {
        let eps = 0.01;
        let learning_rate = 0.05;
        
        let mut new_params = params.to_vec();
        let current_cost = self.evaluate(params, problem)?;

        for i in 0..params.len() {
            let mut perturbed = params.to_vec();
            perturbed[i] += eps;
            let perturbed_cost = self.evaluate(&perturbed, problem)?;
            
            let gradient = (perturbed_cost - current_cost) / eps;
            new_params[i] -= learning_rate * gradient;
            
            // Constrain parameters
            if i % 2 == 0 {
                // Gamma: typically [0, 2*pi]
                new_params[i] = new_params[i].clamp(0.0, 2.0 * std::f64::consts::PI);
            } else {
                // Beta: typically [0, pi]
                new_params[i] = new_params[i].clamp(0.0, std::f64::consts::PI);
            }
        }

        Ok(new_params)
    }

    fn build_circuit(&self, params: &[f64]) -> QuantumCircuit {
        let mut circuit = QuantumCircuit::new(self.n_qubits);
        
        // Initial state: superposition
        for q in 0..self.n_qubits {
            circuit.h(q);
        }

        // QAOA layers
        for layer in 0..self.layers {
            let gamma = params[layer * 2];
            let beta = params[layer * 2 + 1];
            
            // Cost Hamiltonian evolution (simplified - uniform mixing)
            for q in 0..self.n_qubits {
                circuit.rz(q, gamma);
            }
            
            // Mixer Hamiltonian evolution
            for q in 0..self.n_qubits {
                circuit.rx(q, 2.0 * beta);
            }
        }

        circuit
    }

    fn extract_solution(&self, result: &ExecutionResult) -> Vec<bool> {
        // Return most frequent measurement
        if let Some(best) = result.measurements.iter().max_by_key(|m| m.count) {
            best.bitstring.chars().map(|c| c == '1').collect()
        } else {
            vec![false; self.n_qubits]
        }
    }
}

impl OptimizationProblem {
    /// Calculate cost for a given solution
    pub fn cost(&self, solution: &[bool]) -> f64 {
        match self {
            OptimizationProblem::MaxCut { edges } => {
                let mut cut_weight = 0.0;
                for (i, j, w) in edges {
                    if solution.get(*i) != solution.get(*j) {
                        cut_weight += w;
                    }
                }
                -cut_weight // Minimize negative cut weight
            }
            
            OptimizationProblem::Portfolio { expected_returns, covariance, budget, risk_aversion } => {
                let selected: Vec<usize> = solution.iter()
                    .enumerate()
                    .filter(|(_, &s)| s)
                    .map(|(i, _)| i)
                    .collect();
                
                if selected.len() != *budget {
                    return 1e10; // Penalty for wrong budget
                }
                
                let returns: f64 = selected.iter()
                    .map(|&i| -expected_returns[i]) // Negative for minimization
                    .sum();
                
                let mut risk = 0.0;
                for &i in &selected {
                    for &j in &selected {
                        risk += covariance[i][j];
                    }
                }
                
                returns + risk_aversion * risk
            }
            
            OptimizationProblem::Knapsack { values, weights, capacity } => {
                let total_weight: f64 = solution.iter()
                    .enumerate()
                    .filter(|(_, &s)| s)
                    .map(|(i, _)| weights[i])
                    .sum();
                
                if total_weight > *capacity {
                    return 1e10; // Penalty for exceeding capacity
                }
                
                let total_value: f64 = solution.iter()
                    .enumerate()
                    .filter(|(_, &s)| s)
                    .map(|(i, _)| -values[i]) // Negative for minimization
                    .sum();
                
                total_value
            }
        }
    }

    /// Classical solver for baseline comparison
    pub fn solve_classical(&self) -> (Vec<bool>, f64) {
        match self {
            OptimizationProblem::MaxCut { edges } => {
                // Greedy max-cut
                let n = edges.iter().map(|(i, j, _)| i.max(j) + 1).max().unwrap_or(0);
                let mut best_solution = vec![false; n];
                let mut best_cost = f64::INFINITY;
                
                // Simple local search
                for _ in 0..100 {
                    let solution: Vec<bool> = (0..n)
                        .map(|_| fastrand::bool())
                        .collect();
                    
                    let cost = self.cost(&solution);
                    if cost < best_cost {
                        best_cost = cost;
                        best_solution = solution;
                    }
                }
                
                (best_solution, best_cost)
            }
            _ => (vec![], 0.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::backend::SimulatorBackend;

    #[test]
    fn test_maxcut_problem() {
        let edges = vec![
            (0, 1, 1.0),
            (1, 2, 1.0),
            (2, 0, 1.0),
        ];
        let problem = OptimizationProblem::MaxCut { edges };
        
        let solution = vec![true, false, true];
        let cost = problem.cost(&solution);
        
        // Should cut edges (0,1) and (1,2) = weight 2
        assert!((cost + 2.0).abs() < 1e-6);
    }

    #[test]
    fn test_qaoa_optimizer() {
        let backend = Box::new(SimulatorBackend::with_qubits(3));
        let qaoa = QAOAOptimizer::new(2, 3, backend);
        
        let edges = vec![(0, 1, 1.0), (1, 2, 1.0)];
        let problem = OptimizationProblem::MaxCut { edges };
        
        let result = qaoa.solve(&problem, 10);
        assert!(result.is_ok());
        
        let result = result.unwrap();
        assert_eq!(result.best_solution.len(), 3);
    }
}
