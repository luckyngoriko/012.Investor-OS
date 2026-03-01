# Sprint 083: Always-On Security Audit Gate & Advisory Remediation Track

## Metadata

- Sprint ID: 83
- Status: done
- Gate: G30
- Owner: Platform + Security
- Dependencies: 82

## Objective

Eliminate PR security gate blind spots by making `cargo audit` always-on in CI and attempt direct remediation of the highest-priority advisory path.

## Scope In

1. Make G5 (`cargo audit`) execute in PR checks regardless of optional local dependency availability.
2. Attempt dependency remediation for `RUSTSEC-2024-0437` (`prometheus` -> `protobuf`).
3. Capture blocker evidence if remediation cannot be completed in current execution environment.
4. Close sprint with synchronized PM governance artifacts and verification evidence.

## Scope Out

1. Large dependency graph modernization beyond immediate advisory path.
2. Runtime feature work unrelated to security gating and dependency risk posture.

## Work Packages

1. WP-83A: PR workflow hardening for always-on security audit.
2. WP-83B: Advisory remediation attempt for `prometheus/protobuf` path.
3. WP-83C: Risk governance update for unresolved remediation blockers.
4. WP-83D: G30 evidence capture and close-out reporting.

## Acceptance Criteria

1. PR workflow executes `Install cargo-audit` and `G5 - Security audit (blocking)` without `depcheck` skip conditions.
2. Remediation attempt for `RUSTSEC-2024-0437` is executed and evidence-captured.
3. If blocked, risk handling is explicitly documented with owner, rationale, and due date.
4. PM artifacts show Sprint 83 done and program completion 100%.

## Verification Commands

```bash
cargo check --locked
cargo test --lib --locked
cargo audit --db .cache/advisory-db --no-fetch --stale
gh pr checks <pr_number>
```

## Gate Condition

G30 passes when security audit is always-on in PR CI and remediation status is explicitly governed with evidence.

## Evidence Required

1. Workflow diff proving non-skippable G5 audit execution.
2. Command output evidence for remediation attempt and blocker trace.
3. Updated sprint report and synchronized PM state.

## Risks

1. crates.io/network availability may block dependency remediation execution.
2. Unresolved upstream chains may require temporary risk acceptance renewal.
