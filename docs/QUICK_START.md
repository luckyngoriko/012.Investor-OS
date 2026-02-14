# Investor OS - Quick Start Guide

**Get up and running in 5 minutes**

---

## ⚡ 5-Minute Setup

### Step 1: Clone & Build (2 min)

```bash
git clone https://github.com/yourorg/investor-os.git
cd investor-os
cargo build --release
```

### Step 2: Start Database (1 min)

```bash
# Using Docker
docker run -d -p 5432:5432 -e POSTGRES_PASSWORD=postgres postgres:16
docker run -d -p 6379:6379 redis:7

# Or use existing PostgreSQL/Redis
```

### Step 3: Configure (1 min)

```bash
cp .env.example .env
# Edit .env with your settings
```

### Step 4: Run Migrations (30 sec)

```bash
cargo run --bin migrate
```

### Step 5: Start Server (30 sec)

```bash
cargo run --release
```

✅ **Server running at http://localhost:8080**

---

## 🎯 First API Call

### Check Health

```bash
curl http://localhost:8080/api/health
```

**Expected Response**:
```json
{
  "status": "healthy",
  "version": "3.0.0"
}
```

### Get AI Trading Signal

```bash
curl -X POST http://localhost:8080/api/v1/hrm/infer \
  -H "Content-Type: application/json" \
  -d '{
    "pegy": 0.85,
    "insider_sentiment": 0.7,
    "social_sentiment": 0.6,
    "vix": 15.0
  }'
```

**Expected Response**:
```json
{
  "conviction": 0.82,
  "confidence": 0.91,
  "recommended_action": "buy"
}
```

---

## 🐳 Docker Setup (Even Faster)

```bash
# One command to start everything
docker-compose up -d

# Check status
docker-compose ps

# View logs
docker-compose logs -f
```

✅ **All services running!**

---

## 📁 Project Structure

```
investor-os/
├── src/
│   ├── hrm/           # AI Trading Engine
│   ├── broker/        # Broker integrations
│   ├── treasury/      # Crypto custody
│   ├── risk/          # Risk management
│   ├── api/           # REST API
│   └── ...
├── frontend/          # Next.js web app
├── docs/              # Documentation
├── k8s/               # Kubernetes manifests
├── tests/             # Integration tests
└── scripts/           # Helper scripts
```

---

## 🧪 Running Tests

```bash
# Unit tests
cargo test --lib

# Integration tests
cargo test --test integration

# All tests
cargo test
```

---

## 🔧 Common Commands

```bash
# Build release binary
cargo build --release

# Run with logging
RUST_LOG=info cargo run

# Format code
cargo fmt

# Run linter
cargo clippy

# Check without building
cargo check
```

---

## 🚀 Deploy to Kubernetes

```bash
# Create namespace
kubectl create ns investor-os

# Apply manifests
kubectl apply -f k8s/

# Check deployment
kubectl get pods -n investor-os

# Get service URL
kubectl get svc -n investor-os
```

---

## 📚 Next Steps

1. **Read Full Docs**: See `docs/PRODUCT_DOCUMENTATION.md`
2. **API Reference**: Check `docs/API.md`
3. **Architecture**: Read `docs/ARCHITECTURE.md`
4. **Contributing**: See `CONTRIBUTING.md`

---

## ❓ Need Help?

- **Documentation**: https://docs.investor-os.com
- **GitHub Issues**: https://github.com/yourorg/investor-os/issues
- **Email**: support@investor-os.com

---

**Happy Trading!** 🚀
