---
description: Agent startup checklist for Investor OS
---
// turbo-all

# Investor OS — Onboarding Workflow

## 1. Read Critical Files
```bash
cat AGENT_SYSTEM.md
cat DECISION_LOG.md
cat BORROWED.md
cat docs/specs/SPEC-v1.0.md
cat docs/specs/GOLDEN-PATH.md
```

## 2. Check Current Sprint
```bash
# Find current sprint status
grep -n "🏃\|IN PROGRESS\|\[/\]" docs/specs/SPRINT-*.md
```

## 3. Verify Build Health
```bash
cargo build 2>&1 | tail -5
cargo test 2>&1 | tail -10
cargo clippy -- -D warnings 2>&1 | tail -5
```

## 4. Check Database
```bash
docker compose ps
# Verify PostgreSQL + TimescaleDB + pgvector
docker compose exec postgres psql -U investor -c "SELECT version();"
docker compose exec postgres psql -U investor -c "SELECT * FROM pg_extension WHERE extname IN ('timescaledb', 'vector');"
```

## 5. Review Open Issues
```bash
grep -rn "TODO\|FIXME\|HACK\|XXX" crates/ --include="*.rs" | head -20
```

## 6. Confirm Understanding
Before ANY code change, verify:
- [ ] Read AGENT_SYSTEM.md
- [ ] Read current sprint spec
- [ ] Build passes
- [ ] Tests pass
- [ ] Understood the task
