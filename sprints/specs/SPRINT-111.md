# SPRINT-111: Enterprise Project Tracking System

**Status:** done
**Started:** 2026-03-04
**Completed:** 2026-03-04

## Objective

Replace file-based sprint tracking (TOML/YAML/Markdown) with a PostgreSQL-backed
project tracking system. Includes programs, sprints, tasks, dependencies, and a
dashboard API. Seeds the 11-sprint "Production Readiness for Live Trading" program.

## Work Packages

| WP  | Task                                 | Files                                            | Status |
| --- | ------------------------------------ | ------------------------------------------------ | ------ |
| 1   | SQL migration (4 tables + indexes)   | `migrations/20260304000006_project_tracking.sql` | done   |
| 2   | Types + error modules                | `src/projects/types.rs`, `src/projects/error.rs` | done   |
| 3   | Repository (all CRUD)                | `src/projects/repository.rs`                     | done   |
| 4   | Service (business logic)             | `src/projects/service.rs`                        | done   |
| 5   | Seed (production readiness program)  | `src/projects/seed.rs`                           | done   |
| 6   | Module wiring                        | `src/projects/mod.rs`, `src/lib.rs`              | done   |
| 7   | API handlers + routes (12 endpoints) | `src/main.rs`                                    | done   |
| 8   | Gate: clippy + build verification    | —                                                | done   |

## API Endpoints (12)

```
GET    /api/projects/dashboard
GET    /api/projects/programs
POST   /api/projects/programs
GET    /api/projects/programs/:id
PUT    /api/projects/programs/:id/status
GET    /api/projects/sprints
GET    /api/projects/sprints/:number
POST   /api/projects/sprints/:number/start
POST   /api/projects/sprints/:number/done
GET    /api/projects/tasks
PUT    /api/projects/tasks/:id/status
GET    /api/projects/roadmap
```

## Seeded Program: Production Readiness for Live Trading (S111-S121)

11 sprints with dependency graph, covering: project tracking, live market data,
broker sandbox, order reconciliation, position persistence, strategy state, TLS/secrets,
kill switch E2E, soak testing, and live cutover.

## Gates

- clippy: 0 warnings
- tests: 347 passed
- build: clean
