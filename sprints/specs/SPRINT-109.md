# Sprint 109: Wire Portfolio/Tax/Strategy Endpoints to Real Services

**Gate:** G56
**Depends on:** Sprint 108
**Started:** 2026-03-04

## Objective

Replace remaining stub/demo handlers for portfolio optimization, strategy
selection, and tax endpoints with calls to the real service modules
(`risk::PortfolioRisk`, `strategy_selector::StrategySelectorEngine`,
`tax::TaxEngine`, `tax::TaxReportingEngine`).

## Work Packages

| WP  | Title                                 | Files         |
| --- | ------------------------------------- | ------------- |
| 1   | Wire portfolio optimization endpoints | `src/main.rs` |
| 2   | Wire strategy/regime endpoints        | `src/main.rs` |
| 3   | Wire tax endpoints                    | `src/main.rs` |
| 4   | Gate: clippy + tests + closeout       | `sprints/`    |
