# Sprint 1: Foundation

> **Duration:** Week 1-2
> **Goal:** Establish infrastructure and basic data collection
> **Ref:** [SPEC-v1.0](./SPEC-v1.0.md) | [Golden Path](./GOLDEN-PATH.md)

---

## Scope

### ✅ In Scope
- Docker Compose stack
- PostgreSQL + TimescaleDB setup
- Redis for caching/queue
- Celery worker + beat scheduler
- Finnhub data collector
- Price data collector (yfinance)
- Database schema initialization

### ❌ Out of Scope
- Signal calculations
- Web UI
- API endpoints (beyond health check)

---

## Deliverables

| ID | Deliverable | Acceptance Criteria |
|----|-------------|---------------------|
| S1-D1 | docker-compose.yml | All services start with `docker compose up` |
| S1-D2 | PostgreSQL + TimescaleDB | Database accepts connections, hypertable created |
| S1-D3 | Redis | Cache service responds to ping |
| S1-D4 | Celery worker | Worker processes tasks |
| S1-D5 | Celery beat | Scheduler triggers tasks on schedule |
| S1-D6 | Finnhub collector | Fetches and stores EPS estimates |
| S1-D7 | Price collector | Fetches and stores daily OHLCV |
| S1-D8 | Database schema | All tables created per SPEC |

---

## Technical Implementation

### S1-D1: Docker Compose

```yaml
services:
  - postgres (timescale/timescaledb:latest-pg15)
  - redis (redis:7-alpine)
  - data-collector (custom Python image)
  - celery-worker (same image, different command)
  - celery-beat (same image, scheduler)
```

### S1-D2: Database Schema

```sql
-- Companies reference table
CREATE TABLE companies (
    id SERIAL PRIMARY KEY,
    ticker VARCHAR(10) UNIQUE NOT NULL,
    name VARCHAR(255),
    sector VARCHAR(100),
    market_cap_category VARCHAR(20)
);

-- Time-series price data
CREATE TABLE prices (
    time TIMESTAMPTZ NOT NULL,
    ticker VARCHAR(10) NOT NULL,
    open DECIMAL, high DECIMAL, low DECIMAL, close DECIMAL,
    volume BIGINT,
    PRIMARY KEY (time, ticker)
);
SELECT create_hypertable('prices', 'time');

-- EPS estimates from Finnhub
CREATE TABLE eps_estimates (
    id SERIAL PRIMARY KEY,
    ticker VARCHAR(10) NOT NULL,
    period_end DATE,
    eps_estimate DECIMAL,
    eps_actual DECIMAL,
    surprise DECIMAL,
    source VARCHAR(50) DEFAULT 'finnhub',
    fetched_at TIMESTAMPTZ DEFAULT NOW()
);

-- Insider transactions (Form 4)
CREATE TABLE insider_transactions (
    id SERIAL PRIMARY KEY,
    ticker VARCHAR(10) NOT NULL,
    filing_date DATE,
    insider_name VARCHAR(255),
    title VARCHAR(100),
    transaction_type VARCHAR(10), -- P=Purchase, S=Sale
    shares DECIMAL,
    price DECIMAL,
    source VARCHAR(50) DEFAULT 'sec_edgar',
    fetched_at TIMESTAMPTZ DEFAULT NOW()
);
```

### S1-D6: Finnhub Collector

```python
# collectors/finnhub.py
import finnhub
from celery import shared_task

@shared_task
def fetch_eps_estimates(ticker: str):
    """Fetch EPS estimates from Finnhub API"""
    client = finnhub.Client(api_key=FINNHUB_API_KEY)
    estimates = client.company_eps_estimates(ticker)
    # Store in database
    ...

@shared_task
def fetch_analyst_ratings(ticker: str):
    """Fetch analyst ratings from Finnhub"""
    ...
```

### S1-D7: Price Collector

```python
# collectors/prices.py
import yfinance as yf
from celery import shared_task

@shared_task
def fetch_daily_prices(ticker: str):
    """Fetch daily OHLCV from Yahoo Finance"""
    stock = yf.Ticker(ticker)
    hist = stock.history(period="1d")
    # Store in timescaledb
    ...
```

---

## Golden Path Tests

### Automated Tests

```python
# tests/test_sprint1.py

def test_postgres_connection():
    """GP-S1-01: PostgreSQL accepts connections"""
    conn = psycopg2.connect(DATABASE_URL)
    assert conn.closed == 0
    conn.close()

def test_timescale_hypertable():
    """GP-S1-02: TimescaleDB hypertable exists"""
    result = execute("SELECT hypertable_name FROM timescaledb_information.hypertables")
    assert 'prices' in [r[0] for r in result]

def test_redis_ping():
    """GP-S1-03: Redis responds to ping"""
    r = redis.Redis.from_url(REDIS_URL)
    assert r.ping() == True

def test_celery_worker_running():
    """GP-S1-04: Celery worker is active"""
    inspect = app.control.inspect()
    assert inspect.ping() is not None

def test_finnhub_collector():
    """GP-S1-05: Finnhub collector fetches data"""
    result = fetch_eps_estimates.delay("AAPL")
    assert result.get(timeout=30)['status'] == 'success'

def test_price_collector():
    """GP-S1-06: Price collector stores data"""
    result = fetch_daily_prices.delay("AAPL")
    assert result.get(timeout=30)['status'] == 'success'
    
def test_data_stored():
    """GP-S1-07: Data exists in database"""
    prices = execute("SELECT COUNT(*) FROM prices WHERE ticker = 'AAPL'")
    assert prices[0][0] > 0
```

### Manual Checklist

- [ ] `docker compose up -d` starts all services without errors
- [ ] `docker compose ps` shows all containers healthy
- [ ] Can connect to PostgreSQL via `psql`
- [ ] Can see Celery tasks in logs
- [ ] `docker compose logs data-collector` shows successful fetches
- [ ] Query `SELECT * FROM prices LIMIT 10` returns data
- [ ] Query `SELECT * FROM eps_estimates LIMIT 10` returns data

---

## Schedule

| Day | Focus |
|-----|-------|
| Day 1 | Docker Compose + PostgreSQL setup |
| Day 2 | TimescaleDB + Redis + Celery |
| Day 3 | Database schema + migrations |
| Day 4 | Finnhub collector implementation |
| Day 5 | Price collector implementation |
| Day 6 | Integration testing |
| Day 7 | Bug fixes + documentation |

---

## Dependencies

| Dependency | Version | Purpose |
|------------|---------|---------|
| Python | 3.11+ | Runtime |
| PostgreSQL | 15 | Database |
| TimescaleDB | latest | Time-series |
| Redis | 7 | Queue |
| Celery | 5.3+ | Task scheduler |
| finnhub-python | 2.4+ | API client |
| yfinance | 0.2+ | Price data |

---

## Exit Criteria

Sprint 1 is **COMPLETE** when:
- ✅ All 7 automated tests pass
- ✅ Manual checklist 100% verified
- ✅ Data is being collected on schedule
- ✅ No critical bugs open
