# Investor OS — AI Architecture Refactor (Pure Rust)

> **Документ:** RUST_AI_ARCHITECTURE.md  
> **Версия:** 1.0  
> **Дата:** 2026-02-09  
> **Статус:** RFC — За одобрение

---

## 1. Стратегия: Какво Подобряваме

### 1.1 Текущи Проблеми

| Проблем | Влияние | Решение |
|---------|---------|---------|
| Ръчни LLM интеграции | Дублиран код, няма fallback | `langchain` модул с unified API |
| Линейна CQ логика | Не адаптира според режим | `langgraph` state machine |
| Няма durable execution | Губим сигнали при рестарт | `temporal` workflows |
| Phoenix без памет | Не учи от грешки | RAG + LangGraph memory |
| Hardcoded prompts | Трудно за тестване | Template система с versioning |

### 1.2 Целево Състояние

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         Investor OS v3.5 (Rust AI Stack)                    │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐                  │
│  │  LangChain   │◄──►│  LangGraph   │◄──►│   Temporal   │                  │
│  │   "Bricks"   │    │   "Brain"    │    │  "Backbone"  │                  │
│  └──────┬───────┘    └──────┬───────┘    └──────┬───────┘                  │
│         │                   │                   │                          │
│         ▼                   ▼                   ▼                          │
│  ┌─────────────────────────────────────────────────────────────┐          │
│  │              Unified AI Runtime (Rust)                       │          │
│  │  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐         │          │
│  │  │  Chains │  │  Graph  │  │Workflows│  │ Memory  │         │          │
│  │  │ Prompts │  │  Nodes  │  │ Signals │  │  RAG    │         │          │
│  │  │  Tools  │  │  Edges  │  │ Retries │  │ Journal │         │          │
│  │  └─────────┘  └─────────┘  └─────────┘  └─────────┘         │          │
│  └─────────────────────────────────────────────────────────────┘          │
│                              │                                               │
│                              ▼                                               │
│  ┌─────────────────────────────────────────────────────────────┐          │
│  │                    Phoenix Engine v2.0                       │          │
│  │         (Self-learning + Durable + Observable)               │          │
│  └─────────────────────────────────────────────────────────────┘          │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 2. Архитектура: Как Ще Го Направим

### 2.1 LangChain (src/langchain/) — "Тухлите"

**Отговорност:** Композируеми AI компоненти

```rust
// Цел: Всеки LLM извикване е chain
pub trait Chain: Send + Sync {
    async fn run(&self, ctx: ChainContext) -> Result<ChainResult, ChainError>;
}

// Видове chains:
// - LLMChain: prompt → LLM → output
// - SequentialChain: chain1 → chain2 → chain3
// - ParallelChain: [chainA, chainB, chainC] → merge
// - AgentChain: LLM с tools (ReAct pattern)
// - RAGChain: retrieve → prompt → LLM
```

**Примерен Usage:**
```rust
// Преди (ръчно):
let prompt = format!("Analyze {} based on {}", ticker, data);
let response = claude.generate(&prompt).await?;

// След (chain):
let analysis_chain = ChainBuilder::new()
    .with_llm(LLMProvider::Claude(claude))
    .with_template(sec_analysis_prompt())
    .with_parser(JsonParser::<SecAnalysis>::new())
    .build()?;

let result = analysis_chain.run(ctx! {
    "ticker": ticker,
    "filing_content": sec_text
}).await?;
```

**Компоненти:**
- `chains.rs` — Chain trait + имплементации
- `prompts.rs` — Template система с валидация
- `tools.rs` — Tool trait + Trading tools
- `parsers.rs` — Structured output parsing
- `memory.rs` — Conversation + Vector memory

### 2.2 LangGraph (src/langgraph/) — "Мозъкът"

**Отговорност:** State machine за decision flow

```rust
// Цел: Phoenix работи като граф
pub trait Node: Send + Sync {
    fn name(&self) -> &str;
    async fn execute(&self, state: SharedState) -> Result<NodeOutput, GraphError>;
}

pub trait Edge: Send + Sync {
    fn condition(&self, state: &SharedState) -> bool;
    fn target(&self) -> &str;
}

// Граф се дефинира декларативно:
let trading_graph = GraphBuilder::new()
    .add_node("data_collection", DataCollectionNode)
    .add_node("regime_detection", RegimeNode)
    .add_node("breakout_strategy", BreakoutStrategyNode)
    .add_node("mean_reversion", MeanReversionNode)
    .add_node("cq_calculation", CQNode)
    .add_node("risk_check", RiskNode)
    .add_node("execute", ExecutionNode)
    
    // Edges
    .add_edge("start", "data_collection")
    .add_edge("data_collection", "regime_detection")
    
    // Conditional edges
    .add_conditional_edge("regime_detection", 
        |state| state.regime == MarketRegime::Trending,
        "breakout_strategy"
    )
    .add_conditional_edge("regime_detection",
        |state| state.regime == MarketRegime::RangeBound,
        "mean_reversion"
    )
    
    .add_edge("breakout_strategy", "cq_calculation")
    .add_edge("mean_reversion", "cq_calculation")
    .add_edge("cq_calculation", "risk_check")
    
    .add_conditional_edge("risk_check",
        |state| state.cq.value() > 0.7 && state.risk_approved,
        "execute"
    )
    
    .build()?;
```

**Състояние (Shared State):**
```rust
#[derive(Clone)]
pub struct TradingState {
    // Input
    pub ticker: String,
    pub timestamp: DateTime<Utc>,
    
    // Market Data
    pub price: Decimal,
    pub ohlcv: Vec<OHLCV>,
    pub regime: MarketRegime,
    
    // Signals
    pub quality_score: Score,
    pub insider_score: Score,
    pub sentiment_score: Score,
    pub breakout_score: Score,
    pub atr_trend: Score,
    
    // Decision
    pub cq: ConvictionQuotient,
    pub action: Option<Action>,
    pub confidence: Score,
    
    // Risk
    pub risk_approved: bool,
    pub position_size: Decimal,
    
    // Execution
    pub order_id: Option<String>,
    pub execution_price: Option<Decimal>,
    
    // Meta
    pub node_history: Vec<String>,
    pub llm_calls: u32,
    pub tool_calls: u32,
}
```

**Цикли (Loops):**
```rust
// Phoenix self-improvement loop
graph.add_loop(
    "assess_performance",
    |state| state.should_retrain(),
    "regime_detection"  // Връща се за нова стратегия
);
```

### 2.3 Temporal (src/temporal/) — "Гръбнакът"

**Отговорност:** Durable, reliable execution

```rust
// Workflow trait
#[async_trait]
pub trait Workflow: Send + Sync {
    type Input: Serialize + DeserializeOwned;
    type Output: Serialize + DeserializeOwned;
    
    async fn run(&self, ctx: WorkflowContext, input: Self::Input) -> Result<Self::Output, WorkflowError>;
}

// Пример: Signal Generation Workflow
pub struct SignalGenerationWorkflow {
    trading_graph: Arc<TradingGraph>,
}

#[async_trait]
impl Workflow for SignalGenerationWorkflow {
    type Input = SignalRequest;
    type Output = SignalResult;
    
    async fn run(&self, ctx: WorkflowContext, input: Self::Input) -> Result<Self::Output, WorkflowError> {
        // 1. Проверка за kill switch (queryable)
        if ctx.query::<bool>("kill_switch").await? {
            return Ok(SignalResult::killed());
        }
        
        // 2. Проверка за data freshness (activity)
        let data_fresh = ctx.activity(
            CheckDataFreshness,
            CheckDataFreshnessInput { ticker: input.ticker.clone() }
        ).await?;
        
        if !data_fresh {
            return Ok(SignalResult::skipped("Stale data"));
        }
        
        // 3. Изпълнение на trading graph (може да е дълго)
        let state = ctx.activity(
            RunTradingGraph,
            RunTradingGraphInput {
                ticker: input.ticker,
                graph: self.trading_graph.clone(),
            }
        ).await?;
        
        // 4. Чакаме confirmation ако е high conviction
        if state.cq.value() > 0.8 {
            // Сигнал към потребителя, чака отговор
            let confirmed = ctx.signal::<bool>("user_confirmation")
                .with_timeout(Duration::minutes(5))
                .await?;
            
            if !confirmed {
                return Ok(SignalResult::rejected("User declined"));
            }
        }
        
        // 5. Execution workflow (отделен за надеждност)
        let execution = ctx.child_workflow(
            ExecutionWorkflow,
            ExecutionInput {
                ticker: state.ticker,
                action: state.action.unwrap(),
                quantity: state.position_size,
            }
        ).await?;
        
        Ok(SignalResult::success(execution))
    }
}
```

**Activities:**
```rust
// Activities са idempotent и могат да се retry-ват
#[activity]
pub async fn fetch_market_data(input: FetchMarketDataInput) -> Result<MarketData, ActivityError> {
    // HTTP call с retry логика
}

#[activity]
pub async fn call_llm(input: LLMInput) -> Result<LLMOutput, ActivityError> {
    // LLM call с fallback
}

#[activity]
pub async fn place_order(input: OrderInput) -> Result<OrderResult, ActivityError> {
    // Broker API call
}

#[activity]
pub async fn write_to_journal(input: JournalEntry) -> Result<(), ActivityError> {
    // Database write
}
```

**Saga Pattern за Orders:**
```rust
// Ако execution fail-ва, компенсираме
let saga = Saga::new()
    .step("reserve_funds", reserve_funds, release_funds)
    .step("place_order", place_order, cancel_order)
    .step("confirm_fill", confirm_fill, cancel_order)
    .step("update_portfolio", update_portfolio, revert_portfolio)
    .step("journal_entry", write_journal, delete_journal_entry);

saga.execute(ctx).await?;
```

---

## 3. Интеграция със Съществуващи Компоненти

### 3.1 Phoenix Engine v2.0

```rust
// Phoenix сега използва LangGraph + Temporal
pub struct PhoenixEngine {
    graph: Arc<TradingGraph>,
    workflow_client: TemporalClient,
    memory: Arc<RagMemory>,
}

impl PhoenixEngine {
    pub async fn train(&self, config: TrainingConfig) -> Result<TrainingResult> {
        // Стартираме training workflow
        let workflow_id = format!("phoenix-training-{}", Uuid::new_v4());
        
        let result = self.workflow_client
            .start_workflow::<TrainingWorkflow>(
                &workflow_id,
                TrainingInput {
                    config,
                    initial_strategy: self.generate_initial_strategy().await?,
                }
            )
            .await?;
        
        // Следим progress чрез queries
        loop {
            let progress = self.workflow_client
                .query_workflow::<TrainingProgress>(&workflow_id, "progress")
                .await?;
            
            if progress.is_complete {
                break;
            }
            
            tokio::time::sleep(Duration::seconds(5)).await;
        }
        
        Ok(result)
    }
    
    async fn generate_initial_strategy(&self) -> Result<Strategy> {
        // Използваме LangChain за генериране
        let chain = self.build_strategy_chain()?;
        let ctx = ChainContext::new()
            .with_variable("regime", "bullish_volatility")
            .with_variable("performance", "previous_results.json");
        
        let result = chain.run(ctx).await?;
        let strategy: Strategy = serde_json::from_str(&result.output)?;
        
        Ok(strategy)
    }
}
```

### 3.2 RAG Integration

```rust
// RAGChain използва neurocod-rag
pub struct InvestorRagChain {
    llm: LLMProvider,
    retriever: Arc<NeurocodRetriever>, // от neurocod-rag
}

impl Chain for InvestorRagChain {
    async fn run(&self, ctx: ChainContext) -> Result<ChainResult, ChainError> {
        let query = ctx.get("query").ok_or(...)?;
        let ticker = ctx.get("ticker").ok_or(...)?;
        
        // Retrieve от pgvector
        let docs = self.retriever
            .search(&SearchQuery {
                query: query.to_string(),
                ticker: ticker.to_string(),
                limit: 5,
            })
            .await?;
        
        // Prompt с контекст
        let prompt = format!(
            "Based on these SEC filings:\n{}\n\nAnswer: {}",
            format_docs(&docs),
            query
        );
        
        let response = self.llm.generate(&prompt).await?;
        
        Ok(ChainResult {
            output: response,
            parsed_output: None,
            metadata: ExecutionMetadata {
                // ...
            },
        })
    }
}
```

### 3.3 CQ Formula v2.5

```rust
// Сега CQ е node в графа
pub struct CQCalculationNode;

#[async_trait]
impl Node for CQCalculationNode {
    fn name(&self) -> &str { "cq_calculation" }
    
    async fn execute(&self, state: SharedState) -> Result<NodeOutput, GraphError> {
        // Може да използва LLM за някои фактори
        let sentiment_chain = build_sentiment_chain()?;
        let sentiment_result = sentiment_chain.run(ctx! {
            "news": state.news_articles.clone(),
            "social": state.social_posts.clone(),
        }).await?;
        
        let sentiment_score: Score = serde_json::from_str(&sentiment_result.output)?;
        
        // Calculate CQ
        let cq = ConvictionQuotient::builder()
            .pegy_relative(state.quality_score)
            .insider_score(state.insider_score)
            .sentiment_score(sentiment_score)
            .regime_fit(state.regime_fit)
            .breakout_score(state.breakout_score)
            .atr_trend(state.atr_trend)
            .calculate();
        
        // Update state
        let mut new_state = state;
        new_state.cq = cq;
        
        Ok(NodeOutput::Continue(new_state))
    }
}
```

---

## 4. Dependencies

```toml
[dependencies]
# Съществуващи
neurocod-rag = { path = "../neurocod-rag" }
tokio = { version = "1.48", features = ["rt-multi-thread", "macros"] }

# LangChain-like (ще ги пишем ние)
# - няма нужда от външни crates,我们自己的实现

# LangGraph-like (ще ги пишем ние)  
# - state machine: petgraph = "0.6"

# Temporal
 temporal-sdk = "0.1"  # или rust-sdk когато е готов
# или наша имплементация с:
# - sqlx за persistence
# - tokio::time за timers
# - channels за signals

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Async traits
async-trait = "0.1"

# Type safety
thiserror = "2.0"

# Testing
mockall = "0.13"
```

---

## 5. Тестване

```rust
// Unit tests
#[tokio::test]
async fn test_cq_calculation_node() {
    let node = CQCalculationNode;
    let state = TradingState::mock();
    
    let result = node.execute(state).await.unwrap();
    
    assert!(result.cq.value() >= 0.0);
    assert!(result.cq.value() <= 1.0);
}

// Integration tests (с mocked LLM)
#[tokio::test]
async fn test_trading_graph() {
    let graph = build_test_graph().await;
    let result = graph.run(TradingState::mock()).await.unwrap();
    
    assert!(result.action.is_some());
}

// Temporal workflow tests
#[tokio::test]
async fn test_signal_workflow() {
    let worker = TestWorker::new();
    
    let result = worker
        .execute_workflow::<SignalGenerationWorkflow>(SignalRequest {
            ticker: "AAPL".to_string(),
        })
        .await;
    
    assert!(result.is_ok());
}
```

---

## 6. Мониторинг и Observability

```rust
// Tracing през целия stack
#[tracing::instrument(skip(self, ctx))]
async fn run(&self, ctx: ChainContext) -> Result<ChainResult, ChainError> {
    tracing::info!(chain_type = "LLM", prompt_hash = hash(&prompt));
    
    let result = self.llm.generate(&prompt).await?;
    
    tracing::info!(
        tokens_used = result.metadata.tokens_used,
        execution_time_ms = result.metadata.execution_time_ms,
        "Chain completed"
    );
    
    Ok(result)
}

// Metrics
// - llm_requests_total
// - llm_tokens_total
// - graph_node_executions_total
// - workflow_duration_seconds
// - workflow_failures_total
```

---

## 7. Рискове и Митигация

| Риск | Вероятност | Влияние | Митигация |
|------|-----------|---------|-----------|
| Temporal Rust SDK не е production-ready | Средна | Високо | Наша имплементация с sqlx + tokio |
| Сложност на графа | Средна | Средно | Започваме с прости linear graphs |
| Performance на RAG | Ниска | Средно | pgvector + кеширане |
| LLM latency | Висока | Средно | Async + caching + timeouts |

---

## 8. Приемственост

- ✅ **Phoenix engine** → обвиваме в workflow
- ✅ **CQ formula** → стават nodes
- ✅ **RAG** → RAGChain
- ✅ **ML APIs** → LLMProvider trait
- ✅ **Kill switch** → Signal в workflows
- ✅ **Golden Path tests** → Пазим ги, добавяме нови

---

## 9. Дефиниция на "Готово"

- [ ] Всички съществуващи тестове минават
- [ ] Phoenix работи като Temporal workflow
- [ ] CQ се изчислява в LangGraph node
- [ ] RAG използва LangChain RAGChain
- [ ] Всички LLM извиквания минават през Chain
- [ ] Всички orders са durable workflows
- [ ] Код покритие ≥ 80%
- [ ] Документацията е ъпдейтната
