---
description: Sprint execution workflow for Investor OS
---
// turbo-all

# Sprint Execution Workflow

## Pre-Work
1. Read sprint spec: `docs/specs/SPRINT-{N}.md`
2. Read golden path tests for this sprint
3. Verify previous sprint gate: ALL 5 gates passed
4. Create sprint branch: `git checkout -b sprint-{N}`

## Implementation Order
1. **Database migrations FIRST** — `migrations/`
2. **Domain types** — `crates/investor-core/`
3. **Business logic** — `crates/investor-signals/` or `crates/investor-decision/`
4. **API endpoints** — `crates/investor-api/`
5. **Golden Path tests** — `tests/golden_path/`
6. **Integration tests** — `tests/`

## Daily Loop
```
1. Write GP test (RED)
2. Implement code (GREEN)
3. Refactor (BLUE)
4. cargo clippy -- -D warnings
5. cargo test
6. Commit with message: "S{N}: {description}"
```

## Sprint Completion
1. Run full test gate:
   ```bash
   cargo test -- --test-threads=1
   cargo clippy -- -D warnings
   cargo build --release
   cargo doc --no-deps
   ```
2. Update `DECISION_LOG.md` with sprint decisions
3. Update `BORROWED.md` if new reuse
4. Mark sprint complete in `docs/specs/SPRINT-{N}.md`
5. Merge to main: `git checkout main && git merge sprint-{N}`

## Common Mistakes
- ❌ Skipping migrations → runtime DB errors
- ❌ Using f64 for money → precision bugs
- ❌ Not testing edge cases (NaN, zero division, empty data)
- ❌ Hardcoding API keys in test files
- ❌ Ignoring kill switch in new features
