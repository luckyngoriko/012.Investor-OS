---
description: Sprint gate protocol for Investor OS
---
// turbo-all

# Sprint Test Gate Protocol

## Pre-flight Check
```bash
# Must ALL pass before sprint advances
cargo test -- --test-threads=1
cargo clippy -- -D warnings
cargo build --release
cargo doc --no-deps
```

## 5-Gate Checklist

### Gate 1: Golden Path (MANDATORY)
```bash
cargo test golden_path -- --test-threads=1
```
- All GP tests for current sprint: GREEN
- No `#[ignore]` on GP tests

### Gate 2: Clippy Clean
```bash
cargo clippy -- -D warnings
```
- Zero warnings
- Zero errors

### Gate 3: Build
```bash
cargo build --release
```
- Clean release build
- No unused dependencies warning

### Gate 4: Coverage
```bash
cargo llvm-cov --html
# Open target/llvm-cov/html/index.html
```
- Line coverage ≥ 80%
- All financial calculation paths covered

### Gate 5: Documentation
```bash
cargo doc --no-deps
```
- No documentation warnings
- All public APIs documented
- DECISION_LOG.md updated with sprint decisions
- BORROWED.md updated if new reuse

## Sprint Status Markers
```markdown
⬜ Not started
🏃 In progress
✅ Complete (all 5 gates passed)
❌ Blocked
```

## Rollback Procedure
If gate fails:
1. Fix the issue
2. Re-run ALL 5 gates (not just the failed one)
3. Only advance when ALL gates pass simultaneously
