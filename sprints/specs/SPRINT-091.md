# Sprint 091: Treasury Production Integrations

## Metadata

- Sprint ID: 91
- Status: queued
- Gate: G38
- Owner: Treasury + Security + Backend
- Dependencies: 89

## Objective

Replace treasury mock/stub runtime paths with production integrations for custody, confirmations, and yield allocation controls.

## Scope In

1. Replace mock custody address/confirmation/withdrawal flows with provider integration.
2. Remove stub-only treasury operations that block production execution.
3. Implement secure provider error handling and audit trails.
4. Add integration tests for critical treasury transaction paths.

## Scope Out

1. New asset-class expansion.
2. Complex cross-chain routing strategies.

## Work Packages

1. WP-91A: Custody provider integration.
2. WP-91B: Withdrawal/confirmation production flow.
3. WP-91C: Yield allocation safety and provider path.
4. WP-91D: Transaction test evidence.

## Acceptance Criteria

1. Treasury runtime does not rely on mock transaction path for core operations.
2. Transaction lifecycle is auditable and resilient to provider failures.
3. Yield allocation path is explicit and non-stub for production mode.
4. Integration tests pass for deposit/withdraw/allocate critical paths.

## Verification Commands

```bash
cargo test --lib --locked treasury::
```

## Gate Condition

G38 passes when treasury critical paths are provider-backed and validated.
