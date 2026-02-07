---
description: RAG module integration workflow for Investor OS financial domain
---

# Financial RAG Integration Workflow

## Overview
Investor OS uses `neurocod-rag` (L1-L28) as foundation, adding financial domain adapters.

## Dependency Setup
```toml
# Cargo.toml
[dependencies]
neurocod-synapse = { path = "../002.NeuroCOD.eu/neurocod_synapse", default-features = false, features = ["rag"] }
```

## Financial Adapters to Build

### 1. Earnings Chunker
Split earnings call transcripts into structured chunks:
- Q&A sections
- Management commentary
- Forward guidance
- Risk disclaimers

### 2. SEC Filing Parser
Parse 10-K / 10-Q filings:
- Item 1: Business Overview
- Item 1A: Risk Factors
- Item 7: MD&A
- Item 8: Financial Statements

### 3. Financial Query Expander
Expand financial terminology:
- "PE" → "price-to-earnings ratio"
- "ROIC" → "return on invested capital"
- "insider buying" → "SEC Form 4 purchase transactions"

### 4. Trade Decision Evidence
Use `evidence.rs` pattern for trade rationale:
```rust
let thesis = InvestmentThesis {
    claim: "ACME Corp is undervalued vs peers",
    evidence: vec![
        DataPoint::from_pegy(0.85, peer_median: 1.35),
        DataPoint::from_insider(3, "cluster_buy", 90),
        DataPoint::from_sentiment(0.72, "earnings_call"),
    ],
};
```

## RAG Levels Used by Investor OS

| Level | Module | Financial Use |
|-------|--------|--------------|
| L1 | Embeddings | SEC filing + earnings embeddings |
| L2 | Chunker | Earnings/filing structured splitting |
| L3 | Hybrid Retriever | "Find all mentions of margin expansion" |
| L5 | Hierarchy | 10-K → Section → Item → Paragraph |
| L11 | Temporal | "What did CEO say about growth LAST quarter?" |
| L14 | Security | Audit trail for all research queries |
| L16 | Evidence | Investment thesis with data citations |
| L21 | Feedback | Learn from profitable vs unprofitable research patterns |
