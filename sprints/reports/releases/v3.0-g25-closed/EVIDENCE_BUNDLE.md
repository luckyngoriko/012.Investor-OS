# Evidence Bundle: v3.0-g25-closed

- Generated: 2026-03-01
- Release tag candidate: `v3.0-g25-closed`
- Gate: `G25`

## Evidence Matrix

1. Migration portability and seed drift guardrails:
   - Command: `cargo test --test sprint78_migration_guardrails_test -- --nocapture`
   - Result: pass (`5/5`)

2. Migration reapply against clean database:
   - Command:
     `DATABASE_URL=postgres://postgres:postgres@127.0.0.1:55432/postgres cargo test --test sprint78_migration_guardrails_test -- --nocapture`
   - Result: pass (`5/5`)

3. Runtime backend contract smoke (strict mode):
   - Command:
     `BACKEND_BASE_URL=http://127.0.0.1:8080 REQUIRE_BACKEND=1 REQUIRE_FRONTEND=0 ./scripts/runtime_contract_smoke.sh`
   - Result: pass

4. Critical E2E runtime/user-flow coverage:
   - Command:
     `cd frontend/investor-dashboard && BACKEND_BASE_URL=http://127.0.0.1:8080 npx playwright test tests/e2e/auth/login.spec.ts tests/e2e/dashboard/dashboard.spec.ts tests/e2e/runtime/runtime-contract.spec.ts --project=chromium`
   - Result: pass (`8/8`)

5. PM governance consistency:
   - Command: `./scripts/verify_pm_sync.sh`
   - Result: pass (`16 done`, `0 in_progress`, `100%`)

6. PM boundary validation:
   - Command: `./scripts/verify_pm_boundaries.sh`
   - Result: pass (`No PM boundary candidates to validate.`)

## Artifacts

1. Sprint report:
   - `sprints/reports/SPRINT-078.md`
2. Program snapshot:
   - `sprints/reports/PROGRESS_SNAPSHOT.md`
3. Runtime guardrail E2E:
   - `frontend/investor-dashboard/tests/e2e/runtime/runtime-contract.spec.ts`
4. Runtime smoke script:
   - `scripts/runtime_contract_smoke.sh`
5. CI integration:
   - `.github/workflows/pr-checks.yml`

## Sign-Off

1. Engineering: approved.
2. QA: approved.
3. SRE/Operations: approved.
4. Security/Compliance: approved.

