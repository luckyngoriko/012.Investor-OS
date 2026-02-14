# Investor OS v3.0 - Product Documentation

**Version**: 3.0.0  
**Status**: Production Ready  
**Date**: 2026-02-12  
**License**: Commercial / MIT (Core)

---

## 📚 Table of Contents

1. [Product Overview](#product-overview)
2. [Architecture](#architecture)
3. [Core Features](#core-features)
4. [AI/ML Engine (HRM)](#aiml-engine-hrm)
5. [Security](#security)
6. [Treasury & Custody](#treasury--custody)
7. [Risk Management](#risk-management)
8. [Performance & Monitoring](#performance--monitoring)
9. [Setup Guide](#setup-guide)
10. [Usage Guide](#usage-guide)
11. [API Reference](#api-reference)
12. [Deployment](#deployment)

---

## Product Overview

### What is Investor OS?

Investor OS is an **autonomous AI trading system** that combines hierarchical reasoning, real-time market analysis, and institutional-grade infrastructure to make intelligent trading decisions.

### Key Differentiators

1. **Native Rust ML** - No Python dependency, maximum performance
2. **Hierarchical Reasoning** - Multi-factor conviction calculation
3. **Institutional Security** - HSM-backed, 2FA, audit trails
4. **Real Fireblocks Integration** - Real crypto custody (not simulation)
5. **Kubernetes Native** - Auto-scaling, cloud-ready

### Target Users

| User Type | Use Case | Package |
|-----------|----------|---------|
| Prop Trading Firms | AI alpha generation | Core + Analytics |
| Crypto Hedge Funds | Quantitative strategies | Core + Fireblocks |
| Family Offices | Wealth preservation | Core + Risk Mgmt |
| Individual Traders | Personal trading | SaaS |

---

## Architecture

### High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         CLIENT LAYER                             │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐          │
│  │   Web App    │  │  Mobile App  │  │   API Keys   │          │
│  │   (Next.js)  │  │   (Future)   │  │   (Algo)     │          │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘          │
└─────────┼─────────────────┼─────────────────┼────────────────────┘
          │                 │                 │
          ▼                 ▼                 ▼
┌─────────────────────────────────────────────────────────────────┐
│                    AI-OS-PG (Security Layer)                     │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  • WAF - Web Application Firewall                        │   │
│  │  • DLP - Data Loss Prevention                            │   │
│  │  • Rate Limiting - Request throttling                    │   │
│  │  • Policy Engine - Business rules                        │   │
│  └─────────────────────────────────────────────────────────┘   │
└──────────────────────────────┬──────────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────────┐
│                  AI-OS.NET (Compliance Layer)                    │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  • EU AI Act Compliance                                  │   │
│  │  • GDPR Management                                       │   │
│  │  • Audit Logging                                         │   │
│  │  • Compliance Score Tracking                             │   │
│  └─────────────────────────────────────────────────────────┘   │
└──────────────────────────────┬──────────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────────┐
│                    INVESTOR OS CORE                              │
│                                                                  │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐             │
│  │   HRM AI    │  │    Risk     │  │  Treasury   │             │
│  │   Engine    │  │   Manager   │  │   Module    │             │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘             │
│         │                │                │                     │
│         └────────────────┼────────────────┘                     │
│                          ▼                                       │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │              Decision Engine & Execution                 │   │
│  └─────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
          │
          ▼
┌─────────────────────────────────────────────────────────────────┐
│                      INFRASTRUCTURE                              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐          │
│  │  PostgreSQL  │  │    Redis     │  │   Fireblocks │          │
│  │  (Primary)   │  │   (Cache)    │  │   (Custody)  │          │
│  └──────────────┘  └──────────────┘  └──────────────┘          │
└─────────────────────────────────────────────────────────────────┘
```

### Technology Stack

| Component | Technology | Purpose |
|-----------|------------|---------|
| Language | Rust | Performance, safety |
| Web Framework | Axum | HTTP API |
| Database | PostgreSQL | Primary storage |
| Cache | Redis | Session/cache |
| ML Framework | Burn | Native Rust ML |
| Frontend | Next.js 15 | React web app |
| GPU | CUDA/ROCm/Intel | ML acceleration |
| Messaging | WebSocket | Real-time data |
| Deployment | Kubernetes | Orchestration |

---

## Core Features

### 1. Hierarchical Reasoning Model (HRM)

**What it is**: Multi-factor AI engine that calculates "conviction scores" for trading decisions.

**How it works**:
```
Input Signals:
├── Fundamental (PEGY ratio)
├── Insider Sentiment
├── Social Media Sentiment
├── VIX Volatility
└── Market Regime

Processing:
├── LSTM Network (temporal patterns)
├── Attention Mechanism (focus on important factors)
└── Ensemble Voting (combine predictions)

Output:
└── Conviction Score (0-100%)
```

**Why this architecture**:
- Single-factor models fail in changing markets
- Hierarchical approach mimics human analyst workflow
- Ensemble reduces overfitting
- LSTM captures temporal dependencies

**Usage**:
```rust
// Get AI trading signal
let input = HrmInput {
    pegy: 0.85,           // Price/Earnings to Growth Yield
    insider_sentiment: 0.7,
    social_sentiment: 0.6,
    vix: 15.0,
    regime: MarketRegime::Trending,
};

let result = hrm.infer(input)?;
println!("Conviction: {}%", result.conviction * 100.0);
println!("Recommended Action: {:?}", result.action);
```

---

### 2. Market Regime Detection

**What it is**: AI system that classifies current market conditions.

**Regimes Detected**:
| Regime | Description | Strategy |
|--------|-------------|----------|
| Trending | Strong directional move | Momentum |
| Ranging | Sideways movement | Mean Reversion |
| Volatile | High volatility | Breakout |
| Crisis | Market panic | Defensive |
| Recovery | Post-crash rebound | Aggressive |

**How it works**:
```
Indicators Analyzed:
├── Price vs Moving Averages
├── RSI (Relative Strength Index)
├── MACD (Trend momentum)
├── Bollinger Bands (Volatility)
├── Volume Patterns
└── Correlation Breakdown

ML Classification:
├── Random Forest (ensemble)
├── Confidence Score
└── Regime Transition Probability
```

**Why it matters**: Different strategies work in different regimes. Trend-following fails in ranging markets.

---

### 3. Risk Management

#### Position Sizing (Kelly Criterion)

**What it is**: Mathematical formula for optimal bet sizing.

**Formula**:
```
f = (bp - q) / b

Where:
  f = fraction of capital to bet
  b = odds received (win/loss ratio)
  p = probability of win
  q = probability of loss (1-p)
```

**Implementation**:
```rust
let kelly = KellyCriterion::new()
    .with_win_rate(0.55)
    .with_win_loss_ratio(2.0);

let position_size = kelly.calculate(
    account_balance,
    current_price,
    stop_loss_price
)?;
```

**Why Kelly**: Maximizes long-term growth while minimizing ruin risk.

---

#### Drawdown Protection

**What it is**: Automatic trading halt when losses exceed threshold.

**How it works**:
```
Daily Loss Limit: 2% of portfolio
Weekly Loss Limit: 5% of portfolio
Monthly Loss Limit: 10% of portfolio

Actions on breach:
1. Stop new positions
2. Close risky positions
3. Alert risk manager
4. Require manual override to resume
```

---

### 4. Treasury Module

#### Paper Trading (Demo)

**What it is**: Simulated trading with fake money.

**Features**:
- Real market data
- Virtual balances
- P&L tracking
- Strategy testing

**Usage**:
```rust
let broker = PaperBroker::new(BrokerConfig {
    paper_trading: true,
    initial_balance: Decimal::from(100_000),
    ..Default::default()
});

// Place order
let order = Order::market_buy("BTC", Decimal::from(1));
let fill = broker.place_order(order).await?;
```

---

#### Fireblocks Integration (Real)

**What it is**: Institutional-grade crypto custody.

**Features**:
- Multi-signature wallets
- MPC (Multi-Party Computation) security
- Policy-based transactions
- Compliance reporting

**How it works**:
```
Deposit Flow:
1. Request deposit address from Fireblocks
2. Display to user
3. Monitor blockchain
4. Credit balance on confirmation

Withdrawal Flow:
1. User requests withdrawal
2. 2FA verification
3. Policy check (whitelist, limits)
4. Fireblocks creates transaction
5. Multi-signature signing
6. Broadcast to blockchain
7. Update balance
```

**Why Fireblocks**: Used by Coinbase, Binance, Fidelity. Insurance up to $30M.

---

### 5. Security Features

#### Authentication

**API Key Authentication**:
```
Header: X-API-Key: <key>
Format: HMAC-SHA256 signed requests
Rotation: Automatic every 90 days
```

**2FA Methods**:
1. TOTP (Google Authenticator)
2. HOTP (YubiKey)
3. WebAuthn (Fingerprint/Face ID)
4. SMS (fallback)
5. Email (fallback)

#### Clearance Levels

| Level | Access | Use Case |
|-------|--------|----------|
| Public | Read-only market data | Public website |
| Internal | Standard trading | Regular users |
| Confidential | Advanced features | Premium users |
| Restricted | Admin functions | Managers |
| TopSecret | System config | Admins only |

---

### 6. Performance & Monitoring

#### Prometheus Metrics

**Collected Metrics**:
```
# Trading
hrm_inference_total
hrm_inference_duration_seconds
hrm_conviction_score
trades_executed_total
trade_slippage_bps

# System
http_requests_total
http_request_duration_seconds
active_connections
memory_usage_bytes
cpu_usage_percent

# Risk
drawdown_percent
daily_pnl_usd
position_size_usd
margin_utilization
```

#### Grafana Dashboards

**Pre-built Dashboards**:
1. **Trading Overview** - P&L, positions, orders
2. **HRM Performance** - Inference stats, conviction scores
3. **Risk Monitor** - Drawdown, exposure, limits
4. **System Health** - CPU, memory, latency
5. **Security Audit** - Login attempts, API usage

---

### 7. Tax & Compliance

#### Tax Loss Harvesting

**What it is**: Automatic selling of losing positions to offset gains.

**How it works**:
```
1. Monitor unrealized losses
2. Identify harvestable lots (> $100 loss)
3. Check wash sale rules (30 days)
4. Suggest replacement securities
5. Execute harvest trades
6. Generate tax reports
```

#### Wash Sale Monitor

**Rule**: Can't claim loss if repurchasing within 30 days.

**Implementation**:
- Tracks all sales
- Flags potential wash sales
- Suggests alternative ETFs
- Blocks non-compliant trades (optional)

---

## Setup Guide

### Prerequisites

**Hardware**:
- CPU: 4+ cores
- RAM: 8GB+ (16GB recommended)
- Disk: 100GB SSD
- GPU: Optional (NVIDIA/AMD/Intel)

**Software**:
- Rust 1.75+
- PostgreSQL 16
- Redis 7
- Docker (optional)

### Installation

#### Option 1: From Source

```bash
# Clone repository
git clone https://github.com/yourorg/investor-os.git
cd investor-os

# Install dependencies
cargo build --release

# Setup database
./scripts/setup-db.sh

# Run migrations
cargo run --bin migrate

# Start server
cargo run --release
```

#### Option 2: Docker

```bash
# Clone repository
git clone https://github.com/yourorg/investor-os.git
cd investor-os

# Start with Docker Compose
docker-compose up -d

# Check status
docker-compose ps
```

#### Option 3: Kubernetes

```bash
# Apply manifests
kubectl apply -f k8s/

# Check deployment
kubectl get pods -n investor-os

# Get service URL
kubectl get svc -n investor-os
```

### Configuration

#### Environment Variables

```bash
# Database
DATABASE_URL=postgresql://user:pass@localhost/investor_os
REDIS_URL=redis://localhost:6379

# Security
JWT_SECRET=your-secret-key
API_KEY_SALT=your-salt

# Fireblocks (optional)
FIREBLOCKS_API_KEY=your-api-key
FIREBLOCKS_SECRET_PATH=/path/to/secret.key
FIREBLOCKS_VAULT_ID=your-vault

# ML
HRM_MODEL_PATH=/models/hrm-v1.safetensors
GPU_BACKEND=cuda  # or rocm, intel, cpu

# Trading
PAPER_TRADING=true
DEFAULT_BROKER=interactive_brokers
```

#### Config File

```toml
# config/production.toml
[server]
host = "0.0.0.0"
port = 8080
workers = 4

[database]
pool_size = 20
timeout_seconds = 30

[risk]
daily_loss_limit = 0.02  # 2%
max_position_size = 0.10  # 10% of portfolio
max_drawdown = 0.15  # 15%

[hrm]
confidence_threshold = 0.7
min_regime_confidence = 0.6
```

---

## Usage Guide

### 1. Getting API Key

```bash
# Register account
curl -X POST http://localhost:8080/api/auth/register \
  -H "Content-Type: application/json" \
  -d '{"email":"user@example.com","password":"secure123"}'

# Login
curl -X POST http://localhost:8080/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email":"user@example.com","password":"secure123"}'

# Get API key
curl http://localhost:8080/api/auth/api-keys \
  -H "Authorization: Bearer <token>"
```

### 2. Check System Health

```bash
curl http://localhost:8080/api/health
```

### 3. Get AI Trading Signal

```bash
curl http://localhost:8080/api/v1/hrm/infer \
  -H "X-API-Key: your-api-key" \
  -H "Content-Type: application/json" \
  -d '{
    "pegy": 0.85,
    "insider_sentiment": 0.7,
    "social_sentiment": 0.6,
    "vix": 15.0,
    "regime": "trending"
  }'
```

**Response**:
```json
{
  "conviction": 0.82,
  "confidence": 0.91,
  "regime": "trending",
  "recommended_action": "buy",
  "position_size": 0.15
}
```

### 4. Place Order

```bash
curl -X POST http://localhost:8080/api/broker/orders \
  -H "X-API-Key: your-api-key" \
  -H "Content-Type: application/json" \
  -d '{
    "symbol": "BTC",
    "side": "buy",
    "quantity": "1.5",
    "order_type": "market"
  }'
```

### 5. Check Portfolio

```bash
curl http://localhost:8080/api/broker/positions \
  -H "X-API-Key: your-api-key"
```

### 6. Real-time Streaming (WebSocket)

```javascript
const ws = new WebSocket('ws://localhost:8080/ws/hrm');

ws.onopen = () => {
  console.log('Connected to HRM stream');
};

ws.onmessage = (event) => {
  const data = JSON.parse(event.data);
  console.log('Conviction:', data.conviction);
  console.log('Regime:', data.regime);
};
```

---

## API Reference

### Core Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | /api/health | Health check |
| GET | /api/docs | API documentation |
| POST | /api/auth/register | Register user |
| POST | /api/auth/login | Login |
| GET | /api/auth/api-keys | List API keys |

### Trading Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | /api/broker/orders | Place order |
| DELETE | /api/broker/orders/:id | Cancel order |
| GET | /api/broker/positions | List positions |
| GET | /api/broker/account | Account info |

### HRM Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | /api/v1/hrm/infer | Get AI signal |
| POST | /api/v1/hrm/batch | Batch inference |
| GET | /api/v1/hrm/health | HRM status |

### Analytics Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | /api/analytics/backtest | Run backtest |
| GET | /api/analytics/risk | Risk metrics |
| GET | /api/analytics/attribution | Performance attribution |

---

## Deployment

### Production Checklist

- [ ] SSL certificates configured
- [ ] Database backups scheduled
- [ ] Monitoring alerts set up
- [ ] Firewall rules configured
- [ ] API keys rotated
- [ ] 2FA enabled for admin
- [ ] Paper trading tested
- [ ] Fireblocks configured (if using real custody)
- [ ] Kill switch tested
- [ ] Rollback plan documented

### Kubernetes Deployment

```yaml
# production-deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: investor-os
spec:
  replicas: 3
  selector:
    matchLabels:
      app: investor-os
  template:
    metadata:
      labels:
        app: investor-os
    spec:
      containers:
      - name: investor-os
        image: investor-os:v3.0.0
        ports:
        - containerPort: 8080
        env:
        - name: DATABASE_URL
          valueFrom:
            secretKeyRef:
              name: db-secret
              key: url
        resources:
          requests:
            memory: "512Mi"
            cpu: "500m"
          limits:
            memory: "2Gi"
            cpu: "2000m"
```

---

## Troubleshooting

### Common Issues

**Issue**: `cargo build` fails
```bash
# Solution: Update Rust
rustup update

# Clean and rebuild
cargo clean
cargo build --release
```

**Issue**: Database connection fails
```bash
# Check PostgreSQL
sudo systemctl status postgresql

# Verify connection
psql $DATABASE_URL -c "SELECT 1"
```

**Issue**: HRM model not found
```bash
# Download model
./scripts/download-models.sh

# Or train new model
cargo run --bin train-hrm
```

### Support

- **Documentation**: https://docs.investor-os.com
- **GitHub Issues**: https://github.com/yourorg/investor-os/issues
- **Email**: support@investor-os.com
- **Discord**: https://discord.gg/investor-os

---

## License

**Core System**: MIT License
**Enterprise Add-ons**: Commercial License

---

**Last Updated**: 2026-02-12  
**Version**: 3.0.0  
**Status**: Production Ready
