# 🔥 Phoenix Mode: Autonomous Learning System

> **Status:** Design Phase  
> **Part of:** Sprint 9  
> **Built on:** Sprints 1-8 Infrastructure

---

## What is Phoenix Mode?

Phoenix Mode is an **autonomous learning system** that trains AI trading strategies through:

1. **Paper Trading Simulation** - Virtual trading with historical data
2. **RAG Memory** - Learns from past successes and mistakes
3. **LLM Strategist** - Uses AI (Gemini/OpenAI/Claude) to improve decisions
4. **Realistic Graduation** - Proven, achievable criteria for live trading

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    PHOENIX ENGINE v2.0                           │
├─────────────────────────────────────────────────────────────────┤
│  Input: Historical Market Data (TimescaleDB from Sprint 5)      │
│                        ↓                                         │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │  RAG Memory (neurocod-rag from Sprint 5)                │    │
│  │  ├── Winners: What worked in similar situations         │    │
│  │  ├── Mistakes: What to avoid                          │    │
│  │  ├── Regimes: Performance by market condition         │    │
│  │  └── Strategies: Which strategies work when           │    │
│  └─────────────────────────────────────────────────────────┘    │
│                        ↓                                         │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │  LLM Strategist (Gemini/OpenAI/Claude)                  │    │
│  │  Analyzes context → Generates trading decision          │    │
│  └─────────────────────────────────────────────────────────┘    │
│                        ↓                                         │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │  Paper Trading Simulator (IB API from Sprint 6)         │    │
│  │  Executes trade → Records outcome                       │    │
│  └─────────────────────────────────────────────────────────┘    │
│                        ↓                                         │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │  Graduation Assessment (v2.0 Criteria)                  │    │
│  │  Evaluates if ready for next level                      │    │
│  └─────────────────────────────────────────────────────────┘    │
│                        ↓                                         │
│  Output: Trained Strategy + Graduation Level                    │
└─────────────────────────────────────────────────────────────────┘
```

---

## Key Innovation: Realistic Graduation

### ❌ Old (Unrealistic) Approach
```
Goal: €1,000 → €200,000 in 5 years
CAGR Required: 82%
Problem: Practically impossible with certainty
Result: Overfitting, curve-fitting, failure
```

### ✅ New (Realistic) Approach
```
Level 1: Paper Trading  → 15% CAGR target
Level 2: Micro Live     → 20% CAGR target  (€1,000 real)
Level 3: Small Live     → 25% CAGR target  (€5,000 real)
Level 4: Full Strategy  → 30% CAGR target  (€20-50k real)
Level 5: Master Level   → Track record    (External capital)

Comparison:
- Warren Buffett: ~20% CAGR (60 years)
- RenTech: ~66% CAGR (proprietary)
- Our target: 15-30% (realistic & achievable)
```

---

## Graduation Criteria v2.0

### Financial Metrics
| Metric | Minimum | Target |
|--------|---------|--------|
| CAGR | 15% | 20-30% |
| Max Drawdown | < 15% | < 10% |
| Sharpe Ratio | > 1.2 | > 1.5 |
| Calmar Ratio | > 2.0 | > 3.0 |

### Statistical Requirements
| Requirement | Value |
|-------------|-------|
| Min Trades | 100 (statistical significance) |
| Min Profitable Months | 6 |
| Profitable Month % | > 60% |
| Win Rate | > 52% |
| Profit Factor | > 1.3 |

### Risk Management
| Limit | Value |
|-------|-------|
| Max Beta | 0.7 (independence from market) |
| Risk of Ruin | < 1% |
| Daily Loss Limit | 3% |
| Weekly Loss Limit | 5% |

### Testing Requirements
```
In-Sample:     5 years (training)
Out-of-Sample: 2 years (testing!) ← CRITICAL
Walk-Forward:  Rolling windows
Paper Trading: 6 months minimum
Stress Tests:  8 historical crises
```

---

## Stress Test Scenarios

| Crisis | Period | Market Drop |
|--------|--------|-------------|
| COVID-19 Crash | Feb-Mar 2020 | -35% |
| GFC 2008 | 2007-2009 | -57% |
| Dot-Com Bubble | 2000-2002 | -78% |
| Flash Crash | May 6, 2010 | -10% (1 day) |
| Rate Shock | 1994 | -15% bonds |
| Black Monday | Oct 19, 1987 | -22% (1 day) |
| Russia Default | Aug-Oct 1998 | -25% |

**Pass Criteria:**
- Survive > 70% of scenarios
- Max drawdown in crisis < 30%

---

## Automatic Fail Conditions

```rust
enum FailReason {
    OverfittingDetected,           // In-sample >> Out-of-sample
    SuspiciousPerformance,         // CAGR > 50% (likely curve-fitted)
    HighBeta,                      // Beta > 0.7 (just following market)
    HighPayoffRatio,               // Payoff > 5.0 (lottery tickets)
    InsufficientTrades,            // < 100 trades
    FailedStressTest,              // Lost > 30% in crisis
    HighCostImpact,                // Costs eat > 25% of returns
    PoorCrisisPerformance,         // Lost money in all bear markets
}
```

---

## ML API Integrations

| Provider | Model | Use Case | Cost |
|----------|-------|----------|------|
| **Google** | Gemini Pro | SEC filings, sentiment | Free (1K/day) |
| **OpenAI** | GPT-4o-mini | Earnings analysis | $0.15/M tokens |
| **Anthropic** | Claude 3 | 10-K deep dive | $15/M tokens |
| **HuggingFace** | FinBERT | Sentiment scoring | Free |

---

## RAG Memory Integration

Uses existing **neurocod-rag** from Sprint 5:

```rust
pub struct RAGMemory {
    winners: VectorStore,      // pgvector
    mistakes: VectorStore,     // pgvector
    regimes: VectorStore,      // pgvector
    strategies: VectorStore,   // pgvector
}

// Query example
let similar_cases = rag.query(&Query {
    regime: MarketRegime::RiskOn,
    vix: 15.0,
    cq: 0.85,
    portfolio_state: current_state,
    limit: 10,
}).await;

// Returns top 10 similar historical situations
```

---

## File Structure

```
src/
├── phoenix/
│   ├── mod.rs              # PhoenixEngine
│   ├── graduation.rs       # GraduationCriteria v2.0
│   ├── simulator.rs        # PaperTradingSimulator
│   ├── memory.rs           # RAGMemory wrapper
│   └── strategist.rs       # LLMStrategist
├── ml/
│   └── apis/
│       ├── gemini.rs
│       ├── openai.rs
│       ├── claude.rs
│       └── huggingface.rs
└── lib.rs                  # Add phoenix module
```

---

## Configuration

```yaml
# config/phoenix.yaml
phoenix:
  mode: "training"  # training | paper | live
  
  goals:
    initial_capital: 1000.00
    currency: "EUR"
    
  graduation:
    level1_cagr_min: 0.15      # 15%
    max_drawdown: 0.15          # 15%
    min_sharpe: 1.2
    min_trades: 100
    
  llm:
    primary: "gemini"
    fallback: "openai"
    
  costs:
    commission: 1.00
    slippage: 0.001            # 0.1%
```

---

## Database Schema

```sql
-- Uses existing TimescaleDB from Sprint 5

CREATE TABLE phoenix_training_runs (
    id UUID PRIMARY KEY,
    start_date TIMESTAMPTZ,
    initial_capital DECIMAL(15, 2),
    target_cagr DECIMAL(5, 2),
    status VARCHAR(20),
    final_level VARCHAR(30),
    overall_score DECIMAL(5, 2)
);

CREATE TABLE phoenix_experiences (
    id UUID PRIMARY KEY,
    training_run_id UUID REFERENCES phoenix_training_runs(id),
    timestamp TIMESTAMPTZ,
    market_regime VARCHAR(20),
    situation JSONB,
    decision JSONB,
    outcome JSONB,
    lesson TEXT,
    embedding VECTOR(1536)  -- For similarity search
);
```

---

## Success Metrics

- [ ] Complete 100+ trades in < 1 hour simulation
- [ ] RAG successfully retrieves similar cases
- [ ] LLM improves decisions epoch-over-epoch
- [ ] Pass at least 6 of 8 stress scenarios
- [ ] Graduate to Level 1 (Paper Trading)

---

## Dependencies

| Component | From Sprint | Usage |
|-----------|-------------|-------|
| TimescaleDB | 5 | Historical data storage |
| neurocod-rag | 5 | Vector memory |
| IB API Client | 6 | Paper trading execution |
| Order Management | 6 | Trade execution |
| ML Pipeline | 7 | Feature extraction |
| Risk Analytics | 7 | Risk calculations |
| K8s Deployment | 8 | Production hosting |

---

## Next Steps

1. ✅ Document Phoenix Mode (this file)
2. ✅ Create graduation criteria v2.0
3. ⬜ Implement `phoenix/mod.rs`
4. ⬜ Integrate Gemini API
5. ⬜ Build paper trading simulator
6. ⬜ Create training loop
7. ⬜ Add stress testing
8. ⬜ Build graduation dashboard

---

## References

- [Sprint 9 Specification](../specs/SPRINT-9.md)
- [v2.0 Roadmap](../ROADMAP-v2.0.md)
- [Graduation Criteria](../../src/phoenix/graduation.rs)

---

**Phoenix Mode: Rise from the ashes of failed trades, stronger and wiser.** 🔥
