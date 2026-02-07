# 📋 Investor OS — Decision Log

> **Format:** Adopted from NeuroCAD → AI-OS.NET → Investor OS
> **Rule:** Every non-trivial decision gets an entry

---

## DEC-001: Rust Stack (2026-02-07)

**Context:** SPEC-v1.0 specified Python (FastAPI + Celery). Ecosystem uses Rust.
**Options:**
1. Python — original spec, native FinBERT/HMM
2. Rust — ecosystem reuse, type safety, performance
3. Hybrid — Python collectors + Rust engine

**Chosen:** Option 2 — Full Rust
**Rationale:**
- Direct reuse of `neurocod-rag` crate (5,906 lines, L1-L28)
- Shared patterns with NeuroCAD (38 sprints proven) and AI-OS.NET
- Single binary deployment vs 5 Docker containers
- Compile-time SQL checking (SQLx) prevents DB bugs
- `rust_decimal` for financial precision (no floating-point errors)

**Trade-off:** FinBERT/HMM need Python sidecar or Rust alternatives (`fastembed`, `linfa`)
**Outcome:** Pending — Sprint 1 will validate

---

## DEC-002: CQ v2.0 Formula — Dennis Upgrade (2026-02-07)

**Context:** Original CQ had 4 equal-weight factors. Dennis strategy analysis revealed missing price action signals.
**Options:**
1. Keep v1.0 (4 factors × 0.25)
2. Add 2 Dennis factors (breakout + ATR trend)

**Chosen:** Option 2 — CQ v2.0
```
CQ = PEGY_rel×0.20 + Insider×0.20 + Sentiment×0.15 + Regime×0.20 + Breakout×0.15 + ATR_trend×0.10
```
**Rationale:** Dennis proved breakout confirmation + volatility trend are critical edge components. The original CQ lacked price action intelligence.
**Outcome:** Pending — needs backtesting validation

---

## DEC-003: neurocod-rag Direct Dependency (2026-02-07)

**Context:** Need RAG for SEC filings, earnings analysis, and decision journal search.
**Options:**
1. Write RAG from scratch
2. Fork neurocod-rag
3. Path dependency on neurocod-rag

**Chosen:** Option 3 — Path dependency
**Rationale:** Keeps upstream updates flowing, zero code duplication. Financial-specific adapters (earnings chunker, SEC parser) will live in `investor-rag` crate as a thin wrapper layer.
**Outcome:** Pending

---

## DEC-004: Agent System v1.0 (2026-02-07)

**Context:** Need development governance for Investor OS.
**Chosen:** Copy and adapt from AI-OS.NET v2.0 (itself adapted from NeuroCAD)
**Adaptations:**
- Rust-specific commands (cargo vs npm)
- Financial domain rules (Score newtype, Decimal money)
- Kill switch as first-class citizen
- CQ formula as pattern example
**Outcome:** ✅ Complete
