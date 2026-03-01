# Sprint 082: Security Backlog Burn-down & PR Signal Hardening

## Metadata

- Sprint ID: 82
- Status: in_progress
- Gate: G29
- Owner: Platform + Security + QA
- Dependencies: 81

## Objective

Reduce unresolved security debt that blocks strict PR security gates while preserving deterministic warning-signal enforcement in CI.

## Scope In

1. Enforce warning-budget gating in PR smoke verification.
2. Build a security backlog register from current `cargo audit` output with explicit owner, decision, and due date.
3. Resolve or formally risk-accept high-priority advisories with traceable rationale.
4. Capture verification evidence for G29 close-out.

## Scope Out

1. Product feature development unrelated to CI/security reliability.
2. Large dependency modernization without direct advisory or gate impact.

## Work Packages

1. WP-82A: PR warning-budget gate wiring and evidence output.
2. WP-82B: RustSec advisory triage baseline and prioritization.
3. WP-82C: Targeted remediation or controlled risk acceptance for blocking advisories.
4. WP-82D: G29 evidence capture and governance close-out.

## Acceptance Criteria

1. PR checks fail on warning-budget regressions for smoke E2E execution.
2. A maintained security backlog exists for current advisories with status and ownership.
3. Blocking advisories are either remediated or documented as approved risk with review date.
4. G29 evidence includes PR check, matrix, and release/governance run references.

## Verification Commands

```bash
gh run list --workflow pr-checks.yml --limit 5
gh run view <pr_run_id> --log-failed
gh run list --workflow full-e2e-matrix.yml --limit 5
cargo audit
```

## Gate Condition

G29 passes when PR signal hardening is active and the security backlog has actionable, governed resolution status without unknown blockers.

## Evidence Required

1. Successful PR check runs proving warning-budget gating.
2. Advisory triage register and remediation/risk-accept decisions.
3. Updated sprint report and synchronized PM governance artifacts.

## Risks

1. Some advisories may not have immediate upstream fixes, requiring bounded risk acceptance.
2. Overly broad warning filtering can hide real regressions if not strictly scoped.
