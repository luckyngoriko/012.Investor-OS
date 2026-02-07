# 🔄 Investor OS — Borrowed Components Registry

> **Purpose:** Track ALL reused code, patterns, and documents from the ecosystem
> **Rule:** Every borrow gets an entry. No silent copying.

---

## Summary

| Source Project | Items Borrowed | Lines Saved |
|---------------|---------------|-------------|
| **NeuroCOD** (neurocod-rag) | 14 RAG modules | ~5,906 |
| **AI-OS.NET** | Agent System, Workflows, Audit Trail | ~2,500 |
| **NeuroCAD** | Sprint Gate, Decision Log, Golden Path | ~1,200 |
| **Total** | | **~9,600 lines saved** |

---

## From NeuroCOD — RAG Engine (L1-L28)

| # | Module | File | Lines | Investor OS Use | Adaptation |
|---|--------|------|-------|----------------|------------|
| 1 | **Embeddings** | `embeddings.rs` | 174 | SEC filing embeddings | Model: AllMiniLmL6V2 → finance-tuned model |
| 2 | **Chunker** | `chunker.rs` | ~180 | Earnings report chunking | Add `EarningsChunk`, `FilingChunk` variants |
| 3 | **Hybrid Retriever** | `retriever.rs` | 398 | SEC/Earnings hybrid search | As-is, RRF fusion works for financial docs |
| 4 | **Multi-Indexer** | `indexer.rs` | ~200 | Multi-index (stocks, filings, journal) | Adapt IndexType enum |
| 5 | **Hierarchy** | `hierarchy.rs` | ~200 | Filing structure (10-K → Section → Item) | Map to SEC filing hierarchy |
| 6 | **PgVector Store** | `pgstore.rs` | 561 | Vector storage backbone | Schema: `rag_chunks` → `financial_docs` |
| 7 | **Storage Trait** | `storage.rs` | ~80 | VectorStore trait | As-is |
| 8 | **Config** | `config.rs` | ~120 | RAG config | Add financial-specific defaults |
| 9 | **Types** | `types.rs` | ~180 | Core types (Chunk, Embedding, SearchResult) | Add `FinancialMetadata` field |
| 10 | **Temporal** | `temporal.rs` | 235 | "Insider buys in last 90 days" queries | Perfect fit — TimeRange::last_days(90) |
| 11 | **Evidence** | `evidence.rs` | 307 | Trade rationale with citations | Claim → InvestmentThesis, Evidence → DataPoint |
| 12 | **Feedback** | `feedback.rs` | 208 | Decision journal learning loop | ThumbsUp/Down → Profitable/Unprofitable trade |
| 13 | **Security** | `security.rs` | 284 | Audit trail for all queries | AuditStore → TradeAuditLog |
| 14 | **Query Processing** | `query/*.rs` | ~350 | Financial query expansion | Add financial synonyms (PE → P/E → price-to-earnings) |

### Usage Pattern
```toml
# Cargo.toml — direct path dependency
[dependencies]
neurocod-synapse = { path = "../002.NeuroCOD.eu/neurocod_synapse", default-features = false, features = ["rag"] }
```

---

## From AI-OS.NET — Governance & Infrastructure

| # | Item | Source File | Adaptation |
|---|------|-----------|------------|
| 1 | **AGENT_SYSTEM.md** | `AI-OS.NET/AGENT_SYSTEM.md` v2.0 | Rust commands, CQ formula, financial rules |
| 2 | **Onboarding workflow** | `.agent/workflows/onboarding.md` | Updated file paths, cargo commands |
| 3 | **Golden Path workflow** | `.agent/workflows/golden-path.md` | Rust-only (no Playwright for v1) |
| 4 | **Test Gate workflow** | `.agent/workflows/test-gate.md` | 5-gate protocol adapted for cargo |
| 5 | **Sprint workflow** | `.agent/workflows/sprint.md` | Sprint markers for financial domain |
| 6 | **Documentation workflow** | `.agent/workflows/documentation.md` | Cargo doc templates |
| 7 | **RAG Module workflow** | `.agent/workflows/rag-module.md` | Financial RAG adaptation |
| 8 | **Kill Switch pattern** | Extracted from SPEC | Rust implementation pattern |

---

## From NeuroCAD — Development Patterns

| # | Item | Source | Adaptation |
|---|------|--------|------------|
| 1 | **Decision Log format** | `DECISION_LOG.md` | Identical format, financial context |
| 2 | **Sprint Gate Protocol** | `AGENT_SYSTEM.md` | 5-gate with cargo commands |
| 3 | **Index-based API pattern** | Kernel architecture | Apply to position/portfolio management |
| 4 | **Golden Path methodology** | `GOLDEN-PATH-TESTS.md` | Financial domain tests |

---

## From Richard Dennis — Trading Strategy

| # | Concept | Implementation |
|---|---------|---------------|
| 1 | **ATR Trailing Stop** | `TrailingStop` struct with ATR(20) × 2.0 |
| 2 | **Breakout Score** | 20-period Highest High breakout detection |
| 3 | **Risk:Reward philosophy** | "Lose small many times, win big few times" → CQ threshold logic |
| 4 | **Multi-timeframe** | Period 20 for 1H, Period 55 for daily/weekly |
| 5 | **SMA(200) trend filter** | Only trade in direction of 200-period moving average |
