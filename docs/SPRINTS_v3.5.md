# Investor OS — Sprints v3.5 (AI Stack Refactor)

> **Версия:** 3.5  
> **Дата:** 2026-02-09  
> **Цел:** Интеграция на LangChain, LangGraph, Temporal в Rust  
> **Статус:** RFC

---

## Общ План (12 Спринта)

```
Sprint 0:  Foundation + Planning
Sprint 1:  LangChain Core (Chains, Prompts)
Sprint 2:  LangChain Tools + Parsers
Sprint 3:  LangGraph Core (Graph, State)
Sprint 4:  LangGraph Trading Nodes
Sprint 5:  Temporal Core (Workflows, Activities)
Sprint 6:  Temporal Trading Workflows
Sprint 7:  Integration — Phoenix + LangGraph
Sprint 8:  Integration — Execution + Temporal
Sprint 9:  Observability + Testing
Sprint 10: Performance Optimization
Sprint 11: Documentation + Hardening
```

---

## Sprint 0: Foundation + Planning (1 седмица)

**Цел:** Подготовка на codebase за AI stack refactor

### Задачи
- [ ] Създаване на `src/langchain/`, `src/langgraph/`, `src/temporal/` modules
- [ ] Дефиниране на core traits и типове
- [ ] Update на `Cargo.toml` с нови dependencies
- [ ] Настройка на test infrastructure
- [ ] Migration plan review

### Приемственост
- ✅ Всички съществуващи тестове минават
- ✅ Код компилира
- ✅ Нова структура е одобрена

### Golden Path Test
```bash
cargo test --all
cargo clippy -- -D warnings
```

---

## Sprint 1: LangChain Core (2 седмици)

**Цел:** Chain trait + Prompt templates

### Компоненти
| Файл | Отговорност |
|------|-------------|
| `src/langchain/mod.rs` | Основни типове, Chain trait |
| `src/langchain/chains.rs` | LLMChain, SequentialChain, ParallelChain |
| `src/langchain/prompts.rs` | PromptTemplate с валидация |

### Задачи
- [ ] `Chain` trait с `async fn run()`
- [ ] `ChainContext` за променливи
- [ ] `ChainResult` с метаданни
- [ ] `PromptTemplate` с `{variable}` substitution
- [ ] `LLMChain` — базова интеграция с LLM
- [ ] `SequentialChain` — chaining на няколко chain-a
- [ ] Unit tests за всички chains

### Integration
- Интеграция със съществуващия `ml::apis::LLMProvider`

### Golden Path Test
```rust
#[tokio::test]
async fn test_llm_chain() {
    let chain = LLMChain::new(mock_llm(), PromptTemplate::new("Hello {name}"));
    let result = chain.run(ctx! { "name": "World" }).await.unwrap();
    assert!(result.output.contains("World"));
}
```

---

## Sprint 2: LangChain Tools + Parsers (2 седмици)

**Цел:** Tools система + Structured output

### Компоненти
| Файл | Отговорност |
|------|-------------|
| `src/langchain/tools.rs` | Tool trait, ToolRegistry, Trading tools |
| `src/langchain/parsers.rs` | JSON, List, Score parsers |
| `src/langchain/memory.rs` | Conversation memory |

### Tools за Trading
- `PortfolioTool` — текущ портфейл
- `MarketDataTool` — цени и OHLCV
- `SecSearchTool` — RAG търсене в SEC filings
- `SentimentTool` — sentiment анализ
- `PlaceOrderTool` — изпълнение на поръчки

### Parsers
- `JsonParser<T>` — structured JSON output
- `ScoreParser` — 0-1 или 0-100 → нормализиран
- `ListParser` — списъци

### Задачи
- [ ] `Tool` trait с JSON schema
- [ ] `ToolRegistry` за управление
- [ ] `AgentChain` (ReAct pattern)
- [ ] `RAGChain` — retrieval + generation
- [ ] Всички trading tools
- [ ] Пълен parser coverage

### Golden Path Test
```rust
#[tokio::test]
async fn test_agent_with_tools() {
    let mut registry = ToolRegistry::new();
    registry.register(Box::new(PortfolioTool::new(mock_service())));
    
    let agent = AgentChain::new(mock_llm(), registry);
    let result = agent.run(ctx! { 
        "question": "What is my portfolio value?" 
    }).await.unwrap();
    
    assert!(result.output.contains("$"));
}
```

---

## Sprint 3: LangGraph Core (2 седмици)

**Цел:** State machine framework

### Компоненти
| Файл | Отговорност |
|------|-------------|
| `src/langgraph/mod.rs` | Graph trait, Executor |
| `src/langgraph/state.rs` | `SharedState` за trading |
| `src/langgraph/graph.rs` | `GraphBuilder`, petgraph интеграция |
| `src/langgraph/edges.rs` | Conditional edges |
| `src/langgraph/nodes.rs` | `Node` trait |

### Задачи
- [ ] `SharedState` struct с всички trading полета
- [ ] `Node` trait с `execute()`
- [ ] `NodeOutput` — Continue, End, Jump
- [ ] `GraphBuilder` за декларативно дефиниране
- [ ] Conditional edges с closures
- [ ] Loop detection
- [ ] Graph visualization (DOT format)

### Golden Path Test
```rust
#[tokio::test]
async fn test_simple_graph() {
    let graph = GraphBuilder::new("test")
        .add_node("start", StartNode)
        .add_node("middle", TransformNode)
        .add_node("end", EndNode)
        .add_edge("start", "middle")
        .add_edge("middle", "end")
        .set_start("start")
        .build()
        .unwrap();
    
    let result = graph.execute(SharedState::new("AAPL")).await.unwrap();
    assert!(result.is_complete());
}
```

---

## Sprint 4: LangGraph Trading Nodes (2 седмици)

**Цел:** Trading-specific nodes

### Nodes
| Node | Отговорност |
|------|-------------|
| `DataCollectionNode` | Fetch price, OHLCV |
| `RegimeDetectionNode` | ML regime classification |
| `BreakoutStrategyNode` | Breakout + ATR trend scores |
| `MeanReversionStrategyNode` | Mean reversion scores |
| `CQCalculationNode` | CQ v2.0 formula |
| `RiskCheckNode` | Risk limits, position sizing |
| `ExecutionNode` | Broker order placement |

### Conditional Logic
```rust
graph.add_conditional_edge(
    "detect_regime",
    |state| matches!(state.market_regime, MarketRegime::Trending),
    "breakout_strategy"
);
```

### Задачи
- [ ] Всички trading nodes
- [ ] Mock services за тестване
- [ ] Integration със съществуващите signals
- [ ] Error handling в nodes

### Golden Path Test
```rust
#[tokio::test]
async fn test_trading_graph_end_to_end() {
    let graph = build_trading_graph(mock_services());
    let state = StateBuilder::new("AAPL")
        .with_price(dec!(150.0))
        .with_quality_score(0.8)
        .with_insider_score(0.7)
        .build();
    
    let result = graph.execute(state).await.unwrap();
    
    assert!(result.conviction_quotient.is_some());
    assert!(result.cq >= 0.0 && result.cq <= 1.0);
}
```

---

## Sprint 5: Temporal Core (2 седмици)

**Цел:** Durable workflow engine

### Компоненти
| Файл | Отговорност |
|------|-------------|
| `src/temporal/mod.rs` | Основни типове |
| `src/temporal/workflow.rs` | `Workflow` trait |
| `src/temporal/activity.rs` | `Activity` trait |
| `src/temporal/context.rs` | `WorkflowContext` |
| `src/temporal/saga.rs` | Saga pattern |

### Задачи
- [ ] `Workflow` trait с Input/Output types
- [ ] `Activity` trait за idempotent operations
- [ ] `WorkflowContext` за:
  - Activity execution
  - Timer/ Sleep
  - Signals
  - Queries
  - Child workflows
- [ ] Persistence layer (SQLx)
- [ ] Retry policy
- [ ] Saga pattern за compensation

### Golden Path Test
```rust
#[tokio::test]
async fn test_simple_workflow() {
    let worker = TestWorker::new();
    
    let result = worker
        .execute_workflow::<SimpleWorkflow>("test-1", TestInput { value: 42 })
        .await
        .unwrap();
    
    assert_eq!(result.value, 42);
}
```

---

## Sprint 6: Temporal Trading Workflows (2 седмици)

**Цел:** Trading workflows с durability

### Workflows
| Workflow | Отговорност |
|----------|-------------|
| `SignalGenerationWorkflow` | Пълен pipeline от data до signal |
| `OrderExecutionWorkflow` | Durable order с retry + compensation |
| `PhoenixTrainingWorkflow` | Paper trading с checkpoints |
| `DataCollectionWorkflow` | Periodic data fetch с persistence |

### Activities
- `FetchMarketData` — с retry при API failures
- `CallLLM` — с fallback между providers
- `CalculateCQ` — локално изчисление
- `PlaceOrder` — broker API с compensation
- `WriteJournal` — DB write

### Задачи
- [ ] SignalGeneration workflow
- [ ] OrderExecution workflow с Saga
- [ ] PhoenixTraining workflow с progress tracking
- [ ] Kill switch signal handling
- [ ] All activities с proper retry

### Golden Path Test
```rust
#[tokio::test]
async fn test_signal_workflow() {
    let worker = TestWorker::new();
    let input = SignalGenerationInput {
        ticker: "AAPL".to_string(),
        require_confirmation: false,
        min_cq: 0.7,
    };
    
    let result = worker
        .execute_workflow::<SignalGenerationWorkflow>("signal-1", input)
        .await
        .unwrap();
    
    assert!(result.confidence > 0.7);
}
```

---

## Sprint 7: Integration — Phoenix + LangGraph (2 седмици)

**Цел:** Phoenix engine използва LangGraph

### Integration Points
- Phoenix `train()` → стартира `PhoenixTrainingWorkflow`
- Phoenix `generate_strategy()` → използва `LLMChain`
- Phoenix `memory` → `VectorStoreMemory`

### Задачи
- [ ] Refactor PhoenixEngine да използва граф
- [ ] Phoenix nodes: `TrainEpochNode`, `AssessPerformanceNode`, `GraduateNode`
- [ ] Self-improvement loop в графа
- [ ] RAG integration за decision journal
- [ ] Migration на съществуващи Phoenix тестове

### Golden Path Test
```rust
#[tokio::test]
async fn test_phoenix_with_graph() {
    let engine = PhoenixEngine::new()
        .with_langgraph(trading_graph)
        .with_memory(rag_memory);
    
    let result = engine.train(TrainingConfig::default()).await.unwrap();
    
    assert!(result.epochs_completed > 0);
}
```

---

## Sprint 8: Integration — Execution + Temporal (2 седмици)

**Цел:** Всички orders са durable workflows

### Integration Points
- Order placement → `OrderExecutionWorkflow`
- Risk checks → activities
- Execution confirmation → signals
- Position updates → compensation

### Задачи
- [ ] Broker integration с Temporal
- [ ] Order state machine
- [ ] Partial fill handling
- [ ] Error recovery
- [ ] Integration със съществуващите broker модули

### Golden Path Test
```rust
#[tokio::test]
async fn test_durable_order() {
    let client = TemporalClient::new().await.unwrap();
    
    let handle = client
        .start_workflow::<OrderExecutionWorkflow>("order-1", order_input)
        .await
        .unwrap();
    
    // Simulate crash и recovery
    client.simulate_crash().await;
    client.recover().await;
    
    let result = handle.result().await.unwrap();
    assert_eq!(result.status, "FILLED");
}
```

---

## Sprint 9: Observability + Testing (2 седмици)

**Цел:** Visibility и надеждност

### Observability
- Tracing: `#[tracing::instrument]` на всички chains/nodes/workflows
- Metrics: 
  - `llm_requests_total`
  - `llm_tokens_total`
  - `graph_node_executions_total`
  - `workflow_duration_seconds`
  - `workflow_failures_total`
- Logging: Structured JSON logging

### Testing
- Unit tests: >80% coverage
- Integration tests: All workflows
- Property tests: State transitions
- Chaos tests: Simulate failures

### Задачи
- [ ] Tracing instrumentation
- [ ] Metrics collection
- [ ] Dashboard (Grafana)
- [ ] Тестове за всички edge cases
- [ ] Chaos engineering tests

---

## Sprint 10: Performance Optimization (2 седмици)

**Цел:** Speed + Efficiency

### Оптимизации
- LLM caching (Redis)
- Batch processing за activities
- Parallel node execution
- Connection pooling
- Database query optimization

### Benchmarks
| Метрика | Цел |
|---------|-----|
| Chain execution | <100ms без LLM |
| LLM + cache | <50ms |
| Graph execution | <500ms |
| Workflow start | <50ms |
| Activity retry | <1s backoff |

### Задачи
- [ ] Profiling и bottleneck идентификация
- [ ] Caching layer
- [ ] Batch optimizations
- [ ] Connection pools
- [ ] Benchmark suite

---

## Sprint 11: Documentation + Hardening (2 седмици)

**Цел:** Production ready

### Documentation
- [ ] API docs (cargo doc)
- [ ] Architecture docs
- [ ] Runbooks
- [ ] Troubleshooting guide

### Hardening
- [ ] Security audit
- [ ] Kill switch testing
- [ ] Failover procedures
- [ ] Backup/restore

### Final Acceptance
- [ ] Всички GP tests минават
- [ ] Код coverage ≥ 80%
- [ ] Clippy warnings = 0
- [ ] Documentation complete
- [ ] Load testing passed

---

## Timeline

```
Week:  1  2  3  4  5  6  7  8  9  10 11 12 13 14 15 16 17 18 19 20 21 22 23 24
       |--S0--|
       |-----S1-----|
            |-----S2-----|
                 |-----S3-----|
                      |-----S4-----|
                           |-----S5-----|
                                |-----S6-----|
                                     |-----S7-----|
                                          |-----S8-----|
                                               |-----S9-----|
                                                    |-----S10----|
                                                         |-----S11----|

Total: ~24 седмици (6 месеца)
```

---

## Risk Mitigation

| Риск | Sprint | Митигация |
|------|--------|-----------|
| Temporal Rust SDK не е готов | 5-6 | Наша имплементация с SQLx |
| LLM latency | 1-2 | Caching + timeouts |
| Graph complexity | 3-4 | Започваме с прости графове |
| Performance issues | 10 | Profiling early |
| Migration risks | 7-8 | Feature flags, rollback plan |

---

## Definition of Done (v3.5)

- [ ] Phoenix работи като Temporal workflow
- [ ] CQ се изчислява в LangGraph node
- [ ] RAG използва LangChain RAGChain
- [ ] Всички LLM извиквания минават през Chain
- [ ] Всички orders са durable workflows
- [ ] Всички съществуващи тестове минават
- [ ] Нови integration tests за всеки workflow
- [ ] Код покритие ≥ 80%
- [ ] Documentation ъпдейтната
- [ ] Performance benchmarks passed
