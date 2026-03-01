# Sprint 084: Security Gate Install Resilience & Remediation Retry

## Metadata

- Sprint ID: 84
- Status: done
- Gate: G31
- Owner: Platform + Security
- Dependencies: 83

## Objective

Harden PR security gate reliability during `cargo-audit` installation and perform another direct remediation attempt for `RUSTSEC-2024-0437` under current environment constraints.

## Scope In

1. Add retry and network diagnostics around `cargo install cargo-audit --locked` in PR checks.
2. Re-attempt dependency remediation for `RUSTSEC-2024-0437` (`prometheus` -> `protobuf` chain).
3. Capture explicit blocker evidence for DNS/offline-cache failure mode if remediation cannot complete.
4. Synchronize PM governance artifacts for Sprint 84 close-out.

## Scope Out

1. Migration to pre-release dependency tracks (e.g., alpha DB stacks) for advisory suppression.
2. Non-security runtime feature work.

## Work Packages

1. WP-84A: CI install resilience for security gate tooling.
2. WP-84B: Advisory remediation retry execution.
3. WP-84C: Risk register refresh with updated blocker evidence.
4. WP-84D: G31 governance synchronization and close-out.

## Acceptance Criteria

1. PR workflow includes bounded retry behavior (3 attempts) and diagnostics for cargo-audit install failures.
2. `RUSTSEC-2024-0437` remediation retry is executed with command evidence captured.
3. Backlog/risk state reflects the latest blocker evidence and owner accountability.
4. PM artifacts reflect Sprint 84 done and program completion remains 100%.

## Verification Commands

```bash
cargo check --locked
cargo test --lib --locked
cargo audit --db .cache/advisory-db --no-fetch --stale
gh pr checks <pr_number>
```

## Gate Condition

G31 passes when PR security gate install path is resilient/diagnostic and advisory retry evidence is documented with consistent governance state.

## Evidence Required

1. Workflow diff for resilient `Install cargo-audit` step.
2. Remediation retry command outputs including DNS/offline blocker traces.
3. Updated Sprint 84 report and synchronized PM files.

## Risks

1. Intermittent crates.io DNS resolution in execution environment can continue blocking dependency updates.
2. Offline cache may not contain new transitive packages required by upgraded dependencies.
