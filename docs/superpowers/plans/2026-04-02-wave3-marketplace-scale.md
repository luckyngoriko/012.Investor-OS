# Wave 3: Marketplace + Scale Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task.

**Goal:** Add strategy marketplace with copy-trading, performance leaderboard, RL agent, mobile PWA, and alternative data — transforming Investor OS from tool to platform.

**Architecture:** New marketplace tables + frontend pages. RL agent as NATS worker. PWA via Next.js manifest + service worker. Alternative data as Python NATS workers.

**Tech Stack:** Rust (Axum), Python (stable-baselines3), Next.js PWA, PostgreSQL

---

## Tasks

### Task 16: Strategy Builder UI

Create frontend /strategy-builder page — drag-and-drop style UI where user selects models (checkboxes), symbols (multi-select), mode (radio), risk limits (sliders). Saves to user_strategies via existing API. Visual preview of pipeline flow.

### Task 17: Copy Trading Marketplace

Create strategy_listings table + marketplace API. Creators publish strategies with performance stats. Subscribers copy-trade by cloning strategy config. Commission tracking (20% to platform). Frontend /marketplace page with strategy cards, filters, subscribe button.

### Task 18: Performance Leaderboard

Create /leaderboard frontend page. Backend aggregates strategy performance from user_trades: total return, Sharpe, max drawdown, win rate. Ranked list with anonymized usernames. Filter by timeframe (7d/30d/all).

### Task 19: Reinforcement Learning Agent

Create services/ml-gpu/app/models/rl_agent.py — PPO agent using stable-baselines3. State: technical features + model predictions. Action: position size (-1 to +1). Reward: realized P&L - drawdown penalty. NATS worker subscribing to ios.consensus, publishing to ios.predict.rl. Trains online from user_trades.

### Task 20: Mobile PWA

Add Next.js PWA support: manifest.json, service worker, offline page, install prompt. Responsive adjustments for mobile viewports. Push notifications via Web Push API for trading signals.

### Task 21: Alternative Data Integration

Create services/ml-sidecar/app/models/alt_data_model.py — fetches Google Trends (pytrends), Reddit sentiment (PRAW or web scrape), GitHub commit activity for crypto projects. NATS worker publishing to ios.altdata.{symbol}. Feeds into consensus as additional signal.

## Verification

1. Strategy Builder saves valid config to user_strategies
2. Marketplace shows published strategies with subscribe button
3. Leaderboard ranks strategies by Sharpe ratio
4. RL agent produces position sizing signal after 100 training steps
5. PWA installs on mobile, shows offline page when disconnected
6. Alternative data scores update every hour
