# Sprint 109 Report: Wire Portfolio/Tax/Strategy Endpoints to Real Services

**Status:** DONE
**Started:** 2026-03-04
**Completed:** 2026-03-04
**Gate:** G56

## Summary

Replaced 6 stub/demo handlers with real service calls to production modules.
All portfolio optimization, strategy selection/regime detection, and tax
endpoints now use actual business logic instead of hardcoded demo data.

## Work Packages Delivered

| WP  | Title                                 | Status |
| --- | ------------------------------------- | ------ |
| 1   | Wire portfolio optimization endpoints | Done   |
| 2   | Wire strategy/regime endpoints        | Done   |
| 3   | Wire tax endpoints                    | Done   |
| 4   | Gate: clippy + tests + closeout       | Done   |

## Details

### WP1: Portfolio Optimization

- `optimize_portfolio` — uses `risk::PortfolioRisk` for real VaR/CVaR calculation
- `efficient_frontier` — uses `risk::PortfolioRisk` to compute frontier points with Sharpe/Sortino

### WP2: Strategy/Regime

- `current_regime` — uses `StrategySelectorEngine::detect_regime()` with `MarketIndicators`
- `select_strategy` — uses real `SelectionCriteria`, returns `SelectionScore` + `PerformanceAttribution`
- Added `pub mod strategy_selector` to `src/lib.rs`

### WP3: Tax

- `tax_status` — uses `TaxEngine` for unrealized/realized summaries, wash sale violations, harvest opportunities
- `calculate_tax` — uses `TaxReportingEngine` for report generation, Schedule D, filing deadline

## Files Changed

- `src/main.rs` — 6 handlers rewritten (optimize_portfolio, efficient_frontier, current_regime, select_strategy, tax_status, calculate_tax)
- `src/lib.rs` — added `pub mod strategy_selector`

## Gate Results

| Check                         | Result               |
| ----------------------------- | -------------------- |
| `cargo clippy -- -D warnings` | 0 warnings           |
| `cargo test --lib`            | 347 passed, 0 failed |

## Test Delta

- Previous: 311 tests
- Current: 347 tests (+36 from strategy_selector + tax modules now compiled)
