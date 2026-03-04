# Sprint 097: Execution Engine & Compliance Real Data

## Metadata

- Sprint ID: 97
- Status: queued
- Gate: G44
- Owner: Backend
- Dependencies: 96

## Objective

Replace all remaining hardcoded/mock values in the execution engine, connection health, and compliance paths with real data sources.

## Scope In

1. Replace hardcoded fill price `Decimal::from(50000)` in TWAP/venue execution with real market prices from streaming or venue API.
2. Implement real connection health caching for `IbBroker::is_connected()`.
3. Extract real `user_id` from JWT claims in GDPR handlers instead of `"test-user-id"`.
4. Implement real crypto address generation behind `fireblocks` feature flag or disable crypto deposit path without it.
5. Fix paper broker `portfolio.summary(&prices)` to use cached streaming prices instead of empty HashMap.

## Scope Out

1. New execution algorithms (VWAP, iceberg).
2. New compliance frameworks beyond GDPR.
3. Distributed HRM (separate sprint).

## Work Packages

1. WP-97A: Real fill prices — execution engine queries last price from streaming cache or venue API before creating Fill structs. Fallback: reject order if no price available.
2. WP-97B: IB connection health — cache auth result in `AtomicBool`, update on `authenticate()` success/failure, return cached value from `is_connected()`.
3. WP-97C: GDPR user extraction — extract `user_id` from request `Authorization` header JWT claims in `forget_me()` and `export_data()` handlers.
4. WP-97D: Crypto address generation — gate behind `fireblocks` feature flag; without flag, return error "Crypto deposits require custody provider configuration".
5. WP-97E: Paper broker prices — inject streaming price cache into paper broker; use last known price for portfolio summary calculations.

## Acceptance Criteria

1. No execution fills use hardcoded prices; all fills reflect market data or order is rejected.
2. `is_connected()` returns actual connection state.
3. GDPR handlers operate on the authenticated user, not a test placeholder.
4. Crypto deposit without `fireblocks` feature returns a clear error.
5. Paper broker portfolio summary shows correct position values.
6. All existing tests pass; new tests verify each fix.

## Verification Commands

```bash
cargo clippy -- -D warnings
cargo test --lib -- --test-threads=4
cargo test --test '*' -- --test-threads=2
```

## Files to Modify

- `src/execution/mod.rs` (WP-97A)
- `src/broker/ib/mod.rs` (WP-97B)
- `src/compliance/gdpr.rs` (WP-97C)
- `src/treasury/crypto.rs` (WP-97D)
- `src/broker/paper/engine.rs` (WP-97E)
