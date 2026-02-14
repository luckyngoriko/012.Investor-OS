# Sprint 31: ML Strategy Selector

## Overview
Intelligent strategy selection based on market regime detection using ML models.

## Features

### Market Regime Detection
- **Trending Markets**: Momentum strategies
- **Ranging Markets**: Mean reversion
- **Volatile Markets**: Breakout strategies
- **Low Volatility**: Income strategies

### Strategy Recommendations
```rust
pub struct StrategyRecommendation {
    pub strategy_type: StrategyType,
    pub confidence: f64,
    pub expected_return: f64,
    pub risk_level: RiskLevel,
    pub rationale: String,
}
```

### Risk Tolerance Configuration
- Conservative (max 5% drawdown)
- Moderate (max 15% drawdown)
- Aggressive (max 30% drawdown)

### Performance Attribution
- Tracks which strategies work in which regimes
- Continuous learning from outcomes
- Strategy switching prevention (hysteresis)

## API Endpoints
```
GET /api/ml/strategy/recommend
POST /api/ml/strategy/select
GET /api/ml/strategy/performance
```

## Tests
- 9 Golden Path tests passing
