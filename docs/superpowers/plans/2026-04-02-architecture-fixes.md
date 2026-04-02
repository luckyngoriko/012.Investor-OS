# Architecture Fixes — Tech Debt Cleanup Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task.

**Goal:** Fix 8 architectural problems identified during review: split main.rs, spawn NATS workers, extract real user_id from JWT, env-based DB config, integration tests, sidebar links, deploy, remove duplication.

**Architecture:** Modular route handlers, proper NATS lifecycle, JWT-based user extraction, env-driven config.

**Tech Stack:** Rust (Axum), async-nats, sqlx, Next.js

---

## Tasks

### Fix 1: Split main.rs into route modules

Move 30+ handlers from main.rs into src/api/routes/{auth,strategies,marketplace,custody,kyc,fiat,chat,backtest,leaderboard,predictions,compliance}.rs. main.rs becomes ~100 lines: AppState + create_router() + main().

### Fix 2: Spawn all NATS workers on startup

Add tokio::spawn() for trade_executor, signal_router, portfolio_tracker, event_logger in main.rs startup. Gate behind `if let Some(ref nats)` check.

### Fix 3: Extract real user_id from JWT

Replace hardcoded admin UUID with actual user_id from auth middleware Extension. Auth middleware already puts AuthUser in request extensions — extract it in handlers.

### Fix 4: Python DB config from env

Replace all hardcoded DB URLs in Python workers with `os.environ.get("DATABASE_URL", "postgresql://...")`. Single source of truth from docker-compose env.

### Fix 5: Integration test for full pipeline

Create tests/nats_pipeline_integration_test.rs — publish mock price to ios.prices.BTCUSDT, verify features arrive, verify prediction arrives, verify consensus arrives.

### Fix 6: Add all pages to sidebar

Update frontend/investor-dashboard/components/sidebar.tsx — add links for: Strategy Builder, Marketplace, Leaderboard, Backtest, AI Chat.

### Fix 7: Deploy all waves to TrueNAS

Rebuild api + ml-sidecar + frontend on TrueNAS. Verify all containers healthy.

### Fix 8: Remove prediction_loop.py duplication

Mark prediction_loop.py as deprecated. NATS workers are the canonical pipeline. Keep script for manual one-shot use only.

## Verification

1. main.rs < 150 lines
2. All NATS workers start on boot (check logs)
3. Strategy CRUD uses real user_id from JWT
4. Python workers read DB_URL from env
5. Integration test passes end-to-end
6. All pages accessible from sidebar
7. All containers healthy on TrueNAS
8. No duplicate prediction paths
