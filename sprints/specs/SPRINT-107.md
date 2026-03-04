# Sprint 107: Auth Wiring Completion & Demo Endpoint Cleanup

**Gate:** G54
**Depends on:** Sprint 106
**Started:** 2026-03-04

## Objective

Close the three blocking production gaps identified in the post-106 audit:
API key auth not usable, audit logging not wired, demo endpoints returning fake data.

## Work Packages

| WP  | Title                                | Files                                                     |
| --- | ------------------------------------ | --------------------------------------------------------- |
| 1   | X-API-Key middleware validation      | `src/main.rs` (auth middleware)                           |
| 2   | Audit event logging in auth handlers | `src/main.rs` (login/logout/refresh/TOTP handlers)        |
| 3   | Demo endpoint removal/deprecation    | `src/main.rs` (demo_trade, demo_positions, stub handlers) |
| 4   | Gate: clippy + tests + closeout      | `sprints/`                                                |

## Acceptance Criteria

- API keys created via `POST /api/auth/api-keys` can authenticate requests via `X-API-Key` header
- All auth events (login success/fail, logout, refresh, TOTP verify) produce audit rows
- No endpoint returns hardcoded fake trade/position data without clear `/api/demo/` prefix deprecation
- `cargo clippy -- -D warnings` = 0 warnings
- `cargo test --lib` = all pass
