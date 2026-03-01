# Sprint 090: Distributed Runtime - gRPC + Service Discovery

## Metadata

- Sprint ID: 90
- Status: queued
- Gate: G37
- Owner: Platform + Distributed Systems
- Dependencies: 87, 89

## Objective

Complete production distributed runtime path by implementing gRPC node communication and service discovery integration.

## Scope In

1. Implement real gRPC server/client paths for distributed nodes.
2. Replace `etcd not implemented` path with production discovery backend.
3. Add health checks and failover behavior.
4. Add integration tests for multi-node discovery and inference routing.

## Scope Out

1. Cross-region federation.
2. Autoscaling optimization work.

## Work Packages

1. WP-90A: gRPC server implementation.
2. WP-90B: gRPC client/routing integration.
3. WP-90C: Discovery backend implementation.
4. WP-90D: Multi-node validation evidence.

## Acceptance Criteria

1. No production path returns discovery-not-implemented errors.
2. Node registration/discovery/heartbeat flow works end-to-end.
3. Distributed inference routing succeeds across multiple nodes.
4. Integration tests pass for discovery and routing scenarios.

## Verification Commands

```bash
cargo test --lib --locked distributed::
```

## Gate Condition

G37 passes when distributed runtime communication and discovery are production-operational.
