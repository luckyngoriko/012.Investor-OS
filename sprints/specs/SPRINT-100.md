# SPRINT-100: Final Production Purity — 100% Readiness

**Phase:** production-purity
**Gate:** G47
**Depends On:** [99]
**Started:** 2026-03-04

## Objective

Close every remaining mock, TODO, and stale artefact to reach 100% product readiness. After sprints 96-99 eliminated most fake code, this final sprint targets the 5 remaining simulated data functions in API handlers, 5 `alert()` calls in frontend, 7 bare TODO comments, and 2 stale `.bak` files.

## Work Packages

| WP   | Title                                               | Scope                                                                                                 |
| ---- | --------------------------------------------------- | ----------------------------------------------------------------------------------------------------- |
| 100A | Delete dead `market_data.rs`                        | Remove unreferenced file with 3 `rand::thread_rng()` generators                                       |
| 100B | Replace simulated attribution with DB query         | Real positions DB query in `get_attribution` handler                                                  |
| 100C | Replace simulated signals with DB query             | Real signals DB query in `get_ml_prediction` handler                                                  |
| 100D | Replace frontend `alert()` with toast notifications | 5 files, use existing `addNotification`/`showTradeNotification`                                       |
| 100E | Close 7 TODO comments                               | FX conversion, token estimation, memory warnings, JSON schema validation, graph logging, quantum docs |
| 100F | Delete stale `.bak` files                           | `fx.rs.bak`, `fiat.rs.bak`                                                                            |
| 100G | Sprint closeout & final audit                       | Spec, report, active.toml, verification grep audit                                                    |

## Acceptance Criteria

- `cargo clippy -- -D warnings` — zero warnings
- `cargo test --lib` — all tests pass
- `cargo check` — clean build
- `grep -rn 'alert(' frontend/ --include='*.tsx'` — zero results
- `grep -rn 'generate_simulated' src/ --include='*.rs'` — zero results
- `grep -rn 'rand::thread_rng' src/api/ --include='*.rs'` — zero results
