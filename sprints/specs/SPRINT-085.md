# Sprint 085: Production Auth & Identity Integration

## Metadata

- Sprint ID: 85
- Status: done
- Gate: G32
- Owner: Backend + Frontend + Security
- Dependencies: 84

## Objective

Replace mock/demo authentication with production-grade identity and session flow across API and dashboard.

## Scope In

1. Replace frontend mock login path with backend auth API integration.
2. Implement token/session validation for protected API routes.
3. Implement login, logout, refresh, and role-based access checks.
4. Add unit/E2E auth regression coverage.

## Scope Out

1. Advanced SSO/OIDC federation.
2. Billing/account management features.

## Work Packages

1. WP-85A: Backend auth endpoints and token lifecycle.
2. WP-85B: Frontend auth-context migration from mocks to API.
3. WP-85C: Route protection and authorization policies.
4. WP-85D: Test coverage and gate evidence.

## Acceptance Criteria

1. No mock/demo credential path remains in runtime auth flow.
2. Protected routes reject invalid/expired sessions.
3. Auth UX supports successful login/logout/refresh flows.
4. Unit and E2E auth suites pass in CI.

## Verification Commands

```bash
cargo check --locked
cargo test --lib --locked
cd frontend/investor-dashboard && npm test -- --run
cd frontend/investor-dashboard && npx playwright test tests/e2e/auth/login.spec.ts --project=chromium
```

## Gate Condition

G32 passes when production auth is active end-to-end and mock auth is removed from runtime code paths.
