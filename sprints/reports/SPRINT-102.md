# SPRINT-102 Report: Auth Hardening & Security Middleware

**Status:** DONE
**Started:** 2026-03-04
**Completed:** 2026-03-04
**Gate:** G49 PASSED

## Summary

Hardened the authentication system with account lockout, password policy enforcement,
session invalidation on password change, Redis-backed login rate limiting, and
OWASP-recommended security headers on all responses.

## Completed Work Packages

| WP  | Task                                  | Status |
| --- | ------------------------------------- | ------ |
| 1   | Account lockout logic                 | DONE   |
| 2   | Migration: lockout columns            | DONE   |
| 3   | Redis-backed per-IP rate limiter      | DONE   |
| 4   | Password policy validation (5 rules)  | DONE   |
| 5   | Session revocation on password change | DONE   |
| 6   | Security headers middleware           | DONE   |
| 7   | Redis wiring in AppState              | DONE   |
| 8   | Tests + clippy + closeout             | DONE   |

## Key Deliverables

### Account Lockout (WP1-2)

- `LockoutConfig` with env-based configuration (`AUTH_MAX_FAILED_LOGINS`, `AUTH_LOCKOUT_DURATION_SECS`)
- Default: 5 failed attempts → 15 min lockout
- `login()` checks lockout, increments failed attempts, auto-locks, resets on success
- Migration adds `failed_login_attempts` (INTEGER) and `locked_until` (TIMESTAMPTZ) columns
- Repository functions: `increment_failed_logins()`, `lock_user()`, `reset_failed_logins()`

### Password Policy (WP4)

- Min 12 characters, uppercase, lowercase, digit, special character required
- Enforced on user creation and password update
- 5 unit tests covering each rule + happy path

### Session Revocation (WP5)

- `update_user()` revokes all sessions when password is changed
- Uses `revoke_all_sessions_for_user()` from repository

### Rate Limiting (WP3+7)

- Redis connection is optional — graceful degradation if unavailable
- `REDIS_URL`, `RATE_LIMIT_LOGIN_MAX`, `RATE_LIMIT_LOGIN_WINDOW_SECS` env vars
- Default: 10 login attempts per IP per 5 minutes
- Returns 429 TOO_MANY_REQUESTS with `retry_after_secs` field
- Applied as route-layer middleware on `/api/auth/login` only

### Security Headers (WP6)

- `security_headers_middleware` applied to all routes via `axum::middleware::from_fn`
- Headers: X-Frame-Options (DENY), X-Content-Type-Options (nosniff),
  Strict-Transport-Security (1 year + includeSubDomains), Content-Security-Policy,
  Referrer-Policy, Permissions-Policy, X-XSS-Protection
- Server header removed to reduce fingerprinting

## Gate Verification

```
cargo clippy -- -D warnings   → 0 warnings
cargo test --lib              → 271 passed, 0 failed
cargo check                   → clean build
```

## Files Changed

- `src/auth/mod.rs` — export LockoutConfig
- `src/auth/error.rs` — AccountLocked, WeakPassword variants
- `src/auth/types.rs` — failed_login_attempts, locked_until fields
- `src/auth/password.rs` — validate_password_policy() + 5 tests
- `src/auth/service.rs` — LockoutConfig, lockout logic, password policy, session revocation
- `src/auth/repository.rs` — lockout queries, updated column lists
- `src/middleware/mod.rs` — security_headers_middleware
- `src/main.rs` — LockoutConfig wiring, Redis rate limiter, security headers layer
- `migrations/20260304000002_auth_lockout.sql` — new migration
