# Sprint 37: Synthetic Training Data Generation

## Overview
Generate realistic synthetic training data for HRM model to enable rapid iteration without waiting for years of historical data collection.

## Goals
- Generate 10,000+ synthetic training samples
- Cover all market regimes (Bull, Bear, Sideways, Crisis)
- Add realistic noise and correlations
- Create validation and test splits

## Implementation

### Data Generator
```rust
// src/hrm/data/synthetic.rs
pub struct SyntheticDataGenerator {
    regimes: Vec<MarketRegime>,
    noise_level: f32,
    correlation_matrix: Array2<f32>,
}

impl SyntheticDataGenerator {
    pub fn generate(&self, n_samples: usize) -> Vec<TrainingSample> {
        // Generate realistic market scenarios
    }
}
```

### Training Sample Structure
```rust
pub struct TrainingSample {
    pub input: HRMInput,
    pub target: HRMOutput,
    pub regime: MarketRegime,
    pub metadata: SampleMetadata,
}
```

### Data Augmentation
- Gaussian noise injection
- Feature scaling variations
- Temporal shifts
- Regime transitions

## Test Coverage
- 15 new tests for data generation
- Validation of statistical properties
- Correlation matrix verification

## Status: ✅ COMPLETE

- [x] Synthetic data generator
- [x] All market regimes covered
- [x] 10,000+ samples generated
- [x] Train/validation/test splits
- [x] Tests passing

---
**Prev**: Sprint 36 - HRM Native Engine  
**Next**: Sprint 38 - LSTM Architecture
