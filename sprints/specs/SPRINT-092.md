# Sprint 092: Observability Metrics Production Wiring

## Metadata

- Sprint ID: 92
- Status: done
- Gate: G39
- Owner: SRE + Platform
- Dependencies: 90, 91

## Objective

Replace placeholder metrics endpoint with real Prometheus metrics export and operational observability coverage.

## Scope In

1. Replace placeholder `/metrics` response with registry-based exporter.
2. Wire core service metrics and runtime counters.
3. Validate scrape compatibility and endpoint performance.
4. Add tests and operational checks for metrics integrity.

## Scope Out

1. Dashboard redesign in external monitoring tools.
2. Non-critical observability experiments.

## Work Packages

1. WP-92A: Metrics registry/exporter integration.
2. WP-92B: Core metric instrumentation coverage.
3. WP-92C: Scrape and performance validation.
4. WP-92D: Evidence and runbook update.

## Acceptance Criteria

1. `/metrics` no longer returns placeholder payload.
2. Prometheus can scrape valid metric families from runtime.
3. Key product paths emit metrics with stable labels.
4. Observability tests/checks pass in CI.

## Verification Commands

```bash
cargo test --lib --locked observability::
./scripts/runtime_contract_smoke.sh
```

## Gate Condition

G39 passes when metrics export is production-operational and contract-verified.
