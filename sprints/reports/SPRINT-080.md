# Sprint 080 Report: CI Dependency Determinism & Runtime Verification Unblocking

## Sprint Result

- Status: done
- Gate: passed (G27)
- Planned scope completion: 100%

## Delivered

1. Sprint 80 CI root-cause was isolated:
   - `npm ci` lockfile drift (`@swc/helpers@0.5.19`) broke matrix install stage.
2. WP-80A completed:
   - frontend lockfile synchronized in `frontend/investor-dashboard/package-lock.json`.
3. WP-80B completed:
   - deterministic dependency guard step added to `.github/workflows/full-e2e-matrix.yml`:
     - `Verify lockfile integrity` (`npm ci --dry-run`) before install.
4. WP-80C completed:
   - nightly runtime hard local dependency removed:
     - `Cargo.toml` switched to repo-local `vendor/neurocod-rag`.
   - strict nightly runtime path enforced in `.github/workflows/nightly-runtime-contract.yml`.
5. WP-80D completed:
   - E2E strict-locator failures fixed (duplicate command palette mount + ambiguous risk selector).
   - login accessibility focus flake stabilized in matrix.
6. PM governance artifacts synchronized for sprint close-out and next sprint activation.

## Not Delivered / Deferred

1. No deferred items within Sprint 80 scope.

## Verification Summary

- `Full E2E Matrix` run `22545176963`: `success` on head `2d25d5a` (all projects green).
- `Nightly Runtime Contract` run `22545289397`: `success` (strict runtime path completed).
- `Release Evidence Bundle` run `22545338709`: `success` (`tag=v3.2-g27-closed`).
- `gh run view 22545176963 --json status,conclusion,jobs`: verified per-project matrix pass.
- `gh run view 22545289397 --json status,conclusion,jobs`: verified strict runtime smoke pass.
- `gh run view 22545338709 --json status,conclusion,jobs`: verified release evidence generation pass.

## Program Progress

- Total sprints in program: 19
- Completed sprints: 18
- Overall completion: 95%
- Remaining to 100%: 5%

## Open Risks

1. Non-blocking frontend runtime warnings are still observed in some chart renders and can erode CI signal quality over time.
2. Warning-budget governance is not yet enforced as a gate condition.

## Next Sprint Decision

- Next sprint: 81
- Activation status: in_progress
- Preconditions met: yes
