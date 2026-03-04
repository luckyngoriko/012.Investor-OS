# SPRINT-102: Auth Hardening & Security Middleware

**Phase:** world-class-platform
**Gate:** G49
**Depends on:** Sprint 101

## Objective

Harden the authentication system to resist real-world attacks: account lockout,
per-IP rate limiting (Redis-backed), password policy enforcement, session invalidation
on password change, and security headers middleware.

## Work Packages

| WP  | Task                                                                             | Files                                                      |
| --- | -------------------------------------------------------------------------------- | ---------------------------------------------------------- |
| 1   | Account lockout — track failed attempts, freeze after N failures                 | `src/auth/service.rs`, `src/auth/repository.rs`, migration |
| 2   | Add `failed_login_attempts` + `locked_until` columns to auth_users               | `migrations/20260304000002_auth_lockout.sql`               |
| 3   | Redis-backed per-IP rate limiter for `/api/auth/login`                           | `src/auth/rate_limit.rs`, `src/main.rs`                    |
| 4   | Password policy validation (min 12 chars, mixed case, digit, symbol)             | `src/auth/password.rs`                                     |
| 5   | Revoke all sessions on password change                                           | `src/auth/service.rs`                                      |
| 6   | Security headers middleware (CSP, HSTS, X-Frame-Options, X-Content-Type-Options) | `src/middleware/security_headers.rs`, `src/main.rs`        |
| 7   | Wire Redis into AppState (pool from env)                                         | `src/main.rs`                                              |
| 8   | Unit + integration tests for lockout, rate limit, password policy                | `tests/sprint102_auth_hardening_test.rs`                   |

## SQL Migration

```sql
ALTER TABLE auth_users
    ADD COLUMN failed_login_attempts INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN locked_until TIMESTAMPTZ;
```

## Environment Variables

```
REDIS_URL=redis://localhost:6379
AUTH_MAX_FAILED_LOGINS=5
AUTH_LOCKOUT_DURATION_SECS=900
AUTH_LOGIN_RATE_LIMIT_RPM=10
```

## Acceptance Criteria

- 5 failed logins → account locked for 15 min (configurable)
- Same IP can't hit `/api/auth/login` more than 10x/min
- Password < 12 chars or missing complexity → rejected with clear error
- Password change → all existing sessions revoked
- Every response includes CSP + HSTS + X-Frame-Options headers
- `cargo clippy -- -D warnings` zero warnings
- All new tests pass
