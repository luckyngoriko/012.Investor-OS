# Sprint 091 Report: Treasury Production Integrations

## Sprint Result

- Status: done
- Gate: PASS (G38)
- Scope completion: 100%
- Program completion after close-out: 92%
- Remaining to 100%: 8%

## Delivered

1. Custody deposit lifecycle moved from mock-confirmation to pending->confirmed provider flow.
2. Treasury `process_deposit` now syncs custody ledger to keep confirmation path consistent.
3. Treasury confirmation and withdrawal reconciliation hardened with non-negative pending balance handling.
4. Yield allocation safety path kept explicit with protocol/currency validation and runtime position tracking.
5. Treasury audit trail extended with deposit initiation/confirmation and withdrawal lifecycle evidence.

## Verification Evidence

```bash
cargo test --lib --locked test_confirm_deposit_runtime_path
cargo test --lib --locked test_process_deposit_syncs_custody_for_withdrawal_confirmation
cargo test --lib --locked test_crypto_deposit_confirms
cargo test --lib --locked test_confirm_unknown_deposit_fails
cargo test --lib --locked treasury::
cargo test --lib --locked
```

Results:
- Targeted treasury/custody tests: PASS.
- `cargo test --lib --locked treasury::`: 25 passed, 0 failed.
- `cargo test --lib --locked`: 217 passed, 0 failed, 2 ignored.

## Notes

- Remaining warning outside sprint scope: future incompatibility notice for `redis v0.24.0`.
