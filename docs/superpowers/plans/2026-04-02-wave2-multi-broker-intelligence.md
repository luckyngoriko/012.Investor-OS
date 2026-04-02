# Wave 2: Multi-Broker + Intelligence Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task.

**Goal:** Expand Investor OS to trade stocks/forex via IBKR+OANDA, add cross-asset intelligence, AI chat, Telegram signals, backtesting UI, and smart order routing.

**Architecture:** New broker connectors implement existing Broker trait. Intelligence models added as NATS workers. Telegram bot subscribes to ios.user.signal subjects. RAG chat uses existing pgvector embeddings.

**Tech Stack:** Rust (Axum), Python (FastAPI), teloxide (Telegram), pgvector (RAG), IBKR TWS API, OANDA REST API

---

## Tasks

### Task 8: IBKR Connector (stocks + options)

Create src/broker/ibkr.rs implementing Broker trait via TWS API. Account balance, positions, order placement. Paper trading mode by default.

### Task 9: OANDA Connector (forex)

Create src/broker/oanda.rs implementing Broker trait via OANDA REST API v20. Currency pairs, account info, market orders.

### Task 10: Cross-Asset Correlation (DCC-GARCH)

Create services/ml-sidecar/app/models/correlation_model.py — DCC-GARCH dynamic correlation between BTC/ETH/SPY/Gold/DXY. NATS worker publishing to ios.predict.correlation.

### Task 11: On-Chain Analytics

Create services/ml-sidecar/app/models/onchain_model.py — Whale wallet tracking via free APIs (Blockchain.com, Etherscan), exchange inflow/outflow, funding rates from Binance.

### Task 12: AI Chat (RAG)

Create src/chat/ module — user asks "Why did AI buy BTC?", system retrieves recent predictions + decision logs from pgvector, generates explanation. REST endpoint POST /api/chat.

### Task 13: Telegram Bot

Create services/telegram-bot/ — Python bot using python-telegram-bot. Subscribes to NATS ios.user.signal.{user_id}. Sends signals, allows /confirm and /reject for semi-auto trades.

### Task 14: Backtesting UI

Create frontend page /backtest — user selects strategy + date range, backend runs historical simulation using stored prices, displays P&L curve + Sharpe + drawdown.

### Task 15: Smart Order Router

Create src/broker/router.rs — compares prices across connected brokers (Binance/IBKR/OANDA), routes order to best execution venue. Logs routing decision.

## Verification

1. IBKR paper trade executes successfully
2. OANDA demo trade executes successfully
3. Correlation matrix updates on new price data
4. On-chain whale alert triggers on large transfer
5. AI Chat answers "why did you buy BTC?" with real prediction context
6. Telegram sends signal notification within 5s of consensus
7. Backtest page shows P&L chart for BTCUSDT
8. Smart router picks cheapest venue for cross-listed asset
