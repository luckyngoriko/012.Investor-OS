# Sprint 085 Report: Production Auth & Identity Integration

## Sprint Result

- Status: in_progress
- Gate: pending (G32)
- Planned scope completion: 80%

## Planned Deliverables

1. Backend token/session auth implementation.
2. Frontend auth de-mock and API wiring.
3. Route authorization enforcement.
4. Auth test evidence.

## Current Progress

1. WP-85A backend auth implemented in `src/main.rs`:
   - added `/api/auth/login`, `/api/auth/refresh`, `/api/auth/me`, `/api/auth/logout`.
   - added in-memory access/refresh session lifecycle with expiry and refresh rotation.
   - added Bearer-token middleware and enforced auth on protected API routes.
2. WP-85B frontend auth migration completed:
   - `frontend/investor-dashboard/lib/auth-context.tsx` migrated from mock users to auth API flow.
   - removed `demo123` / mock credential runtime path from login experience.
   - introduced auth API client and runtime config (`lib/auth-api.ts`, `lib/runtime-config.ts`).
3. WP-85C route protection implemented:
   - added Next middleware (`frontend/investor-dashboard/middleware.ts`) with session-cookie guard.
   - added BFF auth proxy routes in `app/api/auth/*` for backend auth endpoint integration.
4. WP-85D tests partially completed:
   - updated E2E auth helper and suites to non-demo auth helper (`loginAsUser`).
   - added unit tests for auth API client (`tests/unit/lib/auth-api.test.ts`).
   - Rust verification passed: `cargo check --locked`, `cargo test --lib --locked`.

## Risks

1. Frontend test execution is currently blocked by transient DNS issue (`EAI_AGAIN`) while installing npm deps.
2. Final CI evidence for `npm test`, `npm run build`, and Playwright auth smoke is pending rerun after registry connectivity recovery.

## Next Action

1. Re-run `npm ci` once npm registry DNS is stable.
2. Execute sprint verification commands:
   - `cd frontend/investor-dashboard && npm test -- --run`
   - `cd frontend/investor-dashboard && npm run build`
   - `cd frontend/investor-dashboard && npx playwright test tests/e2e/auth/login.spec.ts --project=chromium`
3. Close G32 after full frontend evidence passes.
