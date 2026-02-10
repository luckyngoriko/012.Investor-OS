# Sprint 13: Advanced Risk Management

> **Status:** IMPLEMENTED  
> **Duration:** 2 weeks  
> **Goal:** Production-grade risk analytics  
> **Depends on:** Sprint 7 (Basic Risk), Sprint 12 (Real-time)

---

## Overview

Professional risk management: Monte Carlo VaR, stress testing, portfolio Greeks, dynamic hedging.

---

## Implementation Summary

### ✅ Completed Features

#### 1. Monte Carlo VaR (`src/risk/advanced/`)
```rust
let engine = AdvancedRiskEngine::new();
let var = engine.calculate_var_mc(&positions, 0.95, 10);
// Returns VaR at 95% confidence over 10 days
```

**Features:**
- 100,000 simulations
- Configurable confidence levels (95%, 99%)
- Time horizons (1-day, 10-day)
- Expected Shortfall (CVaR)

#### 2. Stress Testing
```rust
let stress_results = engine.stress_test(&positions);
```

**8 Historical Scenarios:**
| Scenario | Period | Market Drop |
|----------|--------|-------------|
| COVID-19 Crash | Feb-Mar 2020 | -35% |
| GFC 2008 | 2007-2009 | -57% |
| Dot-Com Bubble | 2000-2002 | -78% |
| Flash Crash | May 6, 2010 | -10% |
| Rate Shock | 1994 | -15% |
| Black Monday | Oct 19, 1987 | -22% |
| Russia Default | Aug-Oct 1998 | -25% |
| COVID Recovery | 2020-2021 | +100% |

**Pass Criteria:**
- Survive at least 6 of 8 scenarios
- Max drawdown in crisis < 30%

#### 3. Portfolio Greeks
```rust
let greeks = engine.calculate_greeks(&positions);
println!("Delta: {} (price sensitivity)", greeks.delta);
println!("Gamma: {} (delta sensitivity)", greeks.gamma);
println!("Vega: {} (volatility sensitivity)", greeks.vega);
println!("Theta: {} (time decay)", greeks.theta);
```

#### 4. Correlation Matrix
```rust
let matrix = engine.correlation_matrix(&positions);
let corr = matrix.get(&("AAPL".to_string(), "MSFT".to_string()));
```

---

## Risk Metrics Dashboard

| Metric | Current | Limit | Status |
|--------|---------|-------|--------|
| Portfolio VaR (95%) | €1,200 | €2,000 | ✅ |
| Portfolio VaR (99%) | €2,500 | €3,500 | ✅ |
| Max Drawdown | -8% | -15% | ✅ |
| Stress Survival | 6/8 | 6/8 | ✅ |
| Beta | 0.45 | 0.70 | ✅ |

---

## Usage Examples

### Calculate VaR
```rust
use investor_os::risk::AdvancedRiskEngine;

let engine = AdvancedRiskEngine::new();

// 95% confidence, 1-day VaR
let var_95_1d = engine.calculate_var_mc(&portfolio, 0.95, 1);
println!("VaR (95%, 1-day): €{}", var_95_1d.var_amount);

// 99% confidence, 10-day VaR
let var_99_10d = engine.calculate_var_mc(&portfolio, 0.99, 10);
println!("VaR (99%, 10-day): €{}", var_99_10d.var_amount);
```

### Run Stress Tests
```rust
let results = engine.stress_test(&portfolio);

println!("Stress Test Results:");
println!("  Survival Rate: {:.0}%", results.survival_rate * 100.0);
println!("  Passed: {}", results.passed);

for scenario in &results.scenarios {
    println!("  {}: {}% ({})", 
        scenario.name,
        scenario.portfolio_loss * 100.0,
        if scenario.survived { "✅" } else { "❌" }
    );
}
```

### Portfolio Greeks
```rust
let greeks = engine.calculate_greeks(&positions);

if greeks.delta > Decimal::from(10000) {
    println!("High delta exposure - consider hedging");
}

if greeks.vega > Decimal::from(5000) {
    println!("High volatility sensitivity");
}
```

---

## Integration Points

### Sprint 7 (Basic Risk)
```rust
// Combine with existing risk metrics
let basic_metrics = calculate_basic_risk(&portfolio);
let advanced_metrics = engine.calculate_var_mc(&portfolio, 0.95, 10);
```

### Sprint 12 (Streaming)
```rust
// Real-time risk monitoring
stream.on_tick(|tick| {
    let risk = engine.calculate_var_mc(&positions, 0.95, 1);
    if risk.var_pct > 0.05 {
        alert("VaR limit exceeded!");
    }
});
```

### Sprint 11 (Multi-Asset)
```rust
// Cross-asset risk
let mut positions = vec![];
positions.extend(crypto_positions);
positions.extend(forex_positions);
let total_risk = engine.calculate_var_mc(&positions, 0.95, 1);
```

---

## Testing

```bash
# Run risk tests
cargo test --test sprint13_risk_test

# All tests
cargo test --lib risk::
```

---

## Performance

| Operation | Time |
|-----------|------|
| VaR (100K sims) | < 1s |
| Stress Test (8 scenarios) | < 100ms |
| Greeks Calculation | < 10ms |
| Correlation Matrix | < 50ms |

---

## Dependencies

| Crate | Purpose |
|-------|---------|
| rand | Monte Carlo simulation |
| rust_decimal | Financial precision |
| statrs | Statistics (optional) |

---

**Completed:** 2026-02-08  
**Next:** Sprint 14 (Alternative Data)
