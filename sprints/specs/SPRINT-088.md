# Sprint 088: Analytics Real Market Data Backtesting

## Metadata

- Sprint ID: 88
- Status: done
- Gate: G35
- Owner: Analytics + Data Engineering
- Dependencies: 86, 87

## Objective

Replace analytics mock historical data generation with real market data ingestion for backtesting.

## Scope In

1. Remove mock historical data generation from analytics service runtime path.
2. Integrate real historical data provider/query pipeline.
3. Enforce data-quality guards and error handling.
4. Add deterministic regression tests with fixture datasets.

## Scope Out

1. New strategy research.
2. Advanced portfolio theory features unrelated to data realism.

## Work Packages

1. WP-88A: Data connector for historical bars.
2. WP-88B: Backtest service integration.
3. WP-88C: Data quality/validation checks.
4. WP-88D: Test evidence and baseline metrics.

## Acceptance Criteria

1. Analytics runtime does not depend on generated mock bars.
2. Backtest runs on real sourced data with explicit quality checks.
3. Failure modes are surfaced and traceable.
4. Backtest tests pass with real/fixture datasets.

## Verification Commands

```bash
cargo test --lib --locked analytics::
```

## Gate Condition

G35 passes when analytics backtesting runs against real data sources with passing regression evidence.
