# Changelog

All notable changes to this project are documented in this file.

## [v3.1-g26-closed] - 2026-03-01

### Added

- Release evidence bundle automation script:
  - `scripts/generate_release_evidence_bundle.sh`
- Automated release evidence workflow for manual/tag-triggered generation:
  - `.github/workflows/release-evidence-bundle.yml`
- Formal release evidence bundle for gate `G26`:
  - `sprints/reports/releases/v3.1-g26-closed/`

### Changed

- Sprint 79 close-out governance state finalized at 100% completion:
  - `.current_sprint`
  - `sprints/active.toml`
  - `sprints/SPRINT_REGISTRY.yaml`
  - `sprints/BOARD.md`
  - `sprints/specs/SPRINT-079.md`
  - `sprints/reports/SPRINT-079.md`
  - `sprints/reports/PROGRESS_SNAPSHOT.md`

## [v3.0-g25-closed] - 2026-03-01

### Added

- Migration guardrail regression suite for portability, seed-schema drift prevention, and idempotent reapply checks:
  - `tests/sprint78_migration_guardrails_test.rs`
- Runtime contract smoke script:
  - `scripts/runtime_contract_smoke.sh`
- Runtime contract E2E validation:
  - `frontend/investor-dashboard/tests/e2e/runtime/runtime-contract.spec.ts`
- Formal release evidence bundle for gate `G25`:
  - `sprints/reports/releases/v3.0-g25-closed/`

### Changed

- PR checks pipeline now includes runtime contract coverage in frontend smoke E2E and curl-based runtime checks:
  - `.github/workflows/pr-checks.yml`
- Sprint governance state synchronized for Sprint 78 close-out at 100% completion:
  - `.current_sprint`
  - `sprints/active.toml`
  - `sprints/SPRINT_REGISTRY.yaml`
  - `sprints/BOARD.md`
  - `sprints/specs/SPRINT-078.md`
  - `sprints/reports/SPRINT-078.md`
  - `sprints/reports/PROGRESS_SNAPSHOT.md`

### Fixed

- Runtime contract E2E race condition caused by response-listener timing in monitoring page assertions.
- Test now performs deterministic backend contract requests through Playwright request context.
