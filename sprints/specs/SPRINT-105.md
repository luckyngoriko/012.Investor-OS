# SPRINT-105: Testing, CI Security & Frontend Hardening

**Phase:** world-class-platform
**Gate:** G52
**Depends on:** Sprint 103, Sprint 104

## Objective

Close all testing and CI gaps: auth integration test suite, security scanning in
CI pipeline, load test baseline, frontend error retry logic, accessibility audit,
and 2FA integration into the login flow.

## Work Packages

| WP  | Task                                                                | Files                                                               |
| --- | ------------------------------------------------------------------- | ------------------------------------------------------------------- |
| 1   | Auth integration test: login → refresh → logout → expired → lockout | `tests/sprint105_auth_e2e_test.rs`                                  |
| 2   | Add `cargo-audit` to CI pipeline                                    | `.github/workflows/ci.yml`                                          |
| 3   | Add `npm audit` for frontend in CI                                  | `.github/workflows/ci.yml`                                          |
| 4   | Add Trivy container scanning in CI                                  | `.github/workflows/ci.yml`                                          |
| 5   | k6 load test script (target: 1000 RPS, P99 < 200ms)                 | `tests/load/k6_auth_baseline.js`                                    |
| 6   | Frontend: exponential backoff retry on API errors                   | `frontend/investor-dashboard/lib/api-client.ts`                     |
| 7   | Frontend: error boundaries per page section                         | `frontend/investor-dashboard/components/section-error-boundary.tsx` |
| 8   | Frontend: accessibility audit + fix top WCAG 2.1 AA violations      | `frontend/investor-dashboard/components/`                           |
| 9   | 2FA TOTP integration in login flow (optional per user)              | `src/auth/totp.rs`, `src/auth/service.rs`, `src/main.rs`            |
| 10  | Session data retention policy: auto-cleanup expired sessions (cron) | `src/auth/cleanup.rs`, `src/main.rs`                                |

## New Dependencies

### Backend

```toml
totp-rs = { version = "5", features = ["gen_secret"] }
```

### Frontend

None (retry logic uses built-in fetch).

## CI Pipeline Addition

```yaml
# In .github/workflows/ci.yml
- name: Security Audit (Rust)
  run: |
    cargo install cargo-audit --locked
    cargo audit

- name: Security Audit (Frontend)
  run: |
    cd frontend/investor-dashboard
    npm audit --audit-level=high

- name: Container Scan
  uses: aquasecurity/trivy-action@master
  with:
    image-ref: investor-os:latest
    severity: CRITICAL,HIGH
```

## 2FA Flow

```
POST /api/auth/login  { email, password }
  → if user.totp_enabled:
      → 200 { requires_2fa: true, challenge_token: "..." }
      → POST /api/auth/verify-totp { challenge_token, totp_code }
        → 200 { access_token, refresh_token, ... }
  → else:
      → 200 { access_token, refresh_token, ... }

POST /api/auth/totp/setup   → QR code + secret (admin/self)
POST /api/auth/totp/confirm → Enable after verifying first code
DELETE /api/auth/totp        → Disable 2FA (requires current TOTP code)
```

## Acceptance Criteria

- Auth integration test covers: happy path, bad password, lockout, refresh rotation, logout, expired token
- CI pipeline blocks on `cargo audit` finding CRITICAL/HIGH advisories
- CI pipeline blocks on Trivy CRITICAL/HIGH container vulnerabilities
- k6 baseline: `/api/auth/login` sustains 1000 RPS with P99 < 200ms
- Frontend retries failed requests 3x with exponential backoff (1s, 2s, 4s)
- Frontend pages have section-level error boundaries (crash one panel, rest stays)
- TOTP 2FA fully functional: setup → QR → verify → login requires code
- Expired sessions auto-cleaned every hour (configurable)
- `cargo clippy -- -D warnings` zero warnings
- All new tests pass
