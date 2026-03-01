# Evidence Bundle: v3.1-g26-closed

- Generated: 2026-03-01T13:20:57Z
- Release tag: `v3.1-g26-closed`
- Gate: `G26`

## Verification Summary (Imported)

- `.github/workflows/full-e2e-matrix.yml` added and validated for policy alignment with artifact-based governance evidence.
- `.github/workflows/nightly-runtime-contract.yml` added with strict runtime smoke execution path and deterministic teardown.
- `scripts/flaky_triage_report.sh` validated against real Playwright JSON reporter output (sample run), producing markdown/json triage evidence.
- `scripts/generate_release_evidence_bundle.sh` validated (`bash -n`) and executed successfully against a local test tag/output path.
- `.github/workflows/release-evidence-bundle.yml` added for dispatch/tag-triggered evidence generation and artifact publishing.
- `./scripts/generate_release_evidence_bundle.sh --tag v3.1-g26-closed --gate G26 --scope "Sprint 79 close-out release evidence package."` executed successfully and produced bundle artifacts under `sprints/reports/releases/v3.1-g26-closed/`.
- Manual PM sync check: pass (`PM_SYNC_OK total=17 done=17 in_progress=0 completion=100% remaining=0%`).
- `scripts/verify_pm_sync.sh` / `scripts/verify_pm_boundaries.sh` are not present in the current repository baseline, so equivalent PM sync validation was executed via inline governance check.

## Artifacts

1. Release notes:
   - `sprints/reports/releases/v3.1-g26-closed/RELEASE_NOTES.md`
2. Bundle manifest:
   - `sprints/reports/releases/v3.1-g26-closed/MANIFEST.yaml`
3. Sprint report source:
   - `sprints/reports/SPRINT-079.md`
4. Progress snapshot source:
   - `sprints/reports/PROGRESS_SNAPSHOT.md`
