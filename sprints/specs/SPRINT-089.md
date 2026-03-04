# Sprint 089: Data Source Service SQL Completion

## Metadata

- Sprint ID: 89
- Status: done
- Gate: G36
- Owner: Backend + Database
- Dependencies: 88

## Objective

Complete data source service SQL implementations for list/get/free-source/pricing flows.

## Scope In

1. Implement SQL-backed `list_sources`, `get_source`, and `get_free_sources`.
2. Add pagination/filter semantics and contract validation.
3. Replace `Not implemented` placeholder responses in target endpoints.
4. Add integration tests for service behavior.

## Scope Out

1. New data provider onboarding.
2. UI redesign around data source management.

## Work Packages

1. WP-89A: SQL query implementations.
2. WP-89B: Pagination/filter contract alignment.
3. WP-89C: Error handling and service reliability.
4. WP-89D: Integration test evidence.

## Acceptance Criteria

1. Target service methods no longer return empty/None placeholders by default.
2. Endpoint outputs match contract expectations for populated DB state.
3. Service errors are typed and traceable.
4. Integration tests cover success and empty-result cases.

## Verification Commands

```bash
cargo test --lib --locked data_sources::
```

## Gate Condition

G36 passes when data source service is SQL-complete and validated via tests.
