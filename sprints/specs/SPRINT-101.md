# SPRINT-101: Production-Grade RBAC & Multi-User Auth

**Phase:** production-rbac
**Gate:** G48
**Depends on:** Sprint 100

## Objective

Replace demo-level auth (hardcoded users, plain-text passwords, in-memory sessions)
with production-grade PostgreSQL-backed RBAC: Argon2id password hashing, JWT access
tokens, refresh token rotation, admin-only user provisioning.

## Work Packages

| WP  | Task                                              | Status |
| --- | ------------------------------------------------- | ------ |
| 1   | Cargo.toml: add argon2, make jsonwebtoken non-opt | Done   |
| 2   | SQL migration: auth_users + auth_sessions         | Done   |
| 3   | Types + errors (src/auth/types.rs, error.rs)      | Done   |
| 4   | Password hashing + unit tests (password.rs)       | Done   |
| 5   | JWT encode/decode + unit tests (jwt.rs)           | Done   |
| 6   | Repository — all DB queries (repository.rs)       | Done   |
| 7   | Service — business logic (service.rs)             | Done   |
| 8   | Admin seeding on first boot (seed.rs)             | Done   |
| 9   | Module wiring (mod.rs, lib.rs)                    | Done   |
| 10  | Update main.rs — remove old auth, wire new        | Done   |
| 11  | .env.example documentation                        | Done   |
| 12  | Sprint closeout — clippy, tests, tracking         | Done   |

## Key Decisions

| Decision          | Choice     | Rationale                     |
| ----------------- | ---------- | ----------------------------- |
| Password hashing  | Argon2id   | OWASP recommended, pure Rust  |
| Token format      | JWT HS256  | Stateless, no DB call per req |
| Session storage   | PostgreSQL | Reuses existing PgPool        |
| User provisioning | Admin-only | Controlled trading platform   |
| Refresh tokens    | Rotation   | Prevents reuse attacks        |

## New Dependencies

- `argon2 = "0.5"` (password hashing)
- `jsonwebtoken = "9.3"` (was optional, now required)

## Environment Variables

```
JWT_SECRET=<min 32 chars>       # Required
AUTH_ACCESS_TTL_SECONDS=900     # Default: 15 min
AUTH_REFRESH_TTL_SECONDS=604800 # Default: 7 days
AUTH_ADMIN_PASSWORD=<required>  # For initial admin seed
AUTH_ADMIN_EMAIL=admin@investor-os.com  # Optional
AUTH_ADMIN_NAME=Admin User              # Optional
DATABASE_URL=postgres://...     # Required
```

## Verification

- `cargo clippy -- -D warnings` — zero warnings
- `cargo test --lib` — 266 tests pass (6 new auth tests)
- `cargo check` — clean build
