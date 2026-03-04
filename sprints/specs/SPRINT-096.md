# Sprint 096: Treasury Security & Kill-Switch Hardening

## Metadata

- Sprint ID: 96
- Status: in_progress
- Gate: G43
- Owner: Backend + Security
- Dependencies: 95

## Objective

Eliminate the 6 P0 production blockers that make the system dangerous to operate with real capital: hardcoded 2FA bypass, disabled sanctions screening, non-functional kill switch, fake pre-trade risk data, and broken IB order routing.

## Scope In

1. Remove hardcoded 2FA code "123456" from treasury security, implement real TOTP verification.
2. Implement real sanctions/blacklist screening in `is_blacklisted()` (Chainalysis or policy-based).
3. Implement real position flattening in `flatten_all_positions()` so kill switch actually closes positions.
4. Replace placeholder `positions: vec![]` and `account_value: 100000` in pre-trade risk check with real broker data.
5. Implement ticker-to-conid lookup for Interactive Brokers order submission.
6. Implement `get_executions()` for IB broker to return real trade confirmations.

## Scope Out

1. New treasury features (yield optimization, multi-currency).
2. Non-IB broker fixes (Binance, OANDA).
3. Frontend changes.

## Work Packages

1. WP-96A: Treasury 2FA — remove `"123456"` acceptance, implement TOTP via `totp-rs` or HMAC-based OTP with configurable secret per user.
2. WP-96B: Sanctions screening — implement policy-based address blacklist check with configurable deny-list and optional Chainalysis API integration behind feature flag.
3. WP-96C: Kill switch fix — `flatten_all_positions()` must query broker for open positions and submit market close orders for each. Return actual count of closed positions.
4. WP-96D: Pre-trade risk — `execute_proposal()` must call `broker.get_positions()` and `broker.get_account()` for real data instead of placeholders.
5. WP-96E: IB conid lookup — implement `resolve_conid(ticker)` using IB API `/iserver/secdef/search` endpoint or maintain a local ticker→conid cache.
6. WP-96F: IB executions — implement `get_executions()` via IB API `/iserver/account/orders` to retrieve real fill data.

## Acceptance Criteria

1. No hardcoded authentication bypass exists in production code paths.
2. `is_blacklisted()` checks against a configurable deny-list (at minimum).
3. Kill switch flattens all open positions and returns the actual count.
4. Pre-trade risk checks use real broker positions and account value.
5. IB orders are submitted with correct contract IDs.
6. IB execution retrieval returns real fill data from the API.
7. All existing tests pass; new tests verify each fix with computed output.

## Verification Commands

```bash
cargo clippy -- -D warnings
cargo test --lib -- --test-threads=4
cargo test --test '*' -- --test-threads=2
cargo check --locked
```

## Files to Modify

- `src/treasury/security.rs` (WP-96A, WP-96B)
- `src/broker/execution/mod.rs` (WP-96C, WP-96D)
- `src/broker/ib/mod.rs` (WP-96E, WP-96F)
