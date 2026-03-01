# Sprint 078 Report: Post-Certification Reliability Guardrails & Drift Prevention

## Sprint Result

- Status: in_progress
- Gate: pending (G25)
- Planned scope completion: 38%

## Delivered

1. Sprint 78 activated and synchronized across PM state artifacts.
2. Scope and work packages defined for migration portability and runtime guardrails.
3. Added migration guardrail test suite: `tests/sprint78_migration_guardrails_test.rs`.
4. Implemented schema drift assertions:
   - disallow `requests_per_month` in seed migration
   - enforce `ON CONFLICT DO NOTHING` across seed inserts for `data_sources`, `data_source_endpoints`, `data_source_pricing`
5. Implemented portability assertions for `001_postgres_optimization.sql`:
   - no `CREATE INDEX CONCURRENTLY`
   - required portability guard markers remain present
6. Implemented dynamic integration guardrail:
   - creates fresh temp DB from `DATABASE_URL`
   - applies full migration chain twice (reapply/idempotency check)
   - verifies key migration versions marked successful in `_sqlx_migrations`
7. Added runtime contract E2E assertion:
   - `frontend/investor-dashboard/tests/e2e/runtime/runtime-contract.spec.ts`
   - validates `/api/health`, `/api/runtime/config`, `/api/hrm/status`, and `/metrics` contract shape/availability
8. Added runtime smoke probe script:
   - `scripts/runtime_contract_smoke.sh`
   - validates backend runtime endpoints and optional frontend `/login` + `/monitoring`
   - supports strict mode via `REQUIRE_BACKEND=1` / `REQUIRE_FRONTEND=1`
9. Wired runtime guardrails into CI path:
   - added runtime E2E spec to Playwright smoke suite in `.github/workflows/pr-checks.yml`
   - added best-effort curl runtime smoke step to PR checks

## Not Delivered / Deferred

1. G25 close-out evidence pending final Sprint 78.4 consolidation.

## Verification Summary

- `cargo test --test sprint78_migration_guardrails_test -- --nocapture`: pass (5/5).
- `DATABASE_URL=postgres://postgres:postgres@127.0.0.1:55432/postgres cargo test --test sprint78_migration_guardrails_test -- --nocapture`: pass (5/5), including dynamic fresh-DB migration run.
- `cd frontend/investor-dashboard && npx playwright test tests/e2e/runtime/runtime-contract.spec.ts --project=chromium`: pass (spec skipped when backend unavailable).
- `REQUIRE_BACKEND=1 BACKEND_BASE_URL=http://127.0.0.1:8080 ./scripts/runtime_contract_smoke.sh`: pass for backend runtime contract checks.

## Program Progress

- Total sprints in program: 16
- Completed sprints: 15
- Overall completion: 93%
- Remaining to 100%: 7%

## Open Risks

1. Cross-environment migration compatibility can regress without continuous guardrails.
2. Seed/schema drift may reoccur if table definitions change without synchronized seed updates.
3. Frontend runtime assumptions may diverge from backend contract under future changes.

## Next Sprint Decision

- Next sprint: 78
- Activation status: in_progress
- Preconditions met: yes
