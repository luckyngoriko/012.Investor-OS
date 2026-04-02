# Investor OS — World-Class AI Trading Platform Design

## Date: 2026-04-02

## Status: APPROVED

---

## 1. Vision

Transform Investor OS from a prototype into a production SaaS AI trading platform that:

- Trades **all asset classes** (crypto, forex, stocks, options)
- Serves **many users** with subscription tiers (Free / Pro / Enterprise)
- Offers **3 autonomy modes** per strategy (Signal / Semi-Auto / Full-Auto)
- Enables **real money trading** via user-connected broker accounts
- Includes a **strategy marketplace** for copy-trading (Phase 2)

## 2. Revenue Model

- **Subscription**: Free ($0) / Pro ($79/mo) / Enterprise ($299/mo)
- **Marketplace commission** (Phase 2): 20% of strategy subscription fees
- No performance fees (regulatory complexity)

### Tier Feature Matrix

| Feature     | Free             | Pro                | Enterprise          |
| ----------- | ---------------- | ------------------ | ------------------- |
| Brokers     | 1                | 3                  | Unlimited           |
| Trading     | Paper only       | Live               | Live + Priority     |
| Models      | CatBoost + GARCH | All 8 models       | All + custom        |
| Symbols     | 2                | 20                 | Unlimited           |
| HRM         | No               | Yes                | Yes + GPU priority  |
| Modes       | Signal only      | Signal + Semi-Auto | All 3               |
| API access  | No               | Read-only          | Full                |
| Marketplace | Browse           | Browse + Buy       | Browse + Buy + Sell |

## 3. Architecture

### 3.1 System Components

```
Users (Web / Mobile / API)
         │ HTTPS
         ▼
    nginx (SSL, rate limit)
         │
    ┌────┼────┐
    ▼    ▼    ▼
 Next.js  Rust API  WS Gateway
 (SSR)   (Core)    (per-user push)
         │
         ▼
    ios-nats (JetStream)
    ┌──┬──┬──┬──┬──┬──┐
    ▼  ▼  ▼  ▼  ▼  ▼  ▼
  Feature CatBoost GARCH FinBERT Chronos HRM Consensus
  (Rust)  (CPU)   (CPU)  (CPU)  (GPU)  (GPU) (Rust)
                                              │
                                         Trade Proposer
                                              │
                                         Broker Gateway
                                    ┌────┬────┬────┐
                                    ▼    ▼    ▼    ▼
                                 Binance IBKR OANDA Paper
                                              │
                                         PostgreSQL
                                        (TimescaleDB)
```

### 3.2 NATS Event Subjects (existing + new)

```
# Existing (implemented)
ios.prices.{symbol}              — OHLCV candles
ios.features.{symbol}            — technical indicators
ios.predict.{model}.{symbol}     — model predictions
ios.sentiment.{symbol}           — FinBERT sentiment
ios.consensus.{symbol}           — combined consensus
ios.trade.proposal.{symbol}      — trade proposals

# New (to build)
ios.trade.execute.{user_id}      — order execution events
ios.trade.fill.{user_id}         — fill confirmations
ios.user.signal.{user_id}        — per-user signal delivery
ios.strategy.update.{user_id}    — strategy config changes
ios.portfolio.rebalance.{user_id} — rebalance triggers
ios.alert.{user_id}              — risk alerts, margin calls
```

### 3.3 Multi-Tenancy Model

- `tenant_id` (UUID) on all user-facing tables
- Row-Level Security (RLS) policies in PostgreSQL
- API middleware extracts tenant from JWT
- NATS subjects include user_id for per-user events
- Broker API keys encrypted per-user with AES-256-GCM

### 3.4 New Database Tables

```sql
-- User strategy configuration
user_strategies (
  id, user_id, name, symbols[], models[], mode, risk_limits JSONB,
  is_active, created_at
)

-- Encrypted broker credentials
broker_connections (
  id, user_id, broker_type, encrypted_api_key, encrypted_api_secret,
  permissions[], is_active, last_verified_at
)

-- Subscription billing
subscriptions (
  id, user_id, tier, stripe_customer_id, stripe_subscription_id,
  status, current_period_end, created_at
)

-- User trade history
user_trades (
  id, user_id, strategy_id, broker_type, symbol, side, quantity,
  price, commission, pnl, executed_at
)

-- Marketplace (Phase 2)
strategy_listings (
  id, creator_id, name, description, performance JSONB,
  price_monthly, subscribers_count, is_public
)
```

## 4. Real Money Integration

### Phase 1: Connect Your Broker (Week 1-2)

- User enters Binance API key + secret in settings UI
- Keys encrypted with AES-256-GCM, stored in broker_connections
- Key permissions validated (read + trade, no withdraw)
- Paper trading by default, user explicitly enables live
- Kill switch per user — instant stop all trading

### Phase 2: Multi-Broker (Month 1-3)

- Interactive Brokers (stocks, options) via TWS API
- OANDA (forex) via REST API
- Unified portfolio view across all connected brokers
- Smart Order Router: compare prices, execute on best venue

### Phase 3: Custody (Month 3-6)

- Fireblocks MPC custody for direct crypto deposits
- KYC/AML via Sumsub
- Fiat on-ramp via Stripe Connect
- Regulatory compliance (MiFID II / MiCA)

## 5. Intelligence Layer

### 5.1 Current Models (operational)

| Model            | Type                   | Tier     | Location   |
| ---------------- | ---------------------- | -------- | ---------- |
| CatBoost         | Return prediction      | 1 (CPU)  | ml-sidecar |
| GARCH            | Volatility + VaR       | 1 (CPU)  | ml-sidecar |
| FinBERT          | News sentiment         | 1 (CPU)  | ml-sidecar |
| skfolio          | Portfolio optimization | 1 (CPU)  | ml-sidecar |
| augurs ETS/MSTL  | Time series forecast   | 1 (Rust) | api        |
| Chronos-2        | Zero-shot forecast     | 2 (GPU)  | ml-gpu     |
| HRM (sapientinc) | Hierarchical reasoning | 2 (GPU)  | ml-gpu     |
| Consensus        | Weighted ensemble      | 1 (Rust) | api        |

### 5.2 New Models (to add)

| Model                  | What it does                           | When   |
| ---------------------- | -------------------------------------- | ------ |
| Cross-asset DCC-GARCH  | Dynamic correlation BTC/SPY/Gold       | Wave 2 |
| On-chain analytics     | Whale wallets, exchange flows          | Wave 2 |
| Reinforcement Learning | Learns from own trades, adapts         | Wave 3 |
| Social sentiment       | X/Reddit/Google Trends signals         | Wave 3 |
| Regime-aware switcher  | Auto-switch strategy per market regime | Wave 2 |

### 5.3 Strategy Engine

User configures per strategy:

- **Models**: which ML models to use (checkboxes)
- **Symbols**: which assets to trade (multi-select)
- **Mode**: Signal / Semi-Auto / Full-Auto
- **Risk limits**: max position size %, max daily loss %, max drawdown %
- **Execution**: TWAP / VWAP / Market order
- **Schedule**: always-on / market hours only / custom

Strategy config stored in `user_strategies` table, published to NATS on change.

## 6. Trading Modes

### Signal Mode

- AI publishes signals to `ios.user.signal.{user_id}`
- Frontend shows buy/sell/hold with confidence
- Push notification via WebSocket + Telegram
- User executes manually on their broker
- No API keys required

### Semi-Auto Mode

- AI generates trade proposal with sizing + stop-loss
- User sees proposal in dashboard, clicks Confirm or Reject
- 5-minute timeout (configurable) — unconfirmed = rejected
- On confirm: Broker Gateway executes via user's API keys
- Full audit trail in user_trades

### Full-Auto Mode (Pro/Enterprise only)

- AI executes immediately when consensus confidence > threshold
- User sets risk limits: max position %, max loss/day, max drawdown
- Kill switch: one click stops all auto-trading
- Hourly P&L email/Telegram summary
- All trades logged with AI decision explanation

## 7. Security

- Broker API keys: AES-256-GCM encryption at rest, decrypted only in memory during execution
- Key permissions: reject keys with withdraw permission
- JWT auth with refresh token rotation (existing)
- TOTP 2FA (existing)
- Rate limiting per user per endpoint (existing)
- Kill switch per user + global (existing)
- EU AI Act logging for all AI decisions (existing)
- GDPR: right to deletion, data export (existing)

## 8. Reuse Assessment

| Category        | % Reuse | Details                                         |
| --------------- | ------- | ----------------------------------------------- |
| ML Models       | 100%    | All 8 models operational                        |
| NATS Pipeline   | 100%    | 5 streams, 6+ workers                           |
| Database        | 90%     | Add tenant_id + new tables                      |
| Auth            | 90%     | Add multi-tenant middleware                     |
| Frontend        | 80%     | Add strategy config + broker connect            |
| Broker          | 40%     | Paper works, Binance scaffolded, IBKR/OANDA new |
| Billing         | 0%      | Stripe new                                      |
| Strategy Engine | 0%      | New                                             |

**Overall: ~70% reuse, ~20% upgrade, ~10% new code**

## 9. Implementation Roadmap

### Wave 1: Production Foundation (2 weeks)

1. NATS Sprint N5 — fallback + metrics (1 day)
2. Train HRM on real data (1 day)
3. Multi-tenant auth + RLS (2 days)
4. Broker Gateway — Binance live + encrypted keys (2 days)
5. Strategy Engine — user config → NATS pipeline (2 days)
6. Trading modes — Signal / Semi-Auto / Full-Auto (2 days)
7. Stripe billing — tier gating (1 day)

### Wave 2: Multi-Broker + Intelligence (1 month)

8. IBKR connector (3 days)
9. OANDA connector (2 days)
10. Cross-asset correlation (2 days)
11. On-chain analytics (3 days)
12. AI Chat RAG (2 days)
13. Telegram/Discord bot (2 days)
14. Backtesting UI (3 days)
15. Smart Order Router (2 days)

### Wave 3: Marketplace + Scale (2-3 months)

16. Strategy Builder UI (5 days)
17. Copy trading marketplace (5 days)
18. Performance leaderboard (2 days)
19. Reinforcement Learning agent (5 days)
20. Mobile PWA (3 days)
21. Alternative data integration (3 days)

### Wave 4: Custody + Compliance (3-6 months)

22. Fireblocks custody (5 days)
23. KYC/AML — Sumsub (3 days)
24. Fiat on-ramp — Stripe Connect (3 days)
25. MiFID II / MiCA compliance (5 days)

## 10. Success Metrics

| Metric                    | Wave 1 Target    | Wave 2 Target |
| ------------------------- | ---------------- | ------------- |
| Users                     | 10 beta          | 100+          |
| Live trading users        | 3                | 30+           |
| Prediction accuracy       | >52% directional | >55%          |
| E2E latency (price→trade) | <5s              | <2s           |
| Uptime                    | 99%              | 99.9%         |
| MRR                       | $0 (beta)        | $5K+          |

## 11. Tech Stack Summary

| Layer      | Technology                                          |
| ---------- | --------------------------------------------------- |
| Backend    | Rust (Axum), async, multi-tenant                    |
| ML CPU     | Python (FastAPI), CatBoost, GARCH, FinBERT, skfolio |
| ML GPU     | Python, sapientinc/HRM, Chronos-2, RTX 3090         |
| Event Bus  | NATS JetStream                                      |
| Database   | PostgreSQL + TimescaleDB + pgvector                 |
| Cache      | Redis                                               |
| Frontend   | Next.js 16, React, Tailwind, Framer Motion          |
| Billing    | Stripe                                              |
| Deployment | Docker Compose on TrueNAS, nginx + Let's Encrypt    |
| Monitoring | Prometheus + Grafana                                |
| Storage    | ZFS (MainPool) + NVMe (models/HF cache)             |
| GPU        | NVIDIA RTX 3090 24GB                                |
