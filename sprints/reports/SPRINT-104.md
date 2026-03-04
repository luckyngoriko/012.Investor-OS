# SPRINT-104 Report: Observability & Resilience

**Status:** DONE
**Started:** 2026-03-04
**Completed:** 2026-03-04
**Gate:** G51 PASSED

## Summary

Added production observability infrastructure: request correlation IDs, configurable
DB pool with health checks, persistent audit event logging, and real dependency
status in the health endpoint.

## Completed Work Packages

| WP  | Task                                                     | Status            |
| --- | -------------------------------------------------------- | ----------------- |
| 2   | Request correlation ID middleware (X-Request-Id)         | DONE              |
| 4   | DB pool configurable via env (30 max, 5 min, 5s timeout) | DONE              |
| 6   | Persistent audit log (PostgreSQL `audit_events` table)   | DONE              |
| 9   | Health endpoint reflects real dependency status          | DONE              |
| 10  | Tests for audit event types                              | DONE              |
| 1,3 | OpenTelemetry/Jaeger                                     | DEFERRED          |
| 5   | Redis connection pool                                    | DONE (Sprint 102) |
| 7-8 | Circuit breakers for LLM/broker                          | EXISTS            |

### Deferred / Pre-existing

- **WP1,3 (OpenTelemetry):** Deferred — heavy dependency; existing `tracing` +
  `observability::MetricsCollector` already provides structured logging and Prometheus
  metrics. OpenTelemetry can be added when Jaeger/Tempo is deployed.
- **WP7-8 (Circuit breakers):** Already exist in `src/resilience/mod.rs`
  (standard request-level) and `src/ai_safety/circuit_breaker.rs` (market-level).

## Key Deliverables

### Request Correlation ID (WP2)

- Middleware reads `X-Request-Id` from request or generates UUID
- Stored in request extensions via `RequestId` struct
- Propagated to response headers

### DB Pool Configuration (WP4)

- `DB_POOL_MAX_CONNECTIONS` (default 30)
- `DB_POOL_MIN_CONNECTIONS` (default 5)
- 5-second acquire timeout
- Pool stats logged at startup

### Audit Log (WP6)

- `audit_events` table: event_type, user_id, client_ip, details (JSONB), created_at
- `AuditEvent` enum: LoginSuccess, LoginFailed, AccountLocked, Logout, TokenRefresh,
  PasswordChanged, UserCreated, UserUpdated, UserDisabled
- `log_audit_event()` — fire-and-forget, failures logged but never propagate
- `recent_events()` — admin query endpoint
- Indexed by user_id, created_at, event_type

### Real Health Checks (WP9)

- `/api/health` now does `SELECT 1` against PostgreSQL
- Reports actual DB status (pass/fail)
- Reports Redis status (pass/not_configured)
- Overall status: "healthy" or "degraded"
- Environment from `ENVIRONMENT` env var

## Gate Verification

```
cargo clippy -- -D warnings   → 0 warnings
cargo test --lib              → 280 passed, 0 failed
cargo check                   → clean build
```

## Files Created/Changed

- `migrations/20260304000003_audit_events.sql` — new migration
- `src/auth/audit.rs` — new: audit event logging + query
- `src/auth/mod.rs` — added audit module
- `src/main.rs` — correlation ID middleware, DB pool config, health checks, db_pool in AppState
