# Sprint 107 Report: Auth Wiring Completion & Demo Endpoint Cleanup

**Status:** DONE
**Started:** 2026-03-04
**Completed:** 2026-03-04
**Gate:** G54

## Summary

Closed the three blocking production gaps from the post-106 audit:
API keys can now authenticate requests, all auth events are audit-logged,
and demo endpoints returning fake data have been removed or replaced.

## Work Packages Delivered

| WP  | Title                                                        | Status |
| --- | ------------------------------------------------------------ | ------ |
| 1   | X-API-Key middleware validation (fallback auth path)         | Done   |
| 2   | Audit event logging in login/logout/refresh/TOTP handlers    | Done   |
| 3   | Demo endpoint removal + generate-key rewired to real service | Done   |
| 4   | Gate: clippy + tests                                         | Done   |

## Details

### WP1: X-API-Key Auth

- `require_auth_middleware` now tries Bearer JWT first, then X-API-Key
- API key validated via `auth::api_keys::validate_api_key()` → user loaded from DB
- Disabled users rejected even with valid API key
- New `extract_api_key()` helper

### WP2: Audit Logging

- `LoginSuccess` logged on password login and TOTP verify
- `LoginFailed` logged on failed password and failed TOTP
- `TokenRefresh` logged on successful refresh
- `Logout` logged with user ID and client IP
- All fire-and-forget (non-blocking)

### WP3: Demo Cleanup

- Removed `/api/demo/trade` and `/api/demo/positions` (hardcoded fake trades)
- Rewired `/api/security/generate-key` from stub → real `auth::api_keys::create_api_key()`
- Portfolio/tax/strategy endpoints kept as capability docs (already labeled as module descriptions)

## Files Changed

- `src/main.rs` — auth middleware (X-API-Key), audit calls in 4 handlers, demo routes removed, generate-key rewired

## Gate Results

| Check                         | Result               |
| ----------------------------- | -------------------- |
| `cargo clippy -- -D warnings` | 0 warnings           |
| `cargo test --lib`            | 296 passed, 0 failed |
