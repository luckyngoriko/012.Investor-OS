# Sprint 099: Observability, Test Hygiene & Frontend Closure

## Metadata

- Sprint ID: 99
- Status: queued
- Gate: G46
- Owner: Backend + Frontend + QA
- Dependencies: 97, 98

## Objective

Close remaining production gaps: fake observability endpoints, weak test assertions, AntiFakeShield enforcement defaults, and frontend mock data removal.

## Scope In

1. Replace hardcoded `/metrics` endpoint data with real Prometheus metric export.
2. Replace hardcoded `/system-metrics` and `/health` simulated checks with real health probes.
3. Enable AntiFakeShield enforcement by default in production environment configuration.
4. Fix 35 weak test assertions across 8 test files (replace `assert!(true)` and `assert!(result.is_ok())` with computed output verification).
5. Remove `generateMockHistory()` from frontend HRM API and wire to real backend endpoint.
6. Implement early access request in frontend empty states component.

## Scope Out

1. New observability features (distributed tracing, custom dashboards).
2. New test coverage for untested modules.
3. Distributed HRM gRPC server implementation (tracked separately).

## Work Packages

1. WP-99A: Real metrics endpoint — `/metrics` returns actual Prometheus-formatted metrics from the `prometheus` crate registry. Remove hardcoded CPU/RAM/PnL/Sharpe values.
2. WP-99B: Real health checks — `/health` performs actual PostgreSQL `pg_isready` via connection pool and Redis `PING`. `/system-metrics` returns real latency measurements.
3. WP-99C: AntiFake defaults — set `enforce: true` and `enforce_real_data: true` when `ENVIRONMENT=production`. Add `/metrics` and `/system-metrics` to fake endpoint prefixes if they still return synthetic data.
4. WP-99D: Test assertion hardening — fix all 35 weak assertions:
   - `tests/sprint52_eu_compliance_test.rs` (12 assertions)
   - `tests/temporal_test.rs` (5 assertions)
   - `tests/langgraph_test.rs` (3 assertions)
   - `tests/sprint11_multi_asset_test.rs` (2 assertions)
   - `tests/sprint12_streaming_test.rs` (2 assertions)
   - `tests/api_integration_test.rs` (2 assertions)
   - `tests/coverage_graph_test.rs` (1 assertion)
   - `tests/coverage_chains_test.rs` (1 assertion)
5. WP-99E: Frontend HRM — remove `generateMockHistory()`, replace with `fetch('/api/hrm/history')` call. Handle loading/error states.
6. WP-99F: Frontend early access — replace `alert()` in empty states with POST to `/api/waitlist` endpoint (or disable button with "Coming soon" tooltip).

## Acceptance Criteria

1. `/metrics` returns real Prometheus metrics matching actual system state.
2. `/health` reflects actual database and Redis connectivity.
3. AntiFakeShield blocks requests to fake endpoints in production mode.
4. Zero `assert!(true)` or bare `assert!(result.is_ok())` in test suite.
5. Frontend HRM dashboard shows real backend data, no `generateMockHistory()` calls.
6. All gates pass: clippy, tests, build.

## Verification Commands

```bash
cargo clippy -- -D warnings
cargo test --lib -- --test-threads=4
cargo test --test '*' -- --test-threads=2
cd frontend/investor-dashboard && npm test -- --run
cd frontend/investor-dashboard && npm run build
```

## Files to Modify

- `src/main.rs` (WP-99A, WP-99B, WP-99C)
- `src/anti_fake.rs` (WP-99C)
- `tests/sprint52_eu_compliance_test.rs` (WP-99D)
- `tests/temporal_test.rs` (WP-99D)
- `tests/langgraph_test.rs` (WP-99D)
- `tests/sprint11_multi_asset_test.rs` (WP-99D)
- `tests/sprint12_streaming_test.rs` (WP-99D)
- `tests/api_integration_test.rs` (WP-99D)
- `tests/coverage_graph_test.rs` (WP-99D)
- `tests/coverage_chains_test.rs` (WP-99D)
- `frontend/src/api/hrm.ts` (WP-99E)
- `frontend/src/components/HRMDashboard.tsx` (WP-99E)
- `frontend/investor-dashboard/components/empty-states.tsx` (WP-99F)
