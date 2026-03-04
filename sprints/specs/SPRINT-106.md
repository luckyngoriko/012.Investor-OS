# SPRINT-106: Critical Production Gaps — 2FA, Env Validation, API Keys

**Phase:** world-class-platform
**Gate:** G53
**Depends on:** Sprint 105

## Objective

Close all critical gaps before production launch: wire 2FA/TOTP into the auth flow
with DB persistence, add startup env validation, persist API keys in PostgreSQL,
and replace remaining frontend mock data with real API calls.

## Work Packages

| WP  | Task                                                                    | Files                                           |
| --- | ----------------------------------------------------------------------- | ----------------------------------------------- |
| 1   | Migration: `auth_totp_secrets` + `api_keys` tables                      | `migrations/20260304000004_totp_api_keys.sql`   |
| 2   | TOTP service: setup, verify, enable/disable (wraps existing two_factor) | `src/auth/totp.rs`                              |
| 3   | Wire TOTP into login flow (challenge → verify → tokens)                 | `src/auth/service.rs`, `src/auth/types.rs`      |
| 4   | TOTP API endpoints: setup, confirm, verify, disable                     | `src/main.rs`                                   |
| 5   | API key persistence: generate, list, revoke, validate                   | `src/auth/api_keys.rs`, `src/main.rs`           |
| 6   | Startup env validation — require DATABASE_URL, warn on missing optional | `src/config/validation.rs`, `src/main.rs`       |
| 7   | Frontend: wire security-settings to real APIs (devices, keys, history)  | `frontend/.../components/security-settings.tsx` |
| 8   | Frontend: wire admin page to real API                                   | `frontend/.../app/admin/page.tsx`               |
| 9   | Tests + clippy + sprint closeout                                        | tests, sprints/                                 |

## SQL Migration

```sql
CREATE TABLE auth_totp_secrets (
    user_id UUID PRIMARY KEY REFERENCES auth_users(id) ON DELETE CASCADE,
    secret TEXT NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT false,
    verified_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE api_keys (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES auth_users(id) ON DELETE CASCADE,
    name VARCHAR(100) NOT NULL,
    key_hash TEXT NOT NULL UNIQUE,
    key_prefix VARCHAR(12) NOT NULL,
    permissions JSONB NOT NULL DEFAULT '[]',
    expires_at TIMESTAMPTZ,
    last_used_at TIMESTAMPTZ,
    revoked_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_api_keys_user_id ON api_keys(user_id);
CREATE INDEX idx_api_keys_key_hash ON api_keys(key_hash);
```

## TOTP Login Flow

```
POST /api/auth/login  { email, password }
  → if user has TOTP enabled:
      → 200 { requires_totp: true, challenge_token: "..." }
  → else:
      → 200 { access_token, refresh_token, ... }

POST /api/auth/totp/verify  { challenge_token, code }
  → 200 { access_token, refresh_token, ... }

POST /api/auth/totp/setup    → { secret, qr_uri } (authenticated)
POST /api/auth/totp/confirm  { code } → enable TOTP (authenticated)
DELETE /api/auth/totp         { code } → disable TOTP (authenticated)
```

## Acceptance Criteria

- TOTP setup → QR → verify → login requires 6-digit code
- API keys stored with SHA-256 hash, only prefix shown after creation
- Invalid/missing DATABASE_URL → startup fails with clear error message
- Frontend security page shows real device/key data from backend
- `cargo clippy -- -D warnings` zero warnings
- All new tests pass
