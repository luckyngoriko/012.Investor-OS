# Sprint 080 Report: CI Dependency Determinism & Runtime Verification Unblocking

## Sprint Result

- Status: in_progress
- Gate: pending (G27)
- Planned scope completion: 38%

## Delivered

1. Sprint 80 activated and PM state synchronized for scope `63-80`.
2. Post-release verification reruns executed from `main` (`0fa670f`):
   - `Full E2E Matrix`: run `22544364233` -> `failure`
   - `Nightly Runtime Contract`: run `22544365372` -> `success`
   - `Release Evidence Bundle`: run `22544364820` -> `success`
3. Captured root cause for matrix failure:
   - `npm ci` failed due lockfile drift (`Missing: @swc/helpers@0.5.19 from lock file`) during `Install frontend dependencies` step across all matrix jobs.
4. Repository hygiene update:
   - removed local artifact `frontend/investor-dashboard/test-results.json`
   - added ignore pattern `frontend/investor-dashboard/test-results*.json`
5. Implemented WP-80A lockfile remediation for CI determinism:
   - updated `frontend/investor-dashboard/package-lock.json` to align `@swc/helpers` with current `next@16.1.6` expectations (`0.5.19`)
6. Implemented WP-80B dependency-integrity guardrail in E2E matrix workflow:
   - added `Verify lockfile integrity` (`npm ci --dry-run`) before install phase in `.github/workflows/full-e2e-matrix.yml`
7. Implemented WP-80C nightly runtime dependency-path hardening:
   - replaced absolute local path dependency with repo-local crate path:
     - `Cargo.toml`: `neurocod-rag = { path = "vendor/neurocod-rag" }`
     - added fallback crate: `vendor/neurocod-rag/`
   - removed skip-only dependency gate from `.github/workflows/nightly-runtime-contract.yml`
   - nightly workflow now always executes strict runtime path (`build -> start -> readiness -> smoke`)

## Not Delivered / Deferred

1. Full matrix rerun on fixed commit is still in progress; final pass/fail evidence pending.
2. G27 close-out evidence packaging (WP-80D) is pending.

## Verification Summary

- `gh run view 22544364233 --json status,conclusion,jobs`: confirmed matrix failure originates at `Install frontend dependencies` (`npm ci`).
- `gh run view 22544364233 --job 65304270429 --log-failed`: captured `npm ci` lockfile mismatch details.
- `gh run view 22544365372 --json status,conclusion,jobs`: nightly workflow completed successfully with skip-path summary.
- `gh run view 22544364820 --json status,conclusion,jobs`: release evidence workflow completed successfully.
- `docker run --rm -v \"$PWD/frontend/investor-dashboard\":/src:ro,Z node:20 bash -lc 'mkdir -p /work && cp /src/package.json /src/package-lock.json /work/ && cd /work && npm ci'`: pass (Node 20 CI-equivalent lockfile validation).
- `cargo check --quiet`: pass (non-blocking warnings only).
- `gh run view 22544580047 --json jobs`: confirmed lockfile validation and install steps pass across all matrix jobs on commit `5e226f7` (run still in progress).
- `gh run list --workflow .github/workflows/nightly-runtime-contract.yml --limit 1`: latest rerun `22544580020` completed `success`.
- Manual PM sync target for active program scope updated to 18 sprints (`17 done`, `1 in_progress`).

## Program Progress

- Total sprints in program: 18
- Completed sprints: 17
- Overall completion: 94%
- Remaining to 100%: 6%

## Open Risks

1. CI dependency-stage determinism is remediated, but full matrix completion risk remains in runtime/functional assertions.
2. Full matrix run can still fail on functional E2E assertions; dependency-stage unblock is necessary but not sufficient.

## Next Sprint Decision

- Next sprint: 80
- Activation status: in_progress
- Preconditions met: yes
