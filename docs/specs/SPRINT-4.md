# Sprint 4: Web Interface + Monitoring

> **Duration:** Week 7-8
> **Goal:** Dashboard UI, Grafana monitoring, production readiness
> **Ref:** [SPEC-v1.0](./SPEC-v1.0.md) | [Golden Path](./GOLDEN-PATH.md)

---

## Scope

### ✅ In Scope
- Next.js dashboard
- Proposals review page
- Positions tracking page
- Decision journal view
- Grafana dashboards
- Alert rules
- Production documentation

### ❌ Out of Scope
- Mobile app
- Broker integration
- Automated trade execution

---

## Deliverables

| ID | Deliverable | Acceptance Criteria |
|----|-------------|---------------------|
| S4-D1 | Dashboard home | Shows NAV, regime, pending proposals |
| S4-D2 | Proposals page | List, detail, confirm/reject buttons |
| S4-D3 | Positions page | Current holdings with P&L |
| S4-D4 | Journal page | Decision history with outcomes |
| S4-D5 | Grafana: System | CPU, memory, container health |
| S4-D6 | Grafana: Pipeline | Collector success/failure rates |
| S4-D7 | Grafana: Portfolio | NAV, drawdown, win rate |
| S4-D8 | Alerts | Data staleness, drawdown, kill switch |
| S4-D9 | Auth | JWT login for dashboard |
| S4-D10 | Documentation | Runbook, deployment guide |

---

## Technical Implementation

### S4-D1: Dashboard Home

```tsx
// app/page.tsx
import { Card, CardContent, CardHeader } from '@/components/ui/card'

export default async function Dashboard() {
  const portfolio = await fetch('/api/portfolio').then(r => r.json())
  const regime = await fetch('/api/regime').then(r => r.json())
  const proposals = await fetch('/api/proposals?status=PENDING').then(r => r.json())
  
  return (
    <div className="grid grid-cols-3 gap-4 p-6">
      {/* NAV Card */}
      <Card>
        <CardHeader>Portfolio Value</CardHeader>
        <CardContent>
          <div className="text-3xl font-bold">€{portfolio.nav.toFixed(2)}</div>
          <div className={portfolio.daily_pnl >= 0 ? 'text-green-500' : 'text-red-500'}>
            {portfolio.daily_pnl > 0 ? '+' : ''}{(portfolio.daily_pnl * 100).toFixed(2)}% today
          </div>
        </CardContent>
      </Card>
      
      {/* Regime Card */}
      <Card>
        <CardHeader>Market Regime</CardHeader>
        <CardContent>
          <RegimeBadge regime={regime.regime} />
          <div className="text-sm text-gray-500">VIX: {regime.vix}</div>
        </CardContent>
      </Card>
      
      {/* Pending Proposals Card */}
      <Card>
        <CardHeader>Pending Decisions</CardHeader>
        <CardContent>
          <div className="text-3xl font-bold">{proposals.length}</div>
          <Link href="/proposals" className="text-blue-500">Review →</Link>
        </CardContent>
      </Card>
    </div>
  )
}
```

### S4-D2: Proposals Page

```tsx
// app/proposals/page.tsx

export default async function ProposalsPage() {
  const proposals = await fetch('/api/proposals').then(r => r.json())
  
  return (
    <div className="p-6">
      <h1 className="text-2xl font-bold mb-4">Trade Proposals</h1>
      
      <div className="space-y-4">
        {proposals.map(p => (
          <ProposalCard key={p.id} proposal={p} />
        ))}
      </div>
    </div>
  )
}

function ProposalCard({ proposal }) {
  return (
    <Card>
      <CardContent className="flex justify-between items-center">
        <div>
          <h3 className="font-bold">{proposal.ticker}</h3>
          <p className="text-sm">{proposal.action} • Size: {(proposal.proposed_size * 100).toFixed(1)}%</p>
          <p className="text-sm text-gray-500">CQ: {proposal.cq_score.toFixed(2)}</p>
        </div>
        
        <div className="flex gap-2">
          <ConfirmButton id={proposal.id} />
          <RejectButton id={proposal.id} />
        </div>
      </CardContent>
      
      <CardFooter>
        <p className="text-sm">{proposal.rationale}</p>
      </CardFooter>
    </Card>
  )
}
```

### S4-D3: Positions Page

```tsx
// app/positions/page.tsx

export default async function PositionsPage() {
  const positions = await fetch('/api/positions').then(r => r.json())
  
  return (
    <Table>
      <TableHeader>
        <TableRow>
          <TableHead>Ticker</TableHead>
          <TableHead>Entry Date</TableHead>
          <TableHead>Entry Price</TableHead>
          <TableHead>Current</TableHead>
          <TableHead>P&L</TableHead>
          <TableHead>% NAV</TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        {positions.map(p => (
          <TableRow key={p.id}>
            <TableCell className="font-bold">{p.ticker}</TableCell>
            <TableCell>{p.entry_date}</TableCell>
            <TableCell>€{p.entry_price}</TableCell>
            <TableCell>€{p.current_price}</TableCell>
            <TableCell className={p.pnl >= 0 ? 'text-green-500' : 'text-red-500'}>
              €{p.pnl.toFixed(2)}
            </TableCell>
            <TableCell>{(p.weight * 100).toFixed(1)}%</TableCell>
          </TableRow>
        ))}
      </TableBody>
    </Table>
  )
}
```

### S4-D5: Grafana Dashboards

```yaml
# config/grafana/dashboards/system.json
{
  "title": "System Health",
  "panels": [
    {
      "title": "Container CPU",
      "type": "timeseries",
      "targets": [{"expr": "container_cpu_usage_seconds_total"}]
    },
    {
      "title": "Container Memory",
      "type": "timeseries",
      "targets": [{"expr": "container_memory_usage_bytes"}]
    },
    {
      "title": "Database Connections",
      "type": "stat",
      "targets": [{"expr": "pg_stat_activity_count"}]
    }
  ]
}

# config/grafana/dashboards/portfolio.json
{
  "title": "Portfolio Performance",
  "panels": [
    {
      "title": "NAV Over Time",
      "type": "timeseries",
      "targets": [{"expr": "investor_os_nav"}]
    },
    {
      "title": "Drawdown",
      "type": "gauge",
      "targets": [{"expr": "investor_os_drawdown"}],
      "thresholds": [
        {"value": -5, "color": "yellow"},
        {"value": -10, "color": "red"}
      ]
    },
    {
      "title": "Win Rate",
      "type": "stat",
      "targets": [{"expr": "investor_os_win_rate"}]
    }
  ]
}
```

### S4-D8: Alert Rules

```yaml
# config/grafana/alerts.yaml
groups:
  - name: investor_os_alerts
    rules:
      - alert: DataStaleness
        expr: time() - investor_os_last_price_update > 86400
        for: 1h
        labels:
          severity: warning
        annotations:
          summary: "Price data is stale (> 24h)"
          
      - alert: DrawdownWarning
        expr: investor_os_drawdown < -0.05
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "Drawdown exceeded 5%"
          
      - alert: DrawdownCritical
        expr: investor_os_drawdown < -0.10
        for: 1m
        labels:
          severity: critical
        annotations:
          summary: "KILL SWITCH: Drawdown exceeded 10%"
          
      - alert: CollectorFailure
        expr: investor_os_collector_errors > 5
        for: 30m
        labels:
          severity: warning
        annotations:
          summary: "Data collector failing repeatedly"
```

---

## Golden Path Tests

### Automated Tests (Playwright)

```typescript
// tests/e2e/golden-path.spec.ts

test.describe('Golden Path', () => {
  test('GP-S4-01: Dashboard loads', async ({ page }) => {
    await page.goto('/')
    await expect(page.locator('text=Portfolio Value')).toBeVisible()
    await expect(page.locator('text=Market Regime')).toBeVisible()
  })
  
  test('GP-S4-02: Proposals page shows pending', async ({ page }) => {
    await page.goto('/proposals')
    await expect(page.locator('h1')).toContainText('Trade Proposals')
  })
  
  test('GP-S4-03: Confirm proposal works', async ({ page }) => {
    await page.goto('/proposals')
    await page.click('button:has-text("Confirm")')
    await expect(page.locator('text=Confirmed')).toBeVisible()
  })
  
  test('GP-S4-04: Reject proposal works', async ({ page }) => {
    await page.goto('/proposals')
    await page.click('button:has-text("Reject")')
    await expect(page.locator('text=Rejected')).toBeVisible()
  })
  
  test('GP-S4-05: Positions page shows holdings', async ({ page }) => {
    await page.goto('/positions')
    await expect(page.locator('table')).toBeVisible()
  })
  
  test('GP-S4-06: Journal page accessible', async ({ page }) => {
    await page.goto('/journal')
    await expect(page.locator('h1')).toContainText('Decision Journal')
  })
  
  test('GP-S4-07: Grafana dashboards load', async ({ page }) => {
    await page.goto('http://localhost:3001')
    await expect(page).toHaveTitle(/Grafana/)
  })
  
  test('GP-S4-08: Kill switch accessible', async ({ page }) => {
    await page.goto('/settings')
    await expect(page.locator('button:has-text("Kill Switch")')).toBeVisible()
  })
})
```

### Manual Checklist

- [ ] Dashboard loads in < 3 seconds
- [ ] NAV displays correctly
- [ ] Regime indicator shows current state
- [ ] Proposals list shows pending items
- [ ] Confirm button works and updates status
- [ ] Reject button works with reason modal
- [ ] Positions table shows all holdings
- [ ] Journal entries are visible
- [ ] Grafana dashboards show metrics
- [ ] Alerts fire on test conditions
- [ ] Kill switch disables trading

---

## Schedule

| Day | Focus |
|-----|-------|
| Day 1 | Next.js setup + dashboard layout |
| Day 2 | Dashboard cards + API integration |
| Day 3 | Proposals page + confirm/reject |
| Day 4 | Positions + journal pages |
| Day 5 | Grafana setup + system dashboard |
| Day 6 | Portfolio dashboard + alerts |
| Day 7 | Authentication + security |
| Day 8 | E2E tests + documentation |

---

## Exit Criteria

Sprint 4 is **COMPLETE** when:
- ✅ All 8 E2E tests pass
- ✅ Manual checklist 100% verified
- ✅ Dashboard loads in < 3s
- ✅ All CRUD operations work
- ✅ Grafana shows real metrics
- ✅ No critical bugs open

---

## Production Readiness Checklist

- [ ] All environment variables documented
- [ ] Docker images built and tagged
- [ ] Deployment runbook created
- [ ] Backup strategy defined
- [ ] Monitoring alerts configured
- [ ] Security review passed
- [ ] Performance tested
