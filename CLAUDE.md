# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Investor OS v3.0 is an autonomous AI trading system built in Rust (backend) + Next.js 16 (frontend). It combines financial RAG, ML inference (HRM model), broker execution, and self-learning (Phoenix engine) into a single platform.

## Build & Development Commands

### Rust Backend

```bash
cargo build                          # Debug build
cargo build --release                # Release build
cargo clippy -- -D warnings          # Lint (must pass with zero warnings)
cargo fmt -- --check                 # Format check
cargo test                           # Run all tests
cargo test --lib -- --test-threads=4 # Unit tests only
cargo test --test '*' -- --test-threads=2  # Integration tests only
cargo test <test_name>               # Run a single test by name
cargo test --test sprint7_analytics_test   # Run a specific test file
cargo bench --bench hrm_inference    # HRM inference benchmarks (Criterion)
cargo doc --no-deps                  # Generate docs
```

### Frontend (Next.js 16 + React 19)

```bash
cd frontend/investor-dashboard
npm run dev              # Dev server
npm run build            # Production build
npm run lint             # ESLint
npm run test             # Vitest unit tests
npm run test:watch       # Vitest watch mode
npm run test:e2e         # Playwright E2E tests
npm run test:all         # Unit + E2E
```

### Infrastructure

```bash
docker compose up -d     # Start PostgreSQL (TimescaleDB) + Redis
```

### Sprint Gate Protocol (all must pass before sprint advances)

```bash
cargo test -- --test-threads=1       # Gate 1: All tests green
cargo clippy -- -D warnings          # Gate 2: Zero warnings
cargo build --release                # Gate 3: Clean build
cargo llvm-cov --html                # Gate 4: Coverage ≥ 80%
cargo doc --no-deps                  # Gate 5: No doc warnings
```

### Binaries

```bash
cargo run                            # Main API server
cargo run --bin gpu-check            # GPU detection utility
cargo run --bin hrm-node             # Distributed HRM inference node
cargo run --bin hrm-lb               # HRM load balancer
```

## Architecture

### System Layers

```
┌─────────────────────────────────────────────────────────┐
│  API Layer (Axum)                                       │
│  /api/rag/* · /api/broker/* · /api/analytics/* · /admin │
├─────────────────────────────────────────────────────────┤
│  AI Decision Layer                                      │
│  HRM Model · Phoenix Engine · LangChain · LangGraph     │
├─────────────────────────────────────────────────────────┤
│  Data & Execution Layer                                 │
│  RAG (SEC/Earnings) · Broker · Analytics · Streaming    │
├─────────────────────────────────────────────────────────┤
│  Infrastructure Layer                                   │
│  Distributed HRM · Risk · Temporal Workflows · Config   │
└─────────────────────────────────────────────────────────┘
```

### Key Modules (src/)

| Module         | Purpose                                                                                                        |
| -------------- | -------------------------------------------------------------------------------------------------------------- |
| `api/`         | Axum HTTP router with handlers for RAG, broker, analytics, admin                                               |
| `hrm/`         | Hierarchical Reasoning Model — native Rust ML (Burn framework), 6D signal input → conviction/action/confidence |
| `phoenix/`     | Self-learning trading engine: paper sim → RAG memory → LLM strategist → graduation assessment                  |
| `broker/`      | Multi-broker execution: Interactive Brokers, Binance, OANDA, paper trading                                     |
| `rag/`         | Financial document RAG: SEC filings (10-K/10-Q/8-K), earnings calls, pgvector search                           |
| `ml/apis/`     | Multi-provider LLM orchestration: Gemini, OpenAI, Claude, HuggingFace with cost tracking and fallback          |
| `langchain/`   | Rust port of LangChain: agents, chains (sequential/parallel), memory, prompt templates, tools                  |
| `langgraph/`   | State machine graphs for trading decisions: nodes, conditional edges, shared state                             |
| `temporal/`    | Durable workflow engine: activities with retry, saga compensation, persistent workers                          |
| `streaming/`   | Real-time market data: WebSocket connections, order book updates, trade analysis                               |
| `distributed/` | Horizontal HRM scaling: gRPC nodes, load balancing (round-robin/least-loaded), service discovery               |
| `risk/`        | Portfolio risk management: VaR, position sizing, stop-loss, drawdown limits                                    |
| `analytics/`   | Backtesting, performance attribution, ML predictions, anomaly detection                                        |
| `treasury/`    | Multi-currency wallet, Fireblocks custody (feature-gated), yield optimization                                  |
| `compliance/`  | EU AI Act & GDPR (feature-gated: `eu_compliance`)                                                              |
| `anti_fake/`   | Runtime anti-spoofing: nonce replay protection, request signing, velocity checks                               |
| `signals/`     | Trading signal generation                                                                                      |
| `config/`      | Environment-driven config: trading modes (Manual/SemiAuto/FullyAuto), risk limits, broker connections          |

### Frontend Structure (frontend/investor-dashboard/)

Next.js 16 App Router with: dashboard, portfolio optimization, positions, risk, HRM real-time, AI training, monitoring, tax, security, strategy, settings, journal, proposals, deployment, admin, login, chart pages. Uses Radix UI + Tailwind CSS 4 + Recharts + Framer Motion. Tests via Vitest + Playwright.

### Data Flow: Trading Decision

```
Streaming (market ticks) → Analytics (signals, regime detection)
  → HRM (6D vector → conviction + action + confidence)
  → Phoenix (if training: RAG memory + LLM strategist)
  → Risk (pre-flight checks, position sizing)
  → Broker (order placement + execution tracking)
  → RAG (decision journal storage)
  → Temporal (durable workflow persistence)
```

### HRM Input Signals (6-dimensional)

| Index | Signal            | Range                           |
| ----- | ----------------- | ------------------------------- |
| 0     | PEGY Score        | 0–1                             |
| 1     | Insider Sentiment | 0–1                             |
| 2     | Social Sentiment  | -1 to 1                         |
| 3     | VIX Level         | 0–100                           |
| 4     | Market Regime     | 0–3 (Bull/Bear/Sideways/Crisis) |
| 5     | Time Factor       | 0–1                             |

## Cargo Features

```toml
default = ["cpu"]              # CPU backend (burn-ndarray)
gpu-auto = ["cuda", "rocm", "intel"]  # Auto-detect GPU
cuda = []                      # NVIDIA CUDA
fireblocks = ["dep:jsonwebtoken"]     # Treasury custody
eu_compliance = []             # EU AI Act & GDPR
```

## Critical Rules

- **Financial math**: Always use `rust_decimal::Decimal`, never `f64`
- **Signal scores**: Use `Score` newtype (`f32` clamped to [0.0, 1.0])
- **Tests**: Every test must verify computed output — never `assert!(true)`, never `assert!(result.is_ok())` on infallible functions
- **Kill switch**: Never remove or weaken. Hard limits: max drawdown -10% → freeze, data staleness >48h → no trades, error rate >5% → pause
- **Decision log**: Non-trivial decisions → `DECISION_LOG.md` with context/options/chosen/rationale
- **Borrowed code**: Reused components → `BORROWED.md` with source/target/adaptations

## Vendor Patches

`Cargo.toml` patches `sqlx`, `sqlx-macros-core`, and `burn-core` from `vendor/patches/` to strip unused backends (MySQL, non-CPU Burn GPU paths). The `neurocod-rag` crate is vendored at `vendor/neurocod-rag/`.

## Sprint System

Sprints tracked in `sprints/active.toml` (machine-readable state), `sprints/specs/` (specifications), and `sprints/reports/` (completion reports). Current phase: enterprise-readiness-recovery (sprints 63–95). Agent workflows in `.agent/workflows/` define processes for sprint gates, golden path testing, and documentation.

## CI/CD

GitHub Actions (`.github/workflows/ci.yml`): fmt → clippy → unit tests → integration tests → security audit (cargo-audit, Trivy, Hadolint) → Docker build → deploy (dev/staging/production with canary). `RUSTFLAGS="-D warnings"` enforced in CI.
