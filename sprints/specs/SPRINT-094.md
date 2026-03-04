# Sprint 094: Real-Capital Go-Live & Live-Data Certification

## Metadata

- Sprint ID: 94
- Status: done
- Gate: G41
- Owner: Backend + Frontend + Platform + Security + QA
- Dependencies: 93

## Objective

Close the remaining production gap so the platform can progress from paper-first operations to controlled real-capital execution with live-data fidelity and enterprise rollout readiness.

## Scope In

1. Real-capital execution path readiness (broker + treasury + risk controls).
2. Frontend/runtime live-data closure for remaining mock-driven product surfaces.
3. Enterprise hardening for metrics, maintenance, backup/restore, and operational controls.
4. Controlled rollout gates: shadow -> canary -> limited capital -> full production recommendation.

## Scope Out

1. New product features unrelated to production-readiness closure.
2. Experimental model research beyond runtime correctness/performance needs.

## Work Packages

1. WP-94A: Frontend de-mock completion (Tax, Security, Portfolio Optimization) via backend contracts.
2. WP-94B: Real-capital execution enablement and reconciliation hardening.
3. WP-94C: Enterprise runtime hardening (metrics, backup, maintenance, distributed/runtime controls).
4. WP-94D: Release gates and go-live certification evidence package.

## Acceptance Criteria

1. No remaining runtime mock data source in targeted user-critical pages.
2. Execution path can operate in controlled real-capital mode with auditable risk controls.
3. Operational guardrails (observability, backup/restore, maintenance, incident hooks) are production-ready.
4. Final gate evidence exists for shadow/canary/limited-capital progression and rollback safety.

## Verification Commands

```bash
cargo check --locked
cargo test --lib --locked
cargo audit
cd frontend/investor-dashboard && npm test -- --run
cd frontend/investor-dashboard && npm run build
cd frontend/investor-dashboard && npx playwright test --project=chromium
```

## Gate Condition

G41 passes when mock/fallback critical gaps are closed, real-capital controls are validated, and rollout gates are green with rollback readiness.
