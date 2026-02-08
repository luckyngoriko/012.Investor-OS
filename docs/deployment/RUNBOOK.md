# Investor OS - Operations Runbook

> **Version:** 1.0 | **For:** On-call engineers

## Emergency Contacts

| Role | Contact |
|------|---------|
| Primary On-call | #investor-os-oncall |
| Engineering Lead | #engineering-leads |
| Trading Desk | #trading-desk |

---

## Critical Alerts

### 🔴 Kill Switch Triggered

**Alert:** `KillSwitchTriggered`

**Impact:** All new trades blocked

**Response:**
1. Check kill switch reason in Grafana or API
2. Assess if trigger was intentional
3. If accidental: Investigate cause, do NOT disable without approval
4. If intentional: Wait for all-clear from trading desk

```bash
# Check kill switch status
curl http://localhost:3000/api/killswitch

# View recent decisions
curl http://localhost:3000/api/journal
```

### 🔴 Drawdown Critical (>10%)

**Alert:** `DrawdownCritical`

**Impact:** Portfolio at risk

**Response:**
1. **IMMEDIATELY** verify kill switch is triggered
2. Notify trading desk
3. Review open positions
4. Document all actions in incident log

```bash
# Get current portfolio
curl http://localhost:3000/api/portfolio

# Get all positions
curl http://localhost:3000/api/positions
```

### 🟡 Data Staleness (>24h)

**Alert:** `DataStaleness`

**Impact:** Trading decisions based on old data

**Response:**
1. Check collector status
2. Restart collectors if needed
3. Verify external API availability

```bash
# Check collector logs
docker compose logs --tail 100 collectors

# Restart collectors
docker compose restart collectors
```

### 🟡 Collector Failure

**Alert:** `CollectorFailure`

**Impact:** Missing market data

**Response:**
1. Identify failing collector
2. Check external API limits
3. Restart specific collector
4. Backfill missing data if needed

---

## Common Procedures

### Restart API Service

```bash
docker compose restart api
```

### Database Maintenance

```bash
# Connect to database
docker compose exec postgres psql -U investor -d investor_os

# Check active connections
SELECT count(*) FROM pg_stat_activity;

# Check table sizes
SELECT schemaname, tablename, pg_size_pretty(pg_total_relation_size(schemaname||'.'||tablename)) 
FROM pg_tables 
WHERE schemaname='public';
```

### Redis Operations

```bash
# Check Redis status
docker compose exec redis redis-cli info

# Flush cache (CAUTION!)
docker compose exec redis redis-cli FLUSHDB
```

### Reset Kill Switch (Emergency Only)

⚠️ **Requires VP Trading approval**

```bash
# This requires database access
docker compose exec postgres psql -U investor -d investor_os -c "UPDATE system_config SET value = 'false' WHERE key = 'kill_switch';"
```

---

## Health Checks

### Full System Check

```bash
#!/bin/bash
# health-check.sh

echo "=== Investor OS Health Check ==="

# API
curl -sf http://localhost:3000/api/health > /dev/null && echo "✅ API OK" || echo "❌ API Down"

# Database
docker compose exec -T postgres pg_isready > /dev/null 2>&1 && echo "✅ Database OK" || echo "❌ Database Down"

# Redis
docker compose exec -T redis redis-cli ping > /dev/null 2>&1 && echo "✅ Redis OK" || echo "❌ Redis Down"

# Grafana
curl -sf http://localhost:3001/api/health > /dev/null && echo "✅ Grafana OK" || echo "❌ Grafana Down"

# Prometheus
curl -sf http://localhost:9090/-/healthy > /dev/null && echo "✅ Prometheus OK" || echo "❌ Prometheus Down"

echo "=== Check Complete ==="
```

---

## Incident Response

### Severity Levels

| Level | Examples | Response Time |
|-------|----------|---------------|
| P0 | Kill switch triggered, >10% drawdown | 5 minutes |
| P1 | Data staleness, collector failures | 30 minutes |
| P2 | API slowdown, high memory | 2 hours |
| P3 | Warnings, non-critical alerts | 24 hours |

### Incident Log Template

```
Incident ID: INC-YYYY-MM-DD-###
Severity: P0/P1/P2/P3
Start Time: YYYY-MM-DD HH:MM UTC
End Time: YYYY-MM-DD HH:MM UTC
Duration: X minutes

Summary: Brief description

Impact: What was affected

Root Cause: Why it happened

Resolution: How it was fixed

Prevention: How to avoid recurrence

Actions Taken:
- [ ] Action 1
- [ ] Action 2
```

---

## Monitoring

### Key Metrics

| Metric | Normal | Warning | Critical |
|--------|--------|---------|----------|
| API Response Time | < 200ms | > 500ms | > 2s |
| Drawdown | < 5% | > 5% | > 10% |
| Data Staleness | < 1h | > 6h | > 24h |
| DB Connections | < 50 | > 70 | > 90 |
| Memory Usage | < 70% | > 80% | > 90% |

### Dashboard URLs

- Grafana: http://localhost:3001
- Prometheus: http://localhost:9090
- API Metrics: http://localhost:3000/metrics

---

## Escalation

```
Level 1: On-call Engineer (5 min)
    ↓
Level 2: Engineering Lead (15 min)
    ↓
Level 3: CTO/VP Engineering (30 min)
    ↓
Level 4: Executive Team (1 hour)
```

---

## Useful Commands

```bash
# View all logs
docker compose logs -f

# View specific service logs
docker compose logs -f api

# Check resource usage
docker stats

# List all containers
docker compose ps

# Scale API service
docker compose up -d --scale api=3
```
