# Sprint 42: HRM Strategy Selector Integration

## Overview
Integrate HRM model with StrategySelectorEngine for ML-based conviction calculation and strategy selection.

## Goals
- Add HRM to StrategySelectorEngine
- Implement calculate_conviction() with HRM fallback
- Connect regime detection to strategy selection
- Maintain backward compatibility

## Implementation

### StrategySelectorEngine Extension
```rust
// src/strategy_selector/mod.rs
pub struct StrategySelectorEngine {
    // ... existing fields
    hrm: Option<HRM>,  // NEW
}

impl StrategySelectorEngine {
    pub fn with_hrm_weights(mut self, path: &str) -> Result<Self> {
        self.hrm = Some(HRM::new(...)?);
        Ok(self)
    }
    
    pub fn calculate_conviction(&self, signals: &HRMInputSignals) -> ConvictionResult {
        if let Some(ref hrm) = self.hrm {
            // Try ML inference first
            match hrm.infer(...) {
                Ok(result) => result,
                Err(_) => self.heuristic_fallback(signals),
            }
        } else {
            // Heuristic fallback
            self.heuristic_fallback(signals)
        }
    }
}
```

### ConvictionResult
```rust
pub struct ConvictionResult {
    pub conviction: f32,           // 0.0 - 1.0
    pub confidence: f32,           // 0.0 - 1.0
    pub regime: MarketRegime,
    pub source: ConvictionSource,  // MLModel or Heuristic
}
```

## Status: ✅ COMPLETE

- [x] HRM integrated in engine
- [x] ML + heuristic fallback
- [x] Strategy selection based on regime
- [x] 5 integration tests passing
- [x] Backward compatible

---
**Prev**: Sprint 41 - Golden Dataset  
**Next**: Sprint 43 - REST API
