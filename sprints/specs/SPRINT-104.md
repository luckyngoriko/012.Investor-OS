# SPRINT-104: Observability & Resilience

**Phase:** world-class-platform
**Gate:** G51
**Depends on:** Sprint 102

## Objective

Full production observability: OpenTelemetry distributed tracing, structured JSON
logging with request correlation, configurable DB pool with health checks, Redis
session/rate-limit integration, and circuit breakers for external APIs.

## Work Packages

| WP  | Task                                                               | Files                                                  |
| --- | ------------------------------------------------------------------ | ------------------------------------------------------ |
| 1   | Add OpenTelemetry + Jaeger exporter (traces)                       | `Cargo.toml`, `src/observability/tracing.rs`           |
| 2   | Request correlation ID middleware (X-Request-Id propagation)       | `src/middleware/correlation.rs`, `src/main.rs`         |
| 3   | Structured JSON logging with request_id, user_id, duration_ms      | `src/main.rs` (tracing-subscriber JSON layer)          |
| 4   | DB pool configurable via env (default 30), health check at startup | `src/main.rs`                                          |
| 5   | Redis connection pool in AppState                                  | `src/main.rs`                                          |
| 6   | Persistent audit log sink (PostgreSQL `audit_events` table)        | `src/auth/audit.rs`, migration                         |
| 7   | Circuit breaker for LLM API calls (Gemini, OpenAI, Claude)         | `src/ml/apis/circuit_breaker.rs`                       |
| 8   | Circuit breaker for broker API calls                               | `src/broker/circuit_breaker.rs`                        |
| 9   | Graceful degradation: health endpoint reflects dependency status   | `src/main.rs` (health handler)                         |
| 10  | Tests + SLO definitions documented                                 | `tests/sprint104_observability_test.rs`, `docs/SLO.md` |

## New Dependencies

```toml
opentelemetry = { version = "0.22", features = ["trace"] }
opentelemetry-otlp = { version = "0.15" }
opentelemetry-jaeger = { version = "0.21" }
tracing-opentelemetry = "0.23"
```

## SQL Migration

```sql
CREATE TABLE audit_events (
    id BIGSERIAL PRIMARY KEY,
    event_type VARCHAR(50) NOT NULL,
    user_id UUID,
    client_ip VARCHAR(45),
    details JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_audit_events_user_id ON audit_events(user_id);
CREATE INDEX idx_audit_events_created_at ON audit_events(created_at);
```

## Environment Variables

```
OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317
OTEL_SERVICE_NAME=investor-os
DB_POOL_MAX_CONNECTIONS=30
DB_POOL_MIN_CONNECTIONS=5
CIRCUIT_BREAKER_FAILURE_THRESHOLD=5
CIRCUIT_BREAKER_RESET_TIMEOUT_SECS=60
```

## SLO Targets

| Metric             | Target  | Measurement                       |
| ------------------ | ------- | --------------------------------- |
| API availability   | 99.9%   | health endpoint up / total checks |
| P99 latency (auth) | < 200ms | OpenTelemetry histogram           |
| P99 latency (data) | < 500ms | OpenTelemetry histogram           |
| Error rate         | < 1%    | 5xx / total requests              |
| Login success rate | > 95%   | successful / total login attempts |

## Acceptance Criteria

- Every request gets a unique X-Request-Id (propagated through logs + traces)
- Traces visible in Jaeger UI with full span hierarchy
- Logs are JSON-structured with request_id, user_id, method, path, status, duration_ms
- DB pool configurable, health check fails startup if PostgreSQL unreachable
- Circuit breaker opens after 5 consecutive failures, resets after 60s
- Audit events persisted to PostgreSQL (login, logout, password change, user CRUD)
- `/api/health` reports real dependency status (db: up/down, redis: up/down)
- `cargo clippy -- -D warnings` zero warnings
- All new tests pass
