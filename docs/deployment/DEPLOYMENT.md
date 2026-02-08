# Investor OS - Deployment Guide

> **Version:** 1.0 | **Sprint:** 4 | **Date:** 2026-02-08

## Prerequisites

- Docker Engine 24.0+
- Docker Compose v2.0+
- 4GB RAM minimum
- 10GB disk space

## Quick Start

```bash
# Clone the repository
git clone https://github.com/neurocod/investor-os.git
cd investor-os

# Set up environment variables
cp .env.example .env
# Edit .env with your settings

# Start all services
docker compose up -d

# Check status
docker compose ps
```

## Environment Variables

Create a `.env` file with the following:

```bash
# Database
DB_USER=investor
DB_PASSWORD=your_secure_password
DB_NAME=investor_os
DB_HOST=postgres
DB_PORT=5432

# Redis
REDIS_URL=redis://redis:6379

# API
API_PORT=3000
RUST_LOG=info

# Grafana
GRAFANA_USER=admin
GRAFANA_PASSWORD=your_secure_password

# External APIs (optional for Sprint 4)
FINNHUB_API_KEY=your_key
ALPHAVANTAGE_API_KEY=your_key
```

## Services

| Service | Port | Description |
|---------|------|-------------|
| API | 3000 | REST API server |
| PostgreSQL | 5432 | Database with TimescaleDB |
| Redis | 6379 | Cache and message broker |
| Grafana | 3001 | Monitoring dashboards |
| Prometheus | 9090 | Metrics collection |

## Verification

### 1. API Health Check

```bash
curl http://localhost:3000/api/health
```

Expected response:
```json
{
  "status": "healthy",
  "timestamp": "2026-02-08T07:30:00Z"
}
```

### 2. Dashboard

Open http://localhost:3001 (Grafana) and login with credentials from `.env`.

Pre-configured dashboards:
- System Health
- Portfolio Performance  
- Data Pipeline

### 3. Frontend

Build and serve the Next.js dashboard:

```bash
cd frontend/investor-dashboard
npm install
npm run build
npm start
```

Access at http://localhost:3002

## Production Deployment

### Security Checklist

- [ ] Change all default passwords
- [ ] Enable HTTPS/WSS
- [ ] Configure firewall rules
- [ ] Set up log aggregation
- [ ] Enable database backups
- [ ] Configure monitoring alerts

### Scaling

For high availability:

```yaml
# docker-compose.override.yml
services:
  api:
    deploy:
      replicas: 3
      resources:
        limits:
          cpus: '2'
          memory: 2G
```

## Troubleshooting

### Database Connection Failed

```bash
# Check postgres logs
docker compose logs postgres

# Verify connection
docker compose exec postgres pg_isready
```

### API Won't Start

```bash
# Check API logs
docker compose logs api

# Verify environment
docker compose exec api env | grep DATABASE
```

### Grafana Dashboards Missing

```bash
# Restart grafana
docker compose restart grafana

# Verify provisioning
docker compose exec grafana ls /etc/grafana/dashboards
```

## Backup Strategy

### Database

```bash
# Create backup
docker compose exec postgres pg_dump -U investor investor_os > backup.sql

# Restore backup
docker compose exec -T postgres psql -U investor investor_os < backup.sql
```

### Grafana

Dashboards are provisioned from Git. Data sources are auto-configured.

## Updates

```bash
# Pull latest changes
git pull origin main

# Rebuild and restart
docker compose down
docker compose up -d --build
```

## Support

- Issues: https://github.com/neurocod/investor-os/issues
- Documentation: /docs
- Runbook: /docs/deployment/RUNBOOK.md
