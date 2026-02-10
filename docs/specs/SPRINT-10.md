# Sprint 10: ML APIs & AI Foundation

> **Status:** IMPLEMENTED  
> **Duration:** 2 weeks  
> **Goal:** Integrate LLM providers and build AI infrastructure  
> **Depends on:** Sprint 9 (Phoenix Mode), Sprint 5 (RAG)

---

## Overview

Integration of major LLM providers (Gemini, OpenAI, Claude, HuggingFace) for AI-powered trading analysis.

---

## Goals

- [ ] Gemini API integration (SEC analysis)
- [ ] OpenAI GPT-4o integration (earnings calls)
- [ ] Anthropic Claude integration (10-K deep dive)
- [ ] HuggingFace FinBERT (sentiment scoring)
- [ ] ML API Orchestrator (fallback chain)
- [ ] Cost tracking & rate limiting
- [ ] Response caching system

---

## Technical Tasks

### 1. Gemini Integration
```rust
src/ml/apis/gemini.rs
```
- [ ] REST API client
- [ ] SEC filing analysis endpoint
- [ ] Sentiment extraction
- [ ] JSON response parsing
- [ ] Rate limit handling (1000 req/day free)

### 2. OpenAI Integration
```rust
src/ml/apis/openai.rs
```
- [ ] GPT-4o-mini client
- [ ] Earnings call analysis
- [ ] Investment thesis generation
- [ ] Streaming responses
- [ ] Cost tracking per request

### 3. Claude Integration
```rust
src/ml/apis/claude.rs
```
- [ ] Claude 3 API client
- [ ] 200K context window support
- [ ] 10-K/10-Q deep analysis
- [ ] Risk factor extraction
- [ ] Management quality assessment

### 4. HuggingFace Integration
```rust
src/ml/apis/huggingface.rs
```
- [ ] FinBERT sentiment model
- [ ] Ticker symbol extraction
- [ ] Embeddings generation
- [ ] Model caching
- [ ] Rate limit handling

### 5. Orchestrator
```rust
src/ml/apis/mod.rs
```
- [ ] Provider trait definition
- [ ] Fallback chain logic
- [ ] Retry with exponential backoff
- [ ] Cost budget enforcement
- [ ] Response time monitoring

### 6. Infrastructure
- [ ] API key management (Vault)
- [ ] Request/response logging
- [ ] Cost dashboard (Grafana)
- [ ] Circuit breaker pattern
- [ ] Response caching (Redis)

---

## API Costs (Estimated)

| Provider | Model | Cost per 1K requests | Monthly Budget |
|----------|-------|---------------------|----------------|
| Google | Gemini Pro | Free | $0 |
| OpenAI | GPT-4o-mini | ~$0.50 | $50 |
| Anthropic | Claude 3 | ~$5.00 | $200 |
| HuggingFace | FinBERT | Free | $0 |
| **Total** | | | **~$250/month** |

---

## Success Criteria

- [ ] All 4 providers integrated
- [ ] < 500ms average latency
- [ ] 99.9% uptime with fallback
- [ ] Cost tracking accurate
- [ ] 100+ tests passing

---

## Dependencies

- Sprint 5: neurocod-rag (for context retrieval)
- Sprint 6: HTTP client infrastructure
- Sprint 9: Phoenix Mode (uses LLM APIs)

---

## Deliverables

1. `src/ml/apis/` - Complete API integrations
2. `tests/ml_apis/` - Unit tests
3. `config/llm.yaml` - Configuration
4. Grafana dashboard - Cost & latency monitoring

---

## Golden Path Tests

```rust
#[test]
fn test_gemini_sec_analysis() { ... }

#[test]
fn test_openai_earnings_analysis() { ... }

#[test]
fn test_claude_10k_analysis() { ... }

#[test]
fn test_huggingface_sentiment() { ... }

#[test]
fn test_fallback_chain() { ... }

#[test]
fn test_cost_tracking() { ... }

#[test]
fn test_rate_limit_handling() { ... }

#[test]
fn test_response_caching() { ... }
```

---

## Risk Assessment

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| API rate limits | High | Medium | Caching, fallback |
| Cost overrun | Medium | High | Budget alerts |
| API changes | Low | Medium | Version pinning |

---

**Next:** Sprint 11 (Multi-Asset: Crypto + Forex)
