# Wave 1a: Production Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Transform Investor OS from prototype to production — NATS fallback, trained HRM, multi-tenant auth, and Binance live trading with encrypted API keys.

**Architecture:** Extend existing Rust API with tenant isolation (tenant_id on tables + RLS policies), add encrypted broker key vault (AES-256-GCM), wire NATS fallback for resilience. HRM trained on real 1440 BTCUSDT candles.

**Tech Stack:** Rust (Axum), async-nats, sqlx, aes-gcm, Python (PyTorch), PostgreSQL + TimescaleDB

---

## Tasks

### Task 1: NATS Sprint N5 — Fallback Monitor

Create src/nats/fallback.rs — monitors NATS connection, switches to HTTP polling after 60s disconnect, resumes when NATS reconnects.

### Task 2: Train HRM on Real Data

Create scripts/ml/train_hrm_real.py — loads 1440 BTCUSDT candles from DB, tokenizes, trains sapientinc/HRM on RTX 3090, saves best checkpoint.

### Task 3: Multi-Tenant Auth + Tables

Create migration with user_strategies, user_trades tables + tenant_id columns + RLS policies on existing tables.

### Task 4: Broker Gateway with Encrypted Keys

Create broker_connections table + AES-256-GCM vault (src/broker/vault.rs) + unified gateway (src/broker/gateway.rs) for storing and decrypting per-user broker API keys.

## Verification

1. cargo build succeeds
2. NATS fallback activates after 60s disconnect
3. HRM trained model achieves >35% action accuracy
4. All new tables exist in postgres
5. Broker keys encrypt/decrypt correctly
