# Sprint 086: Frontend De-Mock - Tax, Security, Portfolio Optimization

## Metadata

- Sprint ID: 86
- Status: queued
- Gate: G33
- Owner: Frontend + API
- Dependencies: 85

## Objective

Replace mock datasets in tax, security, and portfolio optimization pages with real backend contract integration.

## Scope In

1. Remove in-file mock data from `tax`, `security`, and `portfolio-opt` pages.
2. Integrate data fetching, loading/error states, and empty states.
3. Validate schema contracts for all three pages.
4. Add regression tests for real-data rendering.

## Scope Out

1. New analytics modules.
2. New UI feature expansion unrelated to de-mock.

## Work Packages

1. WP-86A: Tax page backend integration.
2. WP-86B: Security page backend integration.
3. WP-86C: Portfolio optimization page backend integration.
4. WP-86D: Test and CI evidence.

## Acceptance Criteria

1. Runtime code in target pages has no mock constants as data source.
2. Pages render backend responses with resilient UX states.
3. Contract failures are surfaced with actionable error messages.
4. Unit and E2E smoke for these pages pass.

## Verification Commands

```bash
cd frontend/investor-dashboard && npm test -- --run
cd frontend/investor-dashboard && npm run build
cd frontend/investor-dashboard && npx playwright test tests/e2e/dashboard/dashboard.spec.ts --project=chromium
```

## Gate Condition

G33 passes when the three target pages are fully backend-driven with passing regression coverage.
