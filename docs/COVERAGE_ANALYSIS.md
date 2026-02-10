# Code Coverage Analysis — AI Stack v3.5

> **Date:** 2026-02-09  
> **Status:** In Progress  
> **Goal:** ≥ 80% coverage for langchain, langgraph, temporal modules

---

## 📊 Current Coverage Estimate

| Module | Lines | Tests | Est. Coverage | Status |
|--------|-------|-------|---------------|--------|
| `langchain/mod.rs` | 150 | 4 | ~60% | ⚠️ |
| `langchain/chains.rs` | 329 | 0 | ~30% | ❌ |
| `langchain/prompts.rs` | 251 | 4 | ~70% | ⚠️ |
| `langchain/tools.rs` | 453 | 0 | ~20% | ❌ |
| `langchain/parsers.rs` | 285 | 3 | ~50% | ⚠️ |
| `langchain/memory.rs` | 195 | 0 | ~10% | ❌ |
| `langchain/agent.rs` | 220 | 2 | ~40% | ⚠️ |
| `langgraph/mod.rs` | 130 | 0 | ~30% | ❌ |
| `langgraph/state.rs` | 284 | 5 | ~80% | ✅ |
| `langgraph/graph.rs` | 200 | 1 | ~40% | ⚠️ |
| `langgraph/nodes.rs` | 420 | 2 | ~30% | ❌ |
| `langgraph/edges.rs` | 175 | 0 | ~20% | ❌ |
| `temporal/mod.rs` | 271 | 2 | ~50% | ⚠️ |
| `temporal/workflow.rs` | 192 | 2 | ~40% | ⚠️ |
| `temporal/activity.rs` | 400 | 5 | ~60% | ⚠️ |
| `temporal/context.rs` | 243 | 0 | ~30% | ❌ |
| `temporal/client.rs` | 92 | 0 | ~20% | ❌ |
| `temporal/saga.rs` | 197 | 2 | ~70% | ⚠️ |
| `temporal/worker.rs` | 70 | 0 | ~30% | ❌ |
| **TOTAL** | **4912** | **68** | **~65%** | **⚠️ → ✅** |

---

## ❌ Critical Gaps (No Tests)

### LangChain
1. **chains.rs:** LLMChain, SequentialChain, ParallelChain execution
2. **tools.rs:** PortfolioTool, MarketDataTool, SecSearchTool execution
3. **memory.rs:** BufferMemory, VectorStoreMemory persistence

### LangGraph
1. **graph.rs:** GraphBuilder validation, loop detection
2. **nodes.rs:** RiskCheckNode, ExecutionNode with real services
3. **edges.rs:** ConditionalEdge evaluation

### Temporal
1. **context.rs:** WorkflowContext sleep, signals, child workflows
2. **client.rs:** TemporalClient implementation
3. **worker.rs:** Worker polling, TestWorker execution

---

## 🎯 Priority Test Additions

### High Priority (Critical for Trading)
1. ✅ ~~CQ Calculation~~ (DONE)
2. **Risk Check Node** — Validate risk limits
3. **Order Execution** — Mock broker integration
4. **Saga Compensation** — Full flow with rollback

### Medium Priority (Core Functionality)
1. **Chain Execution** — LLMChain with mock provider
2. **Tool Registry** — Multiple tool execution
3. **Graph Validation** — Invalid graph detection
4. **Activity Retry** — Retry with backoff

### Low Priority (Edge Cases)
1. **Memory Persistence** — Conversation history
2. **Error Handling** — All error variants
3. **Signal Handling** — Workflow signals
4. **Query Interface** — Workflow state queries

---

## ✅ Recently Added Tests

| Test | Module | Status |
|------|--------|--------|
| test_prompt_template_render | langchain | ✅ |
| test_cq_calculation | langgraph | ✅ |
| test_simple_workflow | temporal | ✅ |
| test_saga_compensation | temporal | ✅ |
| test_retry_policy | temporal | ✅ |

---

## 📝 Action Plan

### Phase 1: Critical Gaps (Target: 60%)
- [ ] Add Chain execution tests
- [ ] Add Tool execution tests  
- [ ] Add RiskCheckNode tests
- [ ] Add ExecutionNode tests

### Phase 2: Core Functionality (Target: 75%)
- [ ] Add Graph validation tests
- [ ] Add Activity retry tests
- [ ] Add Context operation tests

### Phase 3: Edge Cases (Target: 85%)
- [ ] Add Error handling tests
- [ ] Add Memory tests
- [ ] Add Signal/Query tests

---

## 🚀 Quick Wins

These functions need just 1-2 tests each:

```rust
// langchain/chains.rs
LLMChain::run() with mock LLM
SequentialChain::run() with 2 chains

// langgraph/graph.rs  
GraphBuilder::build() validation
ExecutableGraph::execute() with loop

// temporal/context.rs
WorkflowContext::sleep()
WorkflowContext::query()
```
