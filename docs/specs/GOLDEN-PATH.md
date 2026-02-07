# Investor OS - Golden Path Testing Strategy

> **Document:** Master Golden Path
> **Version:** 1.0
> **Date:** 2026-02-04

---

## Overview

This document defines the **Golden Path** testing strategy for Investor OS. Each sprint has specific acceptance criteria validated through automated and manual tests.

---

## Sprint Structure

| Sprint | Focus | Duration | Golden Path |
|--------|-------|----------|-------------|
| [Sprint 1](./SPRINT-1.md) | Foundation | Week 1-2 | Database + Docker + Collectors |
| [Sprint 2](./SPRINT-2.md) | Signals | Week 3-4 | QVM + Insider + Sentiment Scorers |
| [Sprint 3](./SPRINT-3.md) | Decision Engine | Week 5-6 | CQ Calculator + API |
| [Sprint 4](./SPRINT-4.md) | Interface | Week 7-8 | Web UI + Monitoring |

---

## Golden Path Definition

The **Golden Path** is the critical user journey that must work correctly:

```
1. Data collectors fetch fresh data
2. Signal engine calculates scores
3. Decision engine generates CQ-based proposals
4. User sees proposals in dashboard
5. User confirms/rejects proposal
6. System logs decision in journal
7. (Optional) Trade executes via broker
```

---

## Test Categories

### 1. Unit Tests
- Individual function correctness
- Signal calculation accuracy
- Score normalization

### 2. Integration Tests
- Database connectivity
- API endpoint responses
- Service-to-service communication

### 3. End-to-End Tests
- Full golden path execution
- Browser-based UI testing
- Proposal confirmation flow

### 4. Data Quality Tests
- Data freshness validation
- Schema conformance
- Null/missing value handling

---

## Sprint Golden Path Tests

### Sprint 1: Foundation
```
✓ Docker Compose stack starts successfully
✓ PostgreSQL accepts connections
✓ TimescaleDB hypertable created
✓ Redis ping responds
✓ Celery worker picks up tasks
✓ Finnhub collector fetches data
✓ Price data stored in database
```

### Sprint 2: Signals
```
✓ QVM scorer calculates all three factors
✓ Score range is 0-1 for all signals
✓ Insider score updates on new Form 4
✓ Sentiment score reflects StockTwits data
✓ Regime detector returns valid state
✓ All signals stored in database
```

### Sprint 3: Decision Engine
```
✓ CQ score calculated correctly
✓ Trade proposal generated for CQ > 0.5
✓ No proposal for CQ < 0.5
✓ Position sizing matches regime
✓ API returns proposals list
✓ Confirm endpoint updates status
✓ Journal entry created on decision
```

### Sprint 4: Interface
```
✓ Dashboard loads in < 3s
✓ Proposals display with all details
✓ Confirm button triggers API call
✓ Reject button triggers API call
✓ Position list shows current holdings
✓ Grafana dashboard displays metrics
✓ Kill switch accessible and functional
```

---

## Test Execution

### Automated (CI/CD)

```bash
# Run all tests
make test

# Run specific sprint tests
make test-sprint1
make test-sprint2
make test-sprint3
make test-sprint4

# Run golden path e2e
make test-golden-path
```

### Manual Checklist

Each sprint includes a manual validation checklist in the sprint specification.

---

## Success Criteria

Sprint is **COMPLETE** when:
1. ✅ All automated tests pass
2. ✅ Manual checklist verified
3. ✅ Golden path for sprint works end-to-end
4. ✅ No critical/high bugs open
5. ✅ Documentation updated

---

## Links

- [Sprint 1 Specification](./SPRINT-1.md)
- [Sprint 2 Specification](./SPRINT-2.md)
- [Sprint 3 Specification](./SPRINT-3.md)
- [Sprint 4 Specification](./SPRINT-4.md)
