# 🤖 Investor OS — Agent System v1.0

> **Version:** 1.0 | **Updated:** 2026-02-07
> **Source:** Adapted from AI-OS.NET v2.0 + NeuroCAD patterns
> **Stack:** Rust + Axum + SQLx + Next.js 15

---

## 🏗️ Architecture: Rust-First Trading System

```
investor-os/
├── Cargo.toml                    # Workspace root
├── crates/
│   ├── investor-core/            # Domain types, CQ formula, signals
│   ├── investor-collectors/      # Data collectors (Finnhub, SEC, yfinance)
│   ├── investor-signals/         # Signal engine (QVM, Insider, Sentiment)
│   ├── investor-decision/        # Decision engine (CQ, proposals, regime)
│   ├── investor-api/             # Axum REST API
│   └── investor-rag/             # Re-export neurocod-rag with finance adapters
├── frontend/                     # Next.js 15 dashboard
├── migrations/                   # SQLx migrations
├── tests/                        # Integration + Golden Path tests
├── docs/
│   ├── specs/                    # Locked specifications
│   └── architecture/             # Architecture docs
├── AGENT_SYSTEM.md               # THIS FILE
├── DECISION_LOG.md               # Chronological decisions
├── BORROWED.md                   # Cross-project reuse registry
└── docker-compose.yml            # PostgreSQL + TimescaleDB + Redis
```

---

## 📋 Rules (MANDATORY)

### R1: Workflow First
Every action follows a workflow from `.agent/workflows/`. No freestyle coding.

### R2: Golden Path or No Merge
```
Sprint advances ONLY when:
✅ All GP tests pass (cargo test)
✅ Zero clippy warnings (cargo clippy -- -D warnings)
✅ Cargo build succeeds
✅ Coverage ≥ 80%
✅ Docs updated
```

### R3: Decision Log
Every non-trivial decision → `DECISION_LOG.md` entry with:
- Context, Options, Chosen, Rationale, Outcome

### R4: Borrowed Registry
Every reused component → `BORROWED.md` entry with:
- Source project, Target, Adaptations made

### R5: Type Safety
- SQLx compile-time query checking
- All financial calculations use `rust_decimal::Decimal` (not f64)
- Signal scores are `f32` in range [0.0, 1.0] — enforced by `Score` newtype

### R6: Kill Switch
Never remove or weaken the kill switch logic. Hard limits:
- Max drawdown: -10% → system freeze
- Data staleness > 48h → no new trades
- Error rate > 5% → alert + pause

---

## 🔄 Sprint Gate Protocol (5 Gates)

| Gate | Command | Pass Criteria |
|------|---------|--------------|
| **GP Gate** | `cargo test -- --test-threads=1` | All Golden Path tests GREEN |
| **Clippy Gate** | `cargo clippy -- -D warnings` | Zero warnings |
| **Build Gate** | `cargo build --release` | Clean build |
| **Coverage Gate** | `cargo llvm-cov --html` | ≥ 80% line coverage |
| **Doc Gate** | `cargo doc --no-deps` | No doc warnings |

---

## 🧬 Key Patterns (from ecosystem)

### Pattern: Conviction Quotient (CQ) — Domain-Specific
```rust
use rust_decimal::Decimal;

/// CQ v2.0 — Dennis-inspired composite score
pub struct ConvictionQuotient {
    pub pegy_relative: Score,    // 0.20 weight
    pub insider_score: Score,    // 0.20 weight
    pub sentiment_score: Score,  // 0.15 weight
    pub regime_fit: Score,       // 0.20 weight
    pub breakout_score: Score,   // 0.15 weight (Dennis)
    pub atr_trend: Score,        // 0.10 weight (Dennis)
}

impl ConvictionQuotient {
    pub fn calculate(&self) -> Score {
        Score::new(
            self.pegy_relative.0 * 0.20
            + self.insider_score.0 * 0.20
            + self.sentiment_score.0 * 0.15
            + self.regime_fit.0 * 0.20
            + self.breakout_score.0 * 0.15
            + self.atr_trend.0 * 0.10
        )
    }
}
```

### Pattern: Score Newtype (Ecosystem Standard)
```rust
/// Validated score in [0.0, 1.0]
#[derive(Debug, Clone, Copy)]
pub struct Score(f32);

impl Score {
    pub fn new(value: f32) -> Self {
        Self(value.clamp(0.0, 1.0))
    }
}
```

### Pattern: RAG Reuse (from neurocod-rag)
```toml
[dependencies]
# Direct dependency on ecosystem RAG
neurocod-rag = { path = "../002.NeuroCOD.eu/neurocod_synapse", features = ["pgvector"] }
```

---

## ⚡ Quick Reference

| Command | Purpose |
|---------|---------|
| `cargo test` | Run all tests |
| `cargo clippy -- -D warnings` | Lint check |
| `cargo build --release` | Production build |
| `cargo run --bin investor-api` | Start API server |
| `cd frontend && npm run dev` | Start Next.js dashboard |
| `docker compose up -d` | Start PostgreSQL + Redis |

---

## 📊 Current Sprint Focus

**Sprint:** Pre-Sprint (Infrastructure Setup)
**Status:** Setting up agent system, workflows, documentation
**Next:** Sprint 1 — Foundation (Docker + DB + Collectors)

---

## 🚨 Emergency: Kill Switch

```bash
# Manual kill switch
curl -X POST http://localhost:3000/api/killswitch

# Database direct
UPDATE system_config SET value = 'true' WHERE key = 'kill_switch';
```

---

## ⚠️ Common Anti-Patterns

| ❌ Don't | ✅ Do |
|----------|-------|
| Use f64 for money | Use `rust_decimal::Decimal` |
| Skip tests "just this once" | Write GP test FIRST (TDD) |
| Hardcode API keys | Use env vars + `.env` |
| Trust single signal | Always require CQ composite |
| Override kill switch | NEVER disable safety limits |
| Copy without BORROWED.md entry | Always register in BORROWED.md |
