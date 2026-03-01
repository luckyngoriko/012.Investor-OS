# Sprint 082 Evidence: Security Backlog Register

Generated: 2026-03-01
Gate: `G29`
Baseline Source: PR Checks run `22546841010`, step `G5 - Security audit (blocking)`.

## Policy

1. Blocking RustSec vulnerabilities must have one of:
   - direct remediation in lock/dependency graph, or
   - explicit time-bounded risk acceptance with owner, rationale, and review date.
2. Unmaintained crate warnings are tracked as watchlist items and escalated only when reachable exploitability is demonstrated or maintained alternatives are available.
3. `.cargo/audit.toml` is the canonical allowlist for temporary risk acceptance; every entry must map to this register.

## Backlog Items

| Advisory | Crate | Severity | Dependency Path | Status | Decision | Owner | Due Date | Evidence |
|---|---|---|---|---|---|---|---|---|
| RUSTSEC-2024-0437 | protobuf 2.28.0 | vulnerability | `prometheus 0.13.4 -> investor-os` | open-risk-accepted | Temporary risk acceptance renewed after Sprint 84 retry. Online update is blocked by intermittent `index.crates.io` DNS failures and missing offline cache entries for new transitive packages. | Platform Lead | 2026-03-22 | CI logs `22546841010`, Sprint 84 retry outputs (`cargo update -p prometheus`, `cargo update -p prometheus --offline`), `.cargo/audit.toml` |
| RUSTSEC-2023-0071 | rsa 0.9.10 | medium (5.9) | `sqlx-mysql 0.8.6 -> sqlx-macros-core 0.8.6` | open-risk-accepted | Runtime scope is postgres-only; mysql execution path is not used by Investor OS runtime. Keep allowlisted until upstream sqlx toolchain removes vulnerable transitive path. | Backend Lead | 2026-03-15 | CI log run `22546841010`, `cargo tree -i rsa --target all --all-features` (no active path), `.cargo/audit.toml` |
| RUSTSEC-2025-0141 | bincode 1.3.3 / 2.0.1 | warning (unmaintained) | `roqoqo` and `burn-core` chains | watchlist | Track upstream migration plans; no immediate blocker while vulnerability class is not reported. | ML Lead | 2026-03-31 | CI warning output run `22546841010` |
| RUSTSEC-2024-0436 | paste 1.0.15 | warning (unmaintained) | transitive in math/gpu stack | watchlist | Monitor upstream replacement or maintained fork; keep as non-blocking warning. | Platform Lead | 2026-03-31 | CI warning output run `22546841010` |

## Controls Implemented in Sprint 82

1. Added governed audit allowlist at standard cargo-audit path: `.cargo/audit.toml`.
2. Reduced SQLx feature surface in `Cargo.toml` to explicit postgres-only configuration (`default-features = false`).
3. Replaced `sqlx::FromRow` proc-macro usage with manual `FromRow` implementations in `src/rag/search/mod.rs` to reduce macro-related feature coupling.

## Review Cadence

- Weekly review in sprint status cadence until all `open-risk-accepted` items are either remediated or renewed with explicit re-approval.
- Any newly discovered high/critical RustSec advisory bypasses allowlist and triggers immediate remediation sprint entry.
