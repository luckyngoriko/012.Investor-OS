# Sprint 078: Post-Certification Reliability Guardrails & Drift Prevention

## Metadata

- Sprint ID: 78
- Status: done
- Gate: G25
- Owner: Platform + Backend + QA
- Dependencies: 77

## Objective

Extend reliability controls after program closure by converting migration/runtime safeguards into repeatable verification gates, eliminating schema drift blind spots, and hardening cross-environment startup behavior.

## Scope In

1. Add deterministic checks for migration portability across environments with different extension availability.
2. Add guardrails for seed-data schema alignment and idempotent conflict handling.
3. Codify runtime contract assertions for health, ready, metrics, and core API surfaces.
4. Synchronize PM artifacts and evidence for post-certification reliability governance.

## Scope Out

1. New product features unrelated to runtime reliability.
2. Broad refactors without direct impact on migration/runtime guardrails.

## Work Packages

1. WP-78A: Migration portability verification hardening.
2. WP-78B: Seed schema drift prevention and regression assertions.
3. WP-78C: Runtime contract smoke automation across backend/frontend surfaces.
4. WP-78D: Governance evidence packaging and G25 close-out preparation.

## Acceptance Criteria

1. Migration path executes successfully on a clean database under restricted extension sets.
2. Seed migration is idempotent and schema-aligned with no runtime column mismatch failures.
3. Login, dashboard, and monitoring critical paths complete with successful API contract checks.
4. PM state artifacts remain synchronized with verified completion metrics.

## Verification Commands

```bash
./scripts/verify_pm_sync.sh
./scripts/verify_pm_boundaries.sh .current_sprint sprints/active.toml sprints/SPRINT_REGISTRY.yaml sprints/BOARD.md sprints/specs/SPRINT-078.md sprints/reports/SPRINT-078.md
./scripts/status_update.sh
cargo run --bin investor-os
cd frontend/investor-dashboard && pnpm dev --port 3000
```

## Gate Condition

G25 passes when migration portability, seed drift prevention, runtime contract smoke checks, and PM synchronization evidence are all complete without unresolved blockers.

## Evidence Required

1. Successful migration execution evidence on clean environment.
2. Runtime/API smoke evidence for critical user flows.
3. PM synchronization verification outputs.
4. Sprint closure report with residual-risk disposition.

## Risks

1. Legacy schema assumptions not represented in repository migrations may still reappear in edge environments.
2. Environment-dependent behavior (extensions, DB features) may mask latent migration regressions.
3. Runtime contract drift between frontend assumptions and backend routes may regress without continuous checks.
