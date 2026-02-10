# Sprint 9: Phoenix Mode & v2.0 Architecture Design

> **Status:** IMPLEMENTED  
> **Date:** 2026-02-08  
> **Goal:** Document all discussed v2.0 features and Phoenix autonomous learning system  
> **Depends on:** Sprints 1-8 (All completed infrastructure)

---

## Executive Summary

Този спринт документира всички идеи и архитектурни решения, дискутирани за Investor OS v2.0. Всичко се основава на изградената инфраструктура от Sprints 1-8:

- ✅ **Sprints 1-4:** Core infrastructure, signals (QVM), CQ Engine, Web UI
- ✅ **Sprint 5:** PostgreSQL + RAG (neurocod-rag)
- ✅ **Sprint 6:** Interactive Brokers integration
- ✅ **Sprint 7:** Analytics, Backtesting, ML Pipeline (19 features)
- ✅ **Sprint 8:** Kubernetes, CI/CD, Production hardening

---

## 1. Phoenix Mode: Autonomous Learning System

### 1.1 Concept

Система за автономно обучение на AI трейдинг стратегии чрез:
- **Paper Trading Simulation** с виртуален портфейл
- **RAG Memory** (от Sprint 5) за запазване на знания
- **LLM Strategist** (Gemini/OpenAI/Claude) за анализ и подобрение
- **Goal-Based Training** с реалистични критерии

### 1.2 Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    PHOENIX ENGINE v2.0                           │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐      │
│  │  Historical  │───▶│  RAG Memory  │───▶│   LLM        │      │
│  │  Data Store  │    │  (neurocod)  │    │  Strategist  │      │
│  │  (Sprint 5)  │    │  (Sprint 5)  │    │  (Gemini)    │      │
│  └──────────────┘    └──────────────┘    └──────────────┘      │
│         │                   │                   │               │
│         ▼                   ▼                   ▼               │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │              Paper Trading Simulator                      │  │
│  │         (Uses IB API from Sprint 6)                       │  │
│  └──────────────────────────────────────────────────────────┘  │
│         │                                                       │
│         ▼                                                       │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │              Graduation Assessment                        │  │
│  │    (Realistic criteria - NOT 200x in 5 years)            │  │
│  └──────────────────────────────────────────────────────────┘  │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### 1.3 Key Components

#### A. RAG Memory Integration
```rust
// Използва съществуващата neurocod-rag инфраструктура от Sprint 5
pub struct RAGMemory {
    // Векторна база данни (pgvector)
    winners: VectorStore,      // Какво работи
    mistakes: VectorStore,     // Какво НЕ работи  
    regimes: VectorStore,      // Какво работи кога
    strategies: VectorStore,   // Стратегии и тяхната ефективност
}
```

#### B. LLM API Integrations
| Provider | Use Case | Sprint 5 Base |
|----------|----------|---------------|
| **Google Gemini** | SEC analysis, sentiment | ✅ neurocod-rag ready |
| **OpenAI GPT-4o** | Earnings call analysis | ✅ HTTP client exists |
| **Anthropic Claude** | 10-K deep analysis | ✅ HTTP client exists |
| **HuggingFace** | FinBERT embeddings | ✅ Existing integration |

#### C. Paper Trading Simulator
```rust
// Използва IB API client от Sprint 6
pub struct PaperTradingSimulator {
    market_data: Arc<MarketDataCollector>, // от Sprint 1-2
    execution_engine: Box<dyn Execution>,   // от Sprint 6
    portfolio: VirtualPortfolio,
}
```

### 1.4 Training Loop

```rust
impl PhoenixEngine {
    pub async fn run_training_epoch(&mut self) -> TrainingResult {
        // 1. Зареждаме исторически данни (TimescaleDB от Sprint 5)
        let market_data = self.load_historical_data().await;
        
        // 2. За всеки ден от миналото
        for day in market_data {
            // 3. Query RAG: "Какво работи в този режим?"
            let context = self.memory.query_similar_cases(&day).await;
            
            // 4. LLM анализира и предлага действие
            let decision = self.strategist.decide(&context).await;
            
            // 5. Изпълняваме виртуално (IB paper API)
            let outcome = self.simulator.execute(&decision).await;
            
            // 6. Запазваме в RAG
            self.memory.store_experience(&day, &decision, &outcome).await;
        }
        
        // 7. Оценяваме с подобрените критерии
        self.assess_graduation()
    }
}
```

---

## 2. Phoenix Graduation Criteria v2.0

### 2.1 Проблем със старите критерии

**Стар критерий (НЕПРАВИЛЕН):**
- 1000€ → 200,000€ за 5 години = 82% CAGR
- Това е ПРАКТИЧЕСКИ НЕВЪЗМОЖНО със сигурност

**Сравнение:**
| Trader | CAGR | Period |
|--------|------|--------|
| Warren Buffett | 20% | 60 years |
| Renaissance Tech | 66% | ~30 years (secretive) |
| Average Hedge Fund | 8-12% | - |
| **Old Target** | **82%** | **5 years** ❌ |

### 2.2 Нови реалистични критерии

#### A. CAGR Targets по нива
```rust
pub struct CagrTargets {
    pub level1_min: 0.15,        // 15% - Paper Trading
    pub level2_target: 0.20,     // 20% - Micro Live  
    pub level3_target: 0.25,     // 25% - Small Live
    pub level4_optimal: 0.30,    // 30% - Full Strategy
    pub max_suspicious: 0.50,    // 50% - над това = overfitting
}
```

#### B. Risk Limits
```rust
pub struct RiskLimits {
    pub max_drawdown: 15%,       // Беше 20%, сега по-консервативно
    pub max_daily_loss: 3%,
    pub max_weekly_loss: 5%,
    pub max_risk_of_ruin: 1%,
    pub max_beta: 0.7,           // Независимост от пазара
    pub min_calmar: 2.0,         // CAGR / MaxDD
}
```

#### C. Statistical Requirements
```rust
pub struct StatisticalRequirements {
    pub min_total_trades: 100,           // Статистическа значимост
    pub min_trades_per_month: 4,
    pub min_profitable_months: 6,
    pub min_profitable_month_pct: 60%,
    pub min_sharpe: 1.2,
    pub min_sortino: 1.8,
    pub min_win_rate: 52%,
    pub min_profit_factor: 1.3,
    pub max_payoff_ratio: 5.0,           // Не lottery tickets
}
```

#### D. Out-of-Sample Testing
```rust
pub struct PeriodConfig {
    pub in_sample_years: 5,              // Тренировка
    pub out_of_sample_years: 2,          // Тест!
    pub walk_forward_train_months: 24,
    pub walk_forward_test_months: 6,
    pub min_paper_trading_months: 6,     // Реално време
}
```

#### E. Cost Modeling
```rust
pub struct CostModel {
    pub commission_per_trade: $1.00,
    pub slippage_pct: 0.1%,
    pub spread_pct: 0.05%,
    pub market_impact_factor: 0.01%,
    pub borrow_cost_annual: 3%,
}
```

### 2.3 Graduation Levels (Постепенно)

```
Level 0: Not Ready
   ↓ (минимум 100 трейда, 6 месеца)
Level 1: Paper Trading
   - Virtual €1000-10000
   - 6 месеца тест
   ↓ (3 месеца печалба)
Level 2: Micro Live  
   - Real €1000
   - Max position €100
   - Max daily loss €50
   ↓ (6 месеца profit)
Level 3: Small Live
   - Real €5000  
   - Max position €500
   - Portfolio heat 20%
   ↓ (12 месеца profit)
Level 4: Full Strategy
   - Real €20-50k
   - 30% CAGR target
   ↓ (3 години track record)
Level 5: Master Level
   - Може да управлява външен капитал
```

### 2.4 Stress Testing

Използва исторически данни от колекторите (Sprints 1-2):

| Scenario | Period | Market Return |
|----------|--------|---------------|
| COVID-19 Crash | Feb-Mar 2020 | -35% |
| GFC 2008 | 2007-2009 | -57% |
| Dot-Com Crash | 2000-2002 | -78% |
| Flash Crash 2010 | May 6, 2010 | -10% in 1 day |
| Rate Shock 1994 | 1994 | -15% bonds |
| Black Monday 1987 | Oct 19, 1987 | -22% in 1 day |

**Pass Criteria:**
- Survival rate > 70%
- Max drawdown in crisis < 30%

### 2.5 Automatic Fail Conditions

```rust
pub enum FailReason {
    // Overfitting
    OverfittingDetected { in_sample: 80%, out_of_sample: 10% },
    
    // Unrealistic performance
    SuspiciousPerformance { cagr: 100%, reason: "Likely curve-fitted" },
    
    // Market following (not alpha)
    HighBeta { beta: 0.9, max: 0.7 },
    
    // Lottery ticket strategy
    HighPayoffRatio { ratio: 8.0, max: 5.0 },
    
    // Insufficient data
    InsufficientTrades { current: 30, required: 100 },
    
    // Crisis performance
    FailedStressTest { scenario: "GFC 2008", loss: 50% },
    
    // Costs too high
    HighCostImpact { cost_pct: 25% },
}
```

---

## 3. ML API Integrations

### 3.1 Implementation Plan

```rust
// src/ml/apis/mod.rs - NEW MODULE

pub mod gemini;      // Google AI
pub mod openai;      // GPT-4o
pub mod claude;      // Anthropic
pub mod huggingface; // Open source models

// Uses existing:
// - HTTP client from Sprint 6 (IB API)
// - Serialization from core models
// - Error handling from anyhow/thiserror
```

### 3.2 Feature Matrix

| Provider | Model | Use Case | Cost | Rate Limit |
|----------|-------|----------|------|------------|
| Google | Gemini Pro | SEC filings, sentiment | Free (1K req/day) | 1000/day |
| OpenAI | GPT-4o-mini | Earnings analysis | $0.15/M tokens | High |
| Anthropic | Claude 3 | 10-K deep dive | $15/M tokens | Medium |
| HuggingFace | FinBERT | Sentiment scoring | Free | Rate limited |

### 3.3 RAG Integration Points

```rust
// Query към съществуващата RAG система (Sprint 5)

pub struct RAGQuery {
    pub regime: MarketRegime,           // от Sprint 2
    pub indicators: TechnicalIndicators, // от Sprint 3
    pub portfolio_state: PortfolioState, // от Sprint 6
    pub limit: usize,
}

pub struct RAGExperience {
    pub situation: MarketCondition,
    pub decision: TradeDecision,
    pub outcome: TradeOutcome,
    pub lesson: String,                 // LLM generated
}
```

---

## 4. v2.0 Roadmap (225 Ideas Documented)

### 4.1 Categories Overview

| Category | Count | Priority |
|----------|-------|----------|
| Multi-Asset | 10 | P1 |
| AI/ML | 15 | P0 |
| Real-Time | 10 | P1 |
| Risk Management | 15 | P0 |
| Alternative Data | 15 | P2 |
| Social Trading | 10 | P3 |
| Mobile & UX | 15 | P2 |
| DeFi | 15 | P2 |
| Compliance | 10 | P2 |
| Automation | 15 | P1 |
| Global Markets | 15 | P3 |
| Analytics | 15 | P1 |
| Gamification | 15 | P3 |
| Infrastructure | 15 | P2 |
| Experimental | 15 | P4 |
| **Total** | **225** | |

### 4.2 Top Priorities for Implementation

**Phase 1 (Q1 2026):**
1. ✅ Phoenix Mode Core (this sprint)
2. ✅ ML API Integration (Gemini/OpenAI)
3. ✅ Enhanced Graduation Criteria
4. Crypto Trading (Binance)

**Phase 2 (Q2 2026):**
5. Real-time streaming (WebSocket)
6. Advanced ML models (LSTM, XGBoost)
7. Alternative data (News NLP)

**Phase 3 (Q3 2026):**
8. Mobile app (React Native)
9. DeFi integration
10. Global markets expansion

### 4.3 Dependencies on Existing Infrastructure

```
v2.0 Feature                    Depends on Sprint
─────────────────────────────────────────────────
Phoenix Mode                    5 (RAG), 6 (IB API), 7 (ML)
ML APIs                         6 (HTTP client), 5 (RAG)
Crypto Trading                  6 (Order management)
Real-time Streaming             7 (Feature pipeline)
Advanced Risk                   7 (Risk analytics)
DeFi                            6 (Wallet integration)
Mobile App                      4 (Next.js API)
```

---

## 5. Implementation Details

### 5.1 File Structure

```
src/
├── phoenix/                    # NEW
│   ├── mod.rs                  # Phoenix engine
│   ├── graduation.rs           # Graduation criteria v2.0
│   ├── simulator.rs            # Paper trading simulator
│   ├── memory.rs               # RAG memory wrapper
│   └── strategist.rs           # LLM strategist
├── ml/
│   └── apis/                   # NEW
│       ├── mod.rs
│       ├── gemini.rs
│       ├── openai.rs
│       ├── claude.rs
│       └── huggingface.rs
├── domain/
│   └── graduation.rs           # Types (moved from phoenix)
└── lib.rs                      # Add phoenix module
```

### 5.2 Configuration

```yaml
# config/phoenix.yaml
phoenix:
  enabled: true
  mode: "training"  # training, paper, live
  
  goals:
    initial_capital: 1000.00
    currency: "EUR"
    
  graduation:
    level1_cagr_min: 0.15
    level2_cagr_min: 0.20
    level3_cagr_min: 0.25
    max_drawdown: 0.15
    min_sharpe: 1.2
    min_trades: 100
    
  llm:
    primary: "gemini"
    fallback: "openai"
    gemini_api_key: "${GEMINI_API_KEY}"
    openai_api_key: "${OPENAI_API_KEY}"
    
  rag:
    vector_store: "pgvector"  # Existing from Sprint 5
    embedding_model: "text-embedding-3-large"
    top_k: 10
    
  costs:
    commission: 1.00
    slippage: 0.001
    spread: 0.0005
```

### 5.3 Database Schema Additions

```sql
-- Uses existing TimescaleDB from Sprint 5

-- Phoenix training runs
CREATE TABLE phoenix_training_runs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    start_date TIMESTAMPTZ NOT NULL,
    end_date TIMESTAMPTZ,
    initial_capital DECIMAL(15, 2),
    target_cagr DECIMAL(5, 2),
    status VARCHAR(20), -- running, completed, failed
    final_level VARCHAR(30),
    overall_score DECIMAL(5, 2),
    config JSONB
);

-- Training experiences (for RAG)
CREATE TABLE phoenix_experiences (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    training_run_id UUID REFERENCES phoenix_training_runs(id),
    timestamp TIMESTAMPTZ NOT NULL,
    market_regime VARCHAR(20),
    situation JSONB,           -- Market condition snapshot
    decision JSONB,            -- Trade decision
    outcome JSONB,             -- Result
    lesson TEXT,               -- LLM extracted lesson
    embedding VECTOR(1536)     -- For similarity search
);

-- Graduation assessments
CREATE TABLE phoenix_assessments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    training_run_id UUID REFERENCES phoenix_training_runs(id),
    assessment_date TIMESTAMPTZ DEFAULT NOW(),
    level VARCHAR(30),
    overall_score DECIMAL(5, 2),
    metrics JSONB,
    regime_performance JSONB,
    stress_test_results JSONB,
    fail_reasons JSONB,
    recommendations JSONB
);

-- Create index for RAG queries
CREATE INDEX idx_phoenix_experiences_embedding ON phoenix_experiences 
USING ivfflat (embedding vector_cosine_ops);
```

---

## 6. Success Criteria

### 6.1 For Phoenix Mode

- [ ] Системата завършва 100+ трейда за < 1 час симулация
- [ ] RAG memory успешно извлича подобни случаи
- [ ] LLM стратегист подобрява решенията епоха след епоха
- [ ] Graduation criteria са реалистични и постижими
- [ ] Stress testing преминава поне 6 от 8 сценария

### 6.2 For ML APIs

- [ ] Gemini API интеграция работи (< 500ms latency)
- [ ] Fallback към OpenAI при failure
- [ ] Cost tracking и budget alerts
- [ ] Rate limit handling с exponential backoff

### 6.3 For v2.0 Architecture

- [ ] Всички нови модули използват съществуващи core types
- [ ] No breaking changes в API от Sprint 8
- [ ] Backwards compatibility със Sprints 1-8

---

## 7. Risk Assessment

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Overfitting в Phoenix | High | High | Strict OOS testing, walk-forward |
| LLM API costs | Medium | Medium | Caching, rate limiting |
| RAG performance | Medium | Medium | Index optimization, partitioning |
| Unrealistic expectations | High | High | Clear docs, realistic targets |

---

## 8. Next Actions

1. **Immediate:**
   - [ ] Implement `phoenix/graduation.rs` (already documented)
   - [ ] Create `ml/apis/gemini.rs` integration
   - [ ] Update `Cargo.toml` with new dependencies

2. **This Week:**
   - [ ] Phoenix simulator core
   - [ ] RAG memory integration test
   - [ ] Graduation criteria unit tests

3. **Next Sprint (Sprint 10):**
   - [ ] Crypto trading module
   - [ ] Real-time streaming
   - [ ] Advanced ML models

---

## 9. References

- **Sprint 5:** RAG infrastructure (`src/rag/`)
- **Sprint 6:** IB API client (`src/brokers/ib/`)
- **Sprint 7:** ML pipeline (`src/analytics/ml/`)
- **Sprint 8:** Production deployment (`k8s/`)
- **v2.0 Roadmap:** `docs/ROADMAP-v2.0.md`
- **Phoenix Graduation:** `src/phoenix/graduation.rs`

---

## 10. Decision Log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-02-08 | Changed 200x goal to realistic 15-30% CAGR | 82% CAGR is practically impossible with certainty |
| 2026-02-08 | Added 5 graduation levels | Gradual capital increase reduces risk |
| 2026-02-08 | Made stress testing mandatory | Must survive historical crises |
| 2026-02-08 | Added automatic fail conditions | Prevent overfitting and unrealistic strategies |

---

**END OF SPRINT 9 SPECIFICATION**
