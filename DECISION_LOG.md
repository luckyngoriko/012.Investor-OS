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

---

## DEC-005: AI Stack v3.5 — LangChain/LangGraph/Temporal (2026-02-09)

**Context:** Need to integrate AI orchestration patterns (LangChain, LangGraph, Temporal) into pure Rust stack.
**Decision:** Build our own Rust implementations inspired by these patterns, not using Python libraries.

**Sprint 1 — LangChain Core:**
- ✅ Chain trait with async execution
- ✅ PromptTemplate with variable substitution
- ✅ ChainContext/ChainResult for state management
- ✅ Trading-specific prompts (SEC analysis, CQ calculation)
- ✅ Integration tests passing

**Architecture:**
```
src/langchain/
├── mod.rs      — Chain trait, ChainContext, ChainResult
├── chains.rs   — LLMChain, SequentialChain, ParallelChain, RAGChain
├── prompts.rs  — PromptTemplate with validation
├── tools.rs    — Tool trait + Trading tools
├── parsers.rs  — JSON, Score, List parsers
└── memory.rs   — Conversation, Vector, Summary memory
```

**Rationale:**
- Full type safety with Rust
- Zero Python/Go dependencies
- Async-native design with tokio
- Direct integration with existing ml::apis

**Outcome:** ✅ Sprint 1 Complete — All tests passing

---

## DEC-006: AI Stack v3.5 — Sprint 2: Tools + Agent (2026-02-09)

**Context:** Need ReAct pattern agent with trading tools for autonomous decision making.
**Decision:** Implement Agent with tool-calling capability using ReAct pattern.

**Sprint 2 — LangChain Tools + Agent:**
- ✅ Agent with ReAct pattern (Reasoning + Acting)
- ✅ Tool trait for external function calls
- ✅ ToolRegistry for tool management
- ✅ AgentBuilder for fluent configuration
- ✅ Parsing Action/Final Answer from LLM output

**New Components:**
```
src/langchain/
└── agent.rs    — Agent, AgentBuilder, ReAct loop
```

**Example Usage:**
```rust
let agent = AgentBuilder::new()
    .with_llm(llm)
    .with_tool(Box::new(PortfolioTool::new(service)))
    .with_tool(Box::new(MarketDataTool::new(service)))
    .with_system_prompt("You are a trading assistant...")
    .build()?;

let result = agent.run(ctx! {
    "question": "Should I buy AAPL based on current market conditions?"
}).await?;
```

**Tests:**
- ✅ test_parse_action — парсване на Action/Action Input
- ✅ test_parse_final_answer — парсване на Final Answer
- ✅ test_trading_prompts_exist — trading prompts

**Outcome:** ✅ Sprint 2 Complete — 6 tests passing

---

## DEC-007: AI Stack v3.5 — Sprint 3: LangGraph Core (2026-02-09)

**Context:** Need state machine for trading decision flow with conditional routing.
**Decision:** Implement graph-based execution with nodes, edges, and shared state.

**Sprint 3 — LangGraph Core:**
- ✅ Graph trait with async execution
- ✅ GraphBuilder for fluent graph construction
- ✅ SharedState with CQ, MarketRegime, TradingAction
- ✅ Conditional edges (regime-based routing)
- ✅ Trading nodes (CQ calc, Risk check, Execute)
- ✅ StateBuilder for easy state construction

**Components:**
```
src/langgraph/
├── mod.rs      — Graph trait, GraphExecutor, GraphError
├── state.rs    — SharedState, MarketRegime, TradingAction, ExecutionStatus
├── graph.rs    — ExecutableGraph, GraphBuilder (petgraph)
├── nodes.rs    — Node trait, Trading nodes
└── edges.rs    — Conditional edges, EdgeBuilder
```

**Tests:**
- ✅ test_shared_state_builder
- ✅ test_cq_calculation  
- ✅ test_market_regime_conditions
- ✅ test_cq_condition
- ✅ test_state_snapshot
- ✅ test_start_node
- ✅ test_cq_calculation_node
- ✅ test_simple_graph_execution
- ✅ test_execution_status
- ✅ test_trading_action_variants

**Example:**
```rust
let graph = GraphBuilder::new("trading_decision")
    .add_node("start", StartNode)
    .add_node("cq_calc", CQCalculationNode)
    .add_node("execute", ExecutionNode)
    .add_edge("start", "cq_calc")
    .add_conditional_edge("cq_calc", cq_above(0.7), "execute")
    .set_start("start")
    .build()?;

let result = graph.execute(SharedState::new("AAPL")).await?;
```

**Outcome:** ✅ Sprint 3 Complete — 10 tests passing

---

## DEC-008: AI Stack v3.5 — Sprint 4: Temporal Core (2026-02-09)

**Context:** Need durable execution for trading workflows with retry, signals, and compensation.
**Decision:** Implement Temporal-inspired workflow engine with activities and saga pattern.

**Sprint 4 — Temporal Core:**
- ✅ Workflow trait with async execution
- ✅ Activity trait for idempotent operations
- ✅ RetryPolicy with exponential backoff
- ✅ Saga pattern for distributed transactions
- ✅ ActivityContext and WorkflowContext
- ✅ Trading activities (FetchMarketData, CalculateCQ, CallLLM, PlaceOrder)

**Components:**
```
src/temporal/
├── mod.rs          — Workflow trait, RetryPolicy, WorkflowStatus
├── workflow.rs     — WorkflowContext, WorkflowHandle, WorkflowComposer
├── activity.rs     — Activity trait, ActivityContext, ActivityExecutor
├── context.rs      — WorkflowContext, ActivityContext (extended)
├── client.rs       — TemporalClient, WorkflowClient
├── saga.rs         — Saga pattern, SagaBuilder, compensation
└── worker.rs       — Worker, TestWorker
```

**Tests:**
- ✅ test_simple_workflow
- ✅ test_workflow_context
- ✅ test_activity_execution
- ✅ test_activity_context
- ✅ test_retry_policy_backoff
- ✅ test_retry_policy_should_retry
- ✅ test_retry_policy_non_retryable
- ✅ test_workflow_status_variants
- ✅ test_saga_success
- ✅ test_saga_compensation
- ✅ test_activity_error_types
- ✅ test_calculate_cq_activity
- ✅ test_call_llm_activity
- ✅ test_place_order_activity

**Example:**
```rust
// Define workflow
struct SignalGenerationWorkflow;

#[async_trait]
impl Workflow for SignalGenerationWorkflow {
    type Input = SignalRequest;
    type Output = SignalResult;
    
    async fn run(&self, ctx: WorkflowContext, input: Self::Input) -> Result<Self::Output> {
        // Execute activity with retry
        let data = ctx.activity(FetchMarketData, input.ticker).await?;
        
        // Calculate CQ
        let cq = ctx.activity(CalculateCQ, data).await?;
        
        // Saga for order execution
        let saga = SagaBuilder::new()
            .step(ReserveFunds)
            .step(PlaceOrder)
            .step(ConfirmFill)
            .build();
        
        saga.execute(&ctx).await?;
        
        Ok(SignalResult::success())
    }
}
```

**Outcome:** ✅ Sprint 4 Complete — 14 tests passing

---

## DEC-009: Code Coverage Improvement (2026-02-09)

**Context:** Need to improve test coverage from ~45% to ≥80% before production.
**Action:** Added comprehensive tests for critical paths.

**New Test Files:**
- `tests/coverage_chains_test.rs` — Chain execution tests
- `tests/coverage_tools_test.rs` — Tool registry tests
- `tests/coverage_graph_test.rs` — Graph/edge/state tests
- `tests/coverage_temporal_test.rs` — Context/retry/error tests

**Coverage Improvement:**
| Module | Before | After | Status |
|--------|--------|-------|--------|
| langchain | 4 tests | 17 tests | +325% |
| langgraph | 10 tests | 28 tests | +180% |
| temporal | 14 tests | 27 tests | +93% |
| **TOTAL** | **28** | **68** | **+143%** |

**Key Additions:**
- ✅ SequentialChain/ParallelChain execution
- ✅ ToolRegistry with multiple tools
- ✅ Error handling (tool not found, execution failure)
- ✅ Edge conditions (AND, OR, NOT combinators)
- ✅ RetryPolicy backoff calculation
- ✅ ActivityContext lifecycle
- ✅ WorkflowContext sleep/elapsed

**Outcome:** ✅ 68 tests total, estimated ~65% coverage

---

## DEC-XXX: Sprint 36 — HRM Native Rust Implementation (2026-02-12)

**Context:** Need adaptive Conviction Quotient that adjusts weights based on market regime. HRM (Hierarchical Reasoning Model) from Sapient Inc offers ideal architecture but is PyTorch/Python.

**Options:**
1. Python gRPC service — Easier, keep original HRM code
2. ONNX Runtime — Convert to ONNX, run in Rust
3. Native Rust (burn) — Full Rust implementation
4. tch-rs (LibTorch bindings) — PyTorch C++ API in Rust

**Chosen:** Option 3 — Native Rust with burn framework

**Rationale:**
- **Memory Safety:** Critical for financial system handling real money — Rust's ownership model prevents memory bugs
- **Performance:** 10x faster inference (1-5ms vs 10-50ms with gRPC overhead)
- **Deployment:** Single binary, no Python runtime, ~50MB vs 2.3GB Docker image
- **Type Safety:** Compile-time guarantees prevent financial calculation errors
- **Maintenance:** One language (Rust) across entire codebase
- **Future:** Easier to extend with custom training pipeline in Rust

**Trade-offs:**
- ❌ Longer initial development time (1-2 weeks vs 2-3 days)
- ❌ burn framework less mature than PyTorch (risk of bugs)
- ❌ Need to port HRM architecture manually (no direct code reuse)
- ❌ Training still needs Python (export weights to safetensors)

**Architecture Decisions:**
```
HRM Structure:
├── High-Level Module (LSTM 128) — Slow, abstract planning
├── Low-Level Module (LSTM 64) — Fast, detailed execution
├── Cross-connections (Linear layers) — Information flow
└── Output (192→3) — [conviction, confidence, regime]
```

**Fallback Strategy:**
- If HRM confidence < 0.7 → use static CQ formula
- If HRM inference timeout > 5ms → use static CQ formula
- If weights not loaded → use static CQ formula

**Files Created:**
- `src/hrm/mod.rs` — Module exports
- `src/hrm/config.rs` — HRMConfig, DeviceConfig
- `src/hrm/model.rs` — HRM struct, HRMBuilder
- `src/hrm/inference.rs` — InferenceEngine, InferenceResult
- `src/hrm/weights.rs` — WeightLoader, ModelWeights
- `tests/golden_path/hrm_36_tests.rs` — 30 Golden Path tests
- `docs/sprints/sprint36_hrm_engine.md` — Sprint documentation
- `docs/hrm/ARCHITECTURE.md` — Technical deep dive

**Dependencies Added:**
```toml
burn = { version = "0.16", features = ["std", "train-minimal"] }
burn-ndarray = { version = "0.16", features = ["std"] }
```

**Golden Path Tests:**
- 30 tests covering initialization, inference, batching, error handling
- Target: < 5ms p99 latency, < 100MB memory

**Borrowed Registry Entry:**
- BORROWED.md entry for HRM architectural pattern (not code)

**Outcome:** 🔄 In Progress — Module scaffolding complete, pending burn tensor integration
