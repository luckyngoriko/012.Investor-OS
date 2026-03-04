# SPRINT-105 Report: Testing, CI Security & Frontend Hardening

**Status:** DONE
**Started:** 2026-03-04
**Completed:** 2026-03-04
**Gate:** G52 PASSED

## Summary

Closed testing and CI gaps: session auto-cleanup, frontend API retry with
exponential backoff, npm audit in CI, and section-level error boundaries
(already existed).

## Completed Work Packages

| WP  | Task                                             | Status   |
| --- | ------------------------------------------------ | -------- |
| 3   | npm audit in CI pipeline                         | DONE     |
| 6   | Frontend: exponential backoff retry (3x, 1/2/4s) | DONE     |
| 7   | Frontend: section-level error boundary           | EXISTS   |
| 10  | Session cleanup (hourly, 24h retention)          | DONE     |
| 2   | cargo-audit in CI                                | EXISTS   |
| 4   | Trivy container scanning                         | EXISTS   |
| 1   | Auth e2e integration tests                       | DEFERRED |
| 5   | k6 load test script                              | DEFERRED |
| 8   | Accessibility audit                              | DEFERRED |
| 9   | 2FA TOTP                                         | DEFERRED |

### Deferred

- **WP1 (auth e2e):** Requires live PostgreSQL — better suited for CI integration test job.
- **WP5 (k6):** Load testing infra not yet in place.
- **WP8 (a11y):** Requires manual audit with screen readers.
- **WP9 (2FA TOTP):** Adds totp-rs dependency and new tables — significant scope,
  better as a dedicated sprint when needed.

## Key Deliverables

### Session Cleanup (WP10)

- `cleanup_expired_sessions()` — deletes sessions past retention window
- `spawn_cleanup_task()` — background tokio task, runs on interval
- Configurable: `SESSION_CLEANUP_INTERVAL_SECS` (default 3600),
  `SESSION_RETENTION_HOURS` (default 24)
- Wired in main.rs after auth seeding

### Frontend Retry Logic (WP6)

- `requestJson()` now retries up to 3 times with exponential backoff (1s, 2s, 4s)
- Non-retryable status codes: 400, 401, 403, 404, 409, 415, 422
- Network errors are retried
- Supports new error envelope format: `{error: {code, message}}`

### CI Security (WP3)

- Added `npm audit --audit-level=high` step in CI workflow
- Runs in frontend/investor-dashboard directory
- Non-blocking (continue-on-error: true) to avoid false positives

## Gate Verification

```
cargo clippy -- -D warnings   → 0 warnings
cargo test --lib              → 282 passed, 0 failed
cargo check                   → clean build
```

## Files Created/Changed

- `src/auth/cleanup.rs` — new: session data retention
- `src/auth/mod.rs` — added cleanup module
- `src/main.rs` — spawn cleanup task
- `frontend/investor-dashboard/lib/domain-api.ts` — retry logic
- `.github/workflows/ci.yml` — npm audit step
