# Sprint 085 Report: Production Auth & Identity Integration

## Sprint Result

- Status: done
- Gate: passed (G32)
- Planned scope completion: 100%

## Planned Deliverables

1. Backend token/session auth implementation.
2. Frontend auth de-mock and API wiring.
3. Route authorization enforcement.
4. Auth test evidence.

## Completed Work

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
4. WP-85D tests completed:
   - updated E2E auth helper and suites to non-demo auth helper (`loginAsUser`).
   - added unit tests for auth API client (`tests/unit/lib/auth-api.test.ts`).
   - verification evidence passed:
     - `cd frontend/investor-dashboard && npm test -- --run`
     - `cd frontend/investor-dashboard && npm run build`
     - `cd frontend/investor-dashboard && npx playwright test tests/e2e/auth/login.spec.ts --project=chromium`
