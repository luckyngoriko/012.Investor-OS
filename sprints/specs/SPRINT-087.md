# Sprint 087: HRM Real Inference Model Integration

## Metadata

- Sprint ID: 87
- Status: done
- Gate: G34
- Owner: ML + Platform
- Dependencies: 85, 86

## Objective

Replace placeholder HRM inference logic with real model-backed inference and deterministic runtime behavior.

## Scope In

1. Remove `placeholder_inference` runtime path from production inference flow.
2. Integrate validated model loading and inference execution path.
3. Add health/status signals for model readiness and fallback policy.
4. Add performance and correctness tests for inference.

## Scope Out

1. New model architecture research.
2. Multi-model ensemble features.

## Work Packages

1. WP-87A: Model-loading and validation path hardening.
2. WP-87B: Real inference pipeline integration.
3. WP-87C: Runtime observability and fallback policy.
4. WP-87D: Benchmark and regression evidence.

## Acceptance Criteria

1. Production inference does not use placeholder heuristic.
2. Model load failures are explicit and observable.
3. Inference latency and output stability meet defined thresholds.
4. Unit and integration tests pass for real inference path.

## Verification Commands

```bash
cargo test --lib --locked hrm::inference::tests::
cargo test --lib --locked hrm::weights::tests::
cargo test --locked --test hrm_validation_test
```

## Gate Condition

G34 passes when HRM runtime uses real model inference with validated test evidence.
