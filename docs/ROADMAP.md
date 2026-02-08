# Investor OS - Roadmap & Next Steps

> **Date:** 2026-02-08 | **Version:** 1.0 | **Status:** MVP Complete

---

## ✅ Completed (MVP v1.0)

### Sprint 1: Foundation
- [x] PostgreSQL + TimescaleDB setup
- [x] Docker Compose infrastructure
- [x] Data collectors (Finnhub, Yahoo)
- [x] Golden Path tests

### Sprint 2: Signals
- [x] QVM Scorer (Quality, Value, Momentum)
- [x] Insider Scorer (Form 4 analysis)
- [x] Sentiment Scorer (VADER)
- [x] Regime Detector (HMM)
- [x] 18 Golden Path tests passing

### Sprint 3: Decision Engine
- [x] CQ v2.0 calculation (6-factor)
- [x] Position sizing by regime
- [x] Constraint checking
- [x] Trade proposals lifecycle
- [x] Decision journal
- [x] Axum REST API
- [x] 15 Golden Path tests passing

### Sprint 4: Interface + Monitoring
- [x] Next.js 15 Dashboard
- [x] Proposals/Positions/Journal pages
- [x] Grafana dashboards (System, Portfolio, Pipeline)
- [x] Alert rules
- [x] Kill switch UI
- [x] Deployment guide & Runbook
- [x] 10 E2E tests (Playwright)

---

## 🚀 Phase 2: PostgreSQL Performance Optimization

### TimescaleDB Deep Optimization

Already integrated, but needs tuning:

```sql
-- 1. Hypertable optimization
SELECT create_hypertable('prices', 'timestamp', 
    chunk_time_interval => INTERVAL '1 day',
    if_not_exists => TRUE
);

-- 2. Compression for old data
ALTER TABLE prices SET (
    timescaledb.compress,
    timescaledb.compress_segmentby = 'ticker'
);

-- Add compression policy (compress after 7 days)
SELECT add_compression_policy('prices', INTERVAL '7 days');

-- 3. Retention policy (keep 2 years)
SELECT add_retention_policy('prices', INTERVAL '2 years');
```

### PostgreSQL Extensions to Add

| Extension | Purpose | Impact |
|-----------|---------|--------|
| **pg_stat_statements** | Query performance tracking | Identify slow queries |
| **auto_explain** | Auto-log slow queries | Debugging |
| **pg_prewarm** | Preload tables into RAM | Faster startup |
| **pg_trgm** | Fast text search | SEC filings search |
| **btree_gist** | GiST indexes | Range queries |
| **uuid-ossp** | UUID generation | Better IDs |

### Query Optimization

```sql
-- Materialized view for daily portfolio snapshots
CREATE MATERIALIZED VIEW portfolio_daily AS
SELECT 
    date_trunc('day', timestamp) as day,
    ticker,
    avg(close) as avg_close,
    max(high) as day_high,
    min(low) as day_low,
    sum(volume) as total_volume
FROM prices
GROUP BY 1, 2;

-- Index for CQ calculations
CREATE INDEX idx_signals_cq ON signals (ticker, calculated_at) 
INCLUDE (cq_score, regime_fit);

-- Partial index for pending proposals
CREATE INDEX idx_pending_proposals ON proposals (created_at) 
WHERE status = 'PENDING';
```

---

## 🧠 Phase 3: RAG Integration (Sprint 5)

### neurocod-rag Integration

```rust
// Add to Cargo.toml
[dependencies]
neurocod-rag = { path = "../neurocod-rag", features = ["pgvector"] }
```

### Use Cases

1. **SEC Filings Analysis**
   - Chunk 10-K, 10-Q documents
   - Vector embeddings for semantic search
   - RAG for earnings call Q&A

2. **News Aggregation**
   - Real-time news ingestion
   - Sentiment scoring with context
   - Similarity search for related events

3. **Decision Journal AI**
   - Search past decisions by description
   - Pattern recognition in outcomes
   - "Why did I lose on tech stocks?"

### Implementation

```rust
pub struct FinancialRag {
    client: neurocod_rag::Client,
    chunker: EarningsChunker,
}

impl FinancialRag {
    pub async fn analyze_earnings(&self, ticker: &str) -> Result<Analysis> {
        let context = self.client.search(
            &format!("{} earnings risks opportunities", ticker),
            SearchOptions::default().limit(5)
        ).await?;
        
        // Generate insights using context
        self.generate_insights(context).await
    }
}
```

---

## 🔌 Phase 4: Broker Integration (Sprint 6)

### Interactive Brokers (IBKR)

```rust
pub trait Broker {
    async fn place_order(&self, order: Order) -> Result<OrderId>;
    async fn get_positions(&self) -> Result<Vec<Position>>;
    async fn get_account(&self) -> Result<Account>;
    async fn cancel_order(&self, id: OrderId) -> Result<()>;
}

pub struct IBKRClient {
    client: ibapi::Client,
}
```

### Paper Trading Mode

- Separate "paper" portfolio
- Same CQ logic, simulated execution
- Track paper vs real performance

### Order Management

- OCO (One-Cancels-Other) orders
- Trailing stops
- Position scaling (add/reduce)

---

## 📊 Phase 5: Advanced Analytics (Sprint 7)

### Backtesting Framework

```rust
pub struct Backtest {
    start_date: DateTime<Utc>,
    end_date: DateTime<Utc>,
    initial_capital: Money,
    strategy: Box<dyn Strategy>,
}

impl Backtest {
    pub async fn run(&self) -> BacktestResult {
        // Walk-forward analysis
        // Transaction cost modeling
        // Slippage simulation
    }
}
```

### Risk Analytics

- VaR (Value at Risk) calculation
- Sharpe ratio tracking
- Maximum consecutive losses
- Drawdown analysis

### ML Enhancements

- Feature engineering from signals
- XGBoost for CQ prediction
- Anomaly detection for regime changes

---

## 🔒 Phase 6: Production Hardening (Sprint 8)

### Kubernetes Deployment

```yaml
# k8s/api-deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: investor-api
spec:
  replicas: 3
  template:
    spec:
      containers:
      - name: api
        image: investor-os/api:latest
        resources:
          requests:
            memory: "512Mi"
            cpu: "500m"
          limits:
            memory: "2Gi"
            cpu: "2000m"
```

### CI/CD Pipeline

```yaml
# .github/workflows/deploy.yml
name: Deploy
on:
  push:
    branches: [main]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Run tests
        run: cargo test
      - name: E2E tests
        run: cd frontend && npm run test:e2e
  deploy:
    needs: test
    runs-on: ubuntu-latest
    steps:
      - name: Deploy to production
        run: kubectl apply -f k8s/
```

### Security

- [ ] Secrets management (Vault)
- [ ] mTLS between services
- [ ] API rate limiting
- [ ] Audit logging
- [ ] SOC 2 compliance prep

---

## 📅 Proposed Schedule

| Phase | Duration | Focus |
|-------|----------|-------|
| Phase 2 | Week 9 | PostgreSQL optimization |
| Phase 3 | Week 10-11 | RAG integration |
| Phase 4 | Week 12-13 | Broker integration |
| Phase 5 | Week 14-15 | Advanced analytics |
| Phase 6 | Week 16-17 | Production hardening |

---

## 🎯 Immediate Next Steps (Priority Order)

1. **Add pg_stat_statements extension**
   ```bash
   docker compose exec postgres psql -U investor -c "CREATE EXTENSION IF NOT EXISTS pg_stat_statements;"
   ```

2. **Enable TimescaleDB compression**
   ```sql
   -- Add to migration file
   SELECT add_compression_policy('prices', INTERVAL '7 days');
   ```

3. **Create materialized views for CQ aggregations**

4. **Start RAG integration with neurocod-rag**

5. **Research IBKR API credentials**

---

## 💡 Innovation Ideas (Future)

- **Options flow analysis** (Unusual Whales integration)
- **Alternative data** (satellite imagery, credit card data)
- **Social listening** (Reddit, Twitter sentiment)
- **Multi-strategy support** (Value + Momentum + Mean-reversion)
- **Mobile app** (React Native)

---

## 📚 Resources

- [TimescaleDB Best Practices](https://docs.timescale.com/use-timescale/latest/hypertables/about-hypertables/)
- [PostgreSQL Query Optimization](https://www.postgresql.org/docs/current/performance-tips.html)
- [IBKR API Docs](https://interactivebrokers.github.io/tws-api/)
- [neurocod-rag Integration Guide](../integration/RAG.md)

---

*Last updated: 2026-02-08*
