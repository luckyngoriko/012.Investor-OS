# Sprint 081: Runtime Warning Budget Enforcement & Signal Hygiene

## Metadata

- Sprint ID: 81
- Status: done
- Gate: G28
- Owner: Frontend + QA + DevEx
- Dependencies: 80

## Objective

Eliminate recurring non-fatal runtime warning noise from CI execution and enforce a measurable warning budget so E2E signals remain actionable and trustworthy.

## Scope In

1. Remove recurring chart/container runtime warnings in browser matrix runs.
2. Add warning-budget capture and threshold checks in Playwright CI execution.
3. Produce warning-clean evidence for gate close-out with traceable artifacts.

## Scope Out

1. New product features unrelated to runtime signal quality.
2. Broad UI redesign work not required for warning elimination.

## Work Packages

1. WP-81A: Frontend chart/container warning root-cause elimination.
2. WP-81B: Playwright warning-budget extraction and CI policy wiring.
3. WP-81C: Governance evidence capture for warning-clean matrix runs.

## Acceptance Criteria

1. Stable matrix execution has no recurring chart-size runtime warnings in console logs.
2. CI produces explicit warning-budget output and fails when threshold is exceeded.
3. G28 evidence includes run IDs and artifacts proving warning-budget compliance.
4. PM governance artifacts remain synchronized with actual sprint state.

## Verification Commands

```bash
gh run list --workflow full-e2e-matrix.yml --limit 5
gh run view <matrix_run_id> --log-failed
cd frontend/investor-dashboard && npx playwright test tests/e2e/accessibility/a11y.spec.ts tests/e2e/performance/performance.spec.ts --project=chromium
```

## Gate Condition

G28 passes when runtime warning noise is reduced to agreed warning-budget thresholds and CI policy enforces regressions deterministically.

## Evidence Required

1. Passing matrix run IDs with warning-budget output attached.
2. Root-cause + remediation evidence for previously observed runtime warnings.
3. Updated sprint report and synchronized PM state artifacts.

## Risks

1. Warning filtering can hide real regressions if patterns are not narrowly scoped.
2. Cross-browser logging differences can create false positives/negatives in budget counting.
