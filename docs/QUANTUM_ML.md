# Quantum Machine Learning (Sprint 51)

Quantum computing integration for Investor OS using the roqoqo framework.

## Overview

The Quantum ML module provides quantum algorithms for portfolio optimization and
molecular simulations, with both simulator and hardware backend support.

## Features

### QAOA (Quantum Approximate Optimization Algorithm)
- **Max-Cut**: Graph partitioning for portfolio clustering
- **Portfolio Optimization**: Asset selection with risk constraints
- **Knapsack Problem**: Resource allocation optimization

### VQE (Variational Quantum Eigensolver)
- **Molecular Simulation**: H2 ground state energy calculation
- **Ising Model**: Spin system simulations
- **Custom Hamiltonians**: User-defined quantum systems

### Hardware Support
- **Simulator**: CPU-based simulation (up to 20 qubits)
- **IBM Quantum**: Cloud quantum hardware (127+ qubits)
- **Automatic Fallback**: Simulator when hardware unavailable

## API Endpoints

### GET /api/v1/quantum/status
Check quantum backend availability.

```json
{
  "available": true,
  "backend": "simulator",
  "max_qubits": 20,
  "modules": ["QAOA", "VQE", "Variational Circuits"]
}
```

### POST /api/v1/quantum/optimize
Run QAOA optimization.

**Request (MaxCut):**
```json
{
  "problem_type": "maxcut",
  "n_qubits": 4,
  "layers": 2,
  "edges": [[0, 1, 1.0], [1, 2, 1.0], [2, 3, 1.0]]
}
```

**Response:**
```json
{
  "success": true,
  "best_solution": [true, false, true, false],
  "best_cost": -3.0,
  "approximation_ratio": 0.95,
  "iterations": 50,
  "quantum_time_ms": 125.4
}
```

### POST /api/v1/quantum/vqe
Run variational quantum eigensolver.

**Request:**
```json
{
  "molecule": "h2",
  "n_qubits": 2,
  "layers": 2,
  "max_iterations": 100
}
```

**Response:**
```json
{
  "success": true,
  "ground_state_energy": -1.137,
  "iterations": 45,
  "converged": true,
  "quantum_time_ms": 234.5
}
```

### POST /api/v1/quantum/benchmark
Run performance benchmark.

```json
{
  "quantum_time_ms": 12.5,
  "classical_time_ms": 31.25,
  "speedup": 2.5,
  "circuit_depth": 24,
  "n_qubits": 4
}
```

## Usage Example

```rust
use investor_os::research::quantum::{
    QAOAOptimizer, OptimizationProblem,
    BackendFactory, VQE, VQEConfig,
};

// QAOA for portfolio optimization
let optimizer = QAOAOptimizer::new(
    2,  // layers
    5,  // qubits
    BackendFactory::simulator(),
);

let problem = OptimizationProblem::Portfolio {
    expected_returns: vec![0.1, 0.15, 0.08, 0.12, 0.09],
    covariance: vec![vec![1.0; 5]; 5],
    budget: 3,
    risk_aversion: 1.0,
};

let result = optimizer.solve(&problem, 50)?;
println!("Selected assets: {:?}", result.best_solution);

// VQE for molecular simulation
let config = VQEConfig {
    n_qubits: 2,
    n_layers: 2,
    max_iterations: 100,
    tolerance: 1e-6,
    learning_rate: 0.01,
};

let vqe = VQE::new(config, BackendFactory::simulator());
let hamiltonian = h2_hamiltonian();
let result = vqe.optimize(&hamiltonian)?;
println!("Ground state energy: {}", result.ground_state_energy);
```

## Architecture

```
┌─────────────────────────────────────────────┐
│           Quantum ML Module                 │
├─────────────────────────────────────────────┤
│  QAOA        │  VQE         │  Circuits     │
│  ├─ MaxCut   │  ├─ H2       │  ├─ Ansatz    │
│  ├─ Portfolio│  ├─ Ising    │  ├─ Encoding  │
│  └─ Knapsack │  └─ Custom   │  └─ Execution │
├─────────────────────────────────────────────┤
│           Quantum Backend                   │
│  ├─ Simulator (CPU)                         │
│  └─ IBM Quantum (Cloud)                     │
└─────────────────────────────────────────────┘
```

## Configuration

Add to `Cargo.toml`:
```toml
[dependencies]
roqoqo = "1.14"
roqoqo-quest = "0.14"
num-complex = "0.4"
fastrand = "2.3"
```

## Testing

```bash
# Run quantum tests
cargo test --lib quantum

# Run all tests
cargo test --lib
```

## Roadmap

- [x] QAOA implementation
- [x] VQE implementation
- [x] Simulator backend
- [x] REST API endpoints
- [ ] IBM Quantum hardware integration
- [ ] Quantum advantage benchmarking
- [ ] Hybrid quantum-classical models
- [ ] Real-time quantum streaming

## References

- [roqoqo Documentation](https://github.com/HQSquantumsimulations/qoqo)
- [Qiskit Textbook](https://qiskit.org/textbook/)
- [Quantum Approximate Optimization Algorithm](https://arxiv.org/abs/1411.4028)
- [Variational Quantum Eigensolver](https://arxiv.org/abs/1304.3061)
