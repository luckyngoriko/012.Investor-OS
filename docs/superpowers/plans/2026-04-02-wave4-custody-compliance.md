# Wave 4: Custody + Compliance Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task.

**Goal:** Add institutional-grade custody (Fireblocks MPC), KYC/AML identity verification, fiat on-ramp, and EU regulatory compliance — enabling Investor OS to hold and manage user funds directly.

**Architecture:** Fireblocks SDK for MPC custody, Sumsub webhook-based KYC, Stripe Connect for fiat, compliance module extending existing EU AI Act logging.

**Tech Stack:** Rust (Axum), Fireblocks SDK (REST), Sumsub API, Stripe Connect, PostgreSQL

---

## Tasks

### Task 22: Fireblocks MPC Custody Integration

Create src/custody/mod.rs + src/custody/fireblocks.rs — Fireblocks API client for multi-party computation wallet management. Create/manage vaults per user, deposit addresses, withdrawal signing, transaction status tracking. Migration for custody_wallets + custody_transactions tables. REST endpoints: POST /api/custody/deposit-address, POST /api/custody/withdraw, GET /api/custody/balance, GET /api/custody/transactions.

### Task 23: KYC/AML — Sumsub Integration

Create src/compliance/kyc.rs — Sumsub identity verification. Generate applicant tokens, handle webhook callbacks for verification status updates. Migration for kyc_verifications table (user_id, status, level, sumsub_applicant_id, verified_at). Gate live trading behind KYC completion. REST endpoints: POST /api/kyc/start, GET /api/kyc/status, POST /api/kyc/webhook (Sumsub callback).

### Task 24: Fiat On-Ramp — Stripe Connect

Create src/billing/fiat_onramp.rs — Stripe Connect for fiat deposits/withdrawals. Connected accounts per user, payment intents for deposits, payouts for withdrawals. Migration for fiat_transactions table. REST endpoints: POST /api/fiat/deposit, POST /api/fiat/withdraw, GET /api/fiat/history, POST /api/fiat/webhook (Stripe callback).

### Task 25: MiFID II / MiCA Regulatory Compliance

Create src/compliance/mifid.rs + src/compliance/mica.rs — Regulatory reporting and compliance checks. Transaction reporting (MiFID II Article 26), crypto-asset classification (MiCA), best execution policy documentation, client categorization (retail/professional). Extend existing EU AI Act compliance module. Migration for compliance_reports table. REST endpoints: GET /api/compliance/status, GET /api/compliance/reports, POST /api/compliance/classify-client.

## Verification

1. Fireblocks creates vault and generates BTC deposit address (sandbox mode)
2. Sumsub returns applicant token and processes test verification
3. Stripe Connect creates payment intent for $100 deposit
4. Compliance status endpoint returns MiFID + MiCA + AI Act scores
5. KYC-gated endpoint rejects unverified user
6. All custody transactions logged with full audit trail
