# Sprint 086 Report: Frontend De-Mock - Tax, Security, Portfolio Optimization

## Sprint Result

- Status: done
- Gate: passed (G33)
- Planned scope completion: 100%

## Planned Deliverables

1. Tax page backend wiring.
2. Security page backend wiring.
3. Portfolio optimization page backend wiring.
4. Page-level test evidence.

## Completed Work

1. Added backend proxy routes for domain APIs:
   - `app/api/security/{status,clearance-levels,generate-key}/route.ts`
   - `app/api/tax/{status,calculate}/route.ts`
   - `app/api/portfolio/{optimize,efficient-frontier}/route.ts`
   - shared proxy helper `app/api/_backend-proxy.ts`.
2. Added typed frontend domain client: `lib/domain-api.ts`.
3. Replaced runtime mock datasets in:
   - `app/security/page.tsx`
   - `app/tax/page.tsx`
   - `app/portfolio-opt/page.tsx`
4. Added unit test coverage for domain client:
   - `tests/unit/lib/domain-api.test.ts`.
5. Verification evidence passed:
   - `cd frontend/investor-dashboard && npm test -- --run`
   - `cd frontend/investor-dashboard && npm run build`
   - `cd frontend/investor-dashboard && npx playwright test tests/e2e/dashboard/dashboard.spec.ts --project=chromium`

## Entry Criteria

1. Sprint 85 auth contracts are stable.
2. Target API endpoints are available for integration.
