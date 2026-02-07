---
description: Golden Path test writing workflow for Investor OS
---
// turbo-all

# Golden Path Test Workflow

## Philosophy
> "Lose small many times, win big few times" — Richard Dennis
> Same applies to tests: fail fast on bad code, pass big on golden paths.

## Steps

### 1. Identify the Golden Path
The critical user journey being tested. Example:
```
Data collected → Signals calculated → CQ scored → Proposal generated → User confirms → Journal logged
```

### 2. Write Test FIRST (RED)
```rust
// tests/golden_path/gp_s1_01_database_ready.rs

#[tokio::test]
async fn gp_s1_01_postgres_accepts_connections() {
    let pool = get_test_pool().await;
    let row: (i64,) = sqlx::query_as("SELECT 1")
        .fetch_one(&pool).await.unwrap();
    assert_eq!(row.0, 1);
}
```

### 3. Implement Minimum Code (GREEN)
Write just enough production code to make the test pass.

### 4. Refactor (BLUE)
Clean up without breaking the test.

## Naming Convention
```
GP-S{sprint}_{number}_{description}
Example: GP-S1-01 → gp_s1_01_postgres_connection
```

## Rules
- ❌ NEVER delete a passing test
- ❌ NEVER skip a failing test with `#[ignore]`
- ✅ Every sprint has 6-8 GP tests minimum
- ✅ Tests must be deterministic (no network calls in unit tests)
- ✅ Financial calculations tested with known expected values
