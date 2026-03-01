# Sprint 080: CI Dependency Determinism & Runtime Verification Unblocking

## Metadata

- Sprint ID: 80
- Status: in_progress
- Gate: G27
- Owner: Platform + QA + DevEx
- Dependencies: 79

## Objective

Unblock post-release verification by fixing dependency determinism in frontend CI and ensuring nightly runtime contract checks execute meaningful runtime assertions.

## Scope In

1. Resolve frontend lockfile drift so `npm ci` is stable in matrix workflows.
2. Add deterministic dependency-integrity guardrails for Playwright CI paths.
3. Remove or replace nightly runtime hard dependency on unavailable local path dependencies.
4. Re-run release verification workflows and collect passing evidence for G27.

## Scope Out

1. Feature work unrelated to CI/runtime verification reliability.
2. Broad refactors without direct impact on verification determinism.

## Work Packages

1. WP-80A: Frontend lockfile synchronization and `npm ci` stability.
2. WP-80B: CI dependency-integrity guardrails for matrix workflow.
3. WP-80C: Nightly runtime contract execution-path hardening.
4. WP-80D: G27 evidence capture and close-out readiness.

## Acceptance Criteria

1. `Full E2E Matrix` workflow passes dependency-install phase across all matrix projects.
2. `Nightly Runtime Contract` executes an intended runtime validation path (non-placeholder skip) or has explicit approved fallback.
3. Release verification reruns are documented with run IDs, conclusions, and evidence artifacts.
4. PM governance artifacts stay synchronized with actual sprint state and completion metrics.

## Verification Commands

```bash
cd frontend/investor-dashboard && npm ci
gh run list --workflow .github/workflows/full-e2e-matrix.yml --limit 5
gh run list --workflow .github/workflows/nightly-runtime-contract.yml --limit 5
gh run list --workflow .github/workflows/release-evidence-bundle.yml --limit 5
```

## Gate Condition

G27 passes when dependency determinism and runtime verification paths are stable in CI, with no unresolved release-blocking verification gaps.

## Evidence Required

1. Passing workflow runs (matrix/nightly/release-evidence) linked to G27 close-out.
2. Root-cause documentation and remediation evidence for prior `npm ci` failure.
3. Updated PM synchronization artifacts and sprint close-out report.

## Risks

1. Lockfile/package drift can reappear if dependency update flow is not standardized.
2. Nightly runtime checks can produce false confidence when critical checks are skipped by environment constraints.
