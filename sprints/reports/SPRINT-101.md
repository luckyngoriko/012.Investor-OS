# SPRINT-101 Report: Production-Grade RBAC & Multi-User Auth

**Status:** DONE
**Date:** 2026-03-04
**Gate:** G48

## Summary

Replaced demo-level auth (3 hardcoded users, plain-text passwords, in-memory HashMap sessions)
with production-grade PostgreSQL-backed RBAC authentication system.

## Changes

### New Module: `src/auth/` (7 files)

- **error.rs** — `AuthError` enum with thiserror
- **types.rs** — `UserRole`, `AuthUser`, `AccessClaims`, request/response types
- **password.rs** — Argon2id hash + verify with 2 unit tests
- **jwt.rs** — `JwtConfig` with encode/decode and 3 unit tests
- **repository.rs** — All SQL queries for auth_users + auth_sessions (runtime, no compile-time macros)
- **service.rs** — `AuthService`: login, refresh, logout, CRUD with token rotation
- **seed.rs** — `seed_admin_if_empty()` for first boot
- **mod.rs** — Re-exports

### Modified Files

- **Cargo.toml** — Added `argon2 = "0.5"`, made `jsonwebtoken` non-optional
- **src/lib.rs** — Added `pub mod auth`
- **src/main.rs** — Removed ~250 lines old auth, wired new `auth::AuthService`,
  JWT middleware (CPU-only, no DB per request), admin CRUD routes

### New Files

- **migrations/20260304000001_auth_users_sessions.sql** — `auth_users` + `auth_sessions` tables

### API Endpoints (new)

- `POST /api/admin/users` — Create user (admin-only)
- `GET /api/admin/users` — List users (admin-only)
- `GET /api/admin/users/:id` — Get user (admin-only)
- `PUT /api/admin/users/:id` — Update user (admin-only)

### API Contract (unchanged)

- `POST /api/auth/login` — Same request/response shape
- `POST /api/auth/refresh` — Same request/response shape
- `POST /api/auth/logout` — Same request/response shape
- `GET /api/auth/me` — Same response shape

## Verification

| Gate             | Result |
| ---------------- | ------ |
| clippy -D warn   | PASS   |
| cargo test --lib | 266 OK |
| cargo check      | PASS   |

## New Unit Tests (6)

1. `auth::password::hash_and_verify_roundtrip`
2. `auth::password::different_hashes_for_same_password`
3. `auth::jwt::encode_decode_roundtrip`
4. `auth::jwt::expired_token_rejected`
5. `auth::jwt::wrong_secret_rejected`
6. (broker::ib::client test also runs — preexisting)
