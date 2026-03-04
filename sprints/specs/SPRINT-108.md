# Sprint 108: Killswitch Persistence & Auth Test Coverage

**Gate:** G55
**Depends on:** Sprint 107
**Started:** 2026-03-04

## Objective

Persist killswitch state server-side (not just localStorage) and add
comprehensive unit tests for the auth subsystem (login flow, TOTP, API keys,
audit, middleware helpers).

## Work Packages

| WP  | Title                           | Files                                                |
| --- | ------------------------------- | ---------------------------------------------------- |
| 1   | Killswitch backend persistence  | `src/main.rs`, `migrations/`, frontend settings page |
| 2   | Auth integration tests          | `src/auth/` test modules                             |
| 3   | Gate: clippy + tests + closeout | `sprints/`                                           |
