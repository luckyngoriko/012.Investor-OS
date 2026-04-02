# Plan: Prototype → Working Platform

## Date: 2026-03-30

## Status: PENDING — scheduled for 2026-03-31

---

## Current State (Prototype)

- Rust backend running on TrueNAS (5 containers healthy)
- ML sidecar with FinBERT + GARCH loaded, CatBoost awaiting training
- Frontend with predictions dashboard deployed
- Binance WebSocket streaming (BTCUSDT) — ticks arrive but don't persist
- All trading data is hardcoded/simulated — no real data flows through the system
- 3 new DB tables (ml_model_registry, ml_predictions, ml_feature_store) not yet created on production DB

---

## Phase A — Database & Data Foundation (Day 1 morning)

### A1. Run ML migrations on production postgres

- SSH to TrueNAS, exec into ios-postgres
- Execute: 20260330000001_ml_model_registry.sql
- Execute: 20260330000002_ml_predictions.sql
- Execute: 20260330000003_ml_feature_store.sql
- Verify tables exist with \dt

### A2. Verify prices table has TimescaleDB hypertable

- Check if prices table exists and is a hypertable
- If not, create it with OHLCV schema + hypertable conversion
- Add indexes on (symbol, timestamp)

### A3. Wire streaming → prices persistence

- Currently: Binance WS ticks arrive → OrderBook + TradeAnalyzer → signals
- Need: ticks also INSERT into prices table (aggregated 1m candles)
- File: src/streaming/mod.rs — add DB write in tick_processing_loop
- This is the CRITICAL missing link — without persisted prices, nothing else works

---

## Phase B — Feature Pipeline (Day 1 afternoon)

### B1. Automated feature computation

- Create background task in Rust that runs every 5 minutes:
  1. Read last 500 candles from prices table for each symbol
  2. Call compute_technical_features() (already implemented in Sprint 118)
  3. Store result in ml_feature_store table
- File: src/prediction/features/pipeline.rs (new)

### B2. Wire feature store to prediction endpoints

- When POST /api/predictions/predict arrives:
  1. Read latest features from ml_feature_store (instead of requiring client to send them)
  2. If features are stale (>10min), compute fresh from prices table
  3. Send to sidecar with features populated

---

## Phase C — Train Real Models (Day 1 evening)

### C1. Collect enough data

- Let streaming run for several hours to accumulate prices
- Need at least 500+ candles per symbol for meaningful CatBoost training
- Can also backfill from Binance REST API (GET /api/v3/klines)

### C2. Create backfill script

- scripts/ml/backfill_prices.py — fetch historical OHLCV from Binance API
- Store in prices table via direct postgres insert
- Target: 30 days of 1h candles = 720 candles per symbol

### C3. Train CatBoost on real data

- Modify train_catboost.py to read from DB instead of synthetic data
- Compute features from real prices
- Target variable: actual next-period return
- Train/val split: walk-forward (time-based, not random)
- Save model to /mnt/MainPool/appsdata/investor-os.net/models/
- Register in ml_model_registry

### C4. Verify GARCH with real returns

- Feed real returns from prices table to GARCH endpoint
- Verify VaR/CVaR are reasonable for the actual asset

### C5. Test FinBERT with real news

- Create a simple news fetcher (CoinGecko news API or NewsAPI free tier)
- Feed headlines to /v1/predict/finbert
- Store sentiment scores in ml_feature_store as feature_set='sentiment'

---

## Phase D — Prediction Loop (Day 2 morning)

### D1. Scheduled prediction service

- Background task in Rust or Python cron:
  1. Every hour: fetch latest features for each tracked symbol
  2. Call CatBoost prediction → store in ml_predictions
  3. Call GARCH volatility → store in ml_predictions
  4. Call FinBERT on latest news → store sentiment
  5. Run consensus engine → store combined score
  6. Update TickerSignals.ml_prediction_score

### D2. Accuracy resolution

- Connect accuracy_tracker.py to real DB
- Every hour: find predictions where horizon elapsed
- Look up actual price from prices table
- Compute error_metric, fill actual_value
- Refresh mv_ml_model_accuracy materialized view

### D3. Frontend shows real data

- /predictions page already fetches from API
- Once predictions flow → dashboard shows real confidence, real models, real history

---

## Phase E — Broker Integration (Day 2 afternoon)

### E1. Binance Spot API integration

- Create src/broker/binance.rs implementing Broker trait
- API key from env vars (BINANCE_API_KEY, BINANCE_API_SECRET)
- Start with read-only: account balance, open orders, positions
- Paper trading mode: log orders but don't execute

### E2. Trade execution flow

- Prediction consensus > 0.7 confidence + model agreement > 80%
- Generate trade proposal → store in proposals table
- Frontend shows proposal → user confirms/rejects
- On confirm: execute via Binance API (or paper trade)

### E3. Position tracking

- Sync positions from Binance account
- Track P&L in real-time using streaming prices
- Update portfolio metrics

---

## Phase F — Production Hardening (Day 2 evening)

### F1. HTTPS + Reverse Proxy

- nginx container with Let's Encrypt SSL
- Proxy: investor-os.net → frontend:3000
- Proxy: investor-os.net/api → api:8080
- WebSocket proxy for streaming

### F2. Monitoring & Alerts

- Register ML Prometheus metrics (Sprint 135 — already implemented)
- Add Grafana dashboard for:
  - Prediction latency
  - Model accuracy over time
  - Sidecar health
  - Streaming lag

### F3. Backup & Recovery

- Automated postgres backup (pg_dump) to ZFS snapshot
- Model files backup
- Configuration backup

### F4. Rate limiting & Security

- ML sidecar: internal-only (no external port)
- API: rate limit prediction endpoints (10/min per user)
- Input validation on all prediction requests

---

## Priority Order

| Priority | Task                              | Impact                                | Effort |
| -------- | --------------------------------- | ------------------------------------- | ------ |
| 1        | A1-A3: DB + streaming persistence | CRITICAL — nothing works without data | Medium |
| 2        | C2: Backfill historical prices    | HIGH — enables model training         | Low    |
| 3        | B1-B2: Feature pipeline           | HIGH — automates feature computation  | Medium |
| 4        | C3: Train CatBoost on real data   | HIGH — real predictions               | Medium |
| 5        | D1: Prediction loop               | HIGH — automated forecasting          | Medium |
| 6        | D2-D3: Accuracy + frontend        | MEDIUM — validates system             | Low    |
| 7        | E1-E3: Broker integration         | MEDIUM — enables trading              | High   |
| 8        | F1-F4: Production hardening       | LOW (staging) — needed for production | Medium |

---

## Success Criteria

The platform is "working" when:

1. Real prices flow from Binance → prices table (TimescaleDB)
2. Features are computed automatically every 5 minutes
3. CatBoost generates real predictions with >0 accuracy
4. GARCH produces real VaR numbers for BTC
5. FinBERT analyzes real news headlines
6. Consensus engine combines all models
7. /predictions dashboard shows real, updating data
8. Accuracy tracker validates predictions after horizon elapses

---

## Files to Create/Modify

| File                                              | Action                                                 |
| ------------------------------------------------- | ------------------------------------------------------ |
| src/streaming/mod.rs                              | Modify — add price persistence to tick_processing_loop |
| src/prediction/features/pipeline.rs               | Create — scheduled feature computation                 |
| scripts/ml/backfill_prices.py                     | Create — fetch historical OHLCV from Binance           |
| scripts/ml/train_catboost.py                      | Modify — read from DB instead of synthetic             |
| scripts/ml/fetch_news.py                          | Create — news headlines for FinBERT                    |
| services/ml-sidecar/app/tasks/prediction_loop.py  | Create — scheduled prediction service                  |
| services/ml-sidecar/app/tasks/accuracy_tracker.py | Modify — connect to real DB                            |
| src/broker/binance.rs                             | Create — Binance API integration                       |
