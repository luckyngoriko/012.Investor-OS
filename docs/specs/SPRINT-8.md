# Sprint 8: Production Hardening & DevOps

> **Duration:** Week 15-16
> **Goal:** Kubernetes deployment, CI/CD, security hardening, production readiness
> **Ref:** [SPEC-v1.0](./SPEC-v1.0.md) | [Golden Path](./GOLDEN-PATH.md) | [ROADMAP](../ROADMAP.md)

---

## Scope

### ✅ In Scope
- Kubernetes deployment manifests
- GitHub Actions CI/CD pipeline
- Secrets management (Vault integration)
- API rate limiting and DDoS protection
- Database migration automation
- Health checks and graceful shutdown
- Multi-environment setup (dev/staging/prod)
- Disaster recovery procedures

### ❌ Out of Scope
- Multi-region deployment
- Auto-scaling based on ML predictions
- Service mesh (Istio/Linkerd)
- SOC 2 audit completion

---

## Deliverables

| ID | Deliverable | Acceptance Criteria |
|----|-------------|---------------------|
| S8-D1 | Kubernetes manifests | Deployments, Services, ConfigMaps, Secrets |
| S8-D2 | CI/CD pipeline | GitHub Actions: test, build, deploy |
| S8-D3 | Secrets management | Vault integration, secret rotation |
| S8-D4 | Rate limiting | Redis-based rate limiting, 100 req/min |
| S8-D5 | Health checks | Liveness, readiness, graceful shutdown |
| S8-D6 | Migration automation | sqlx migrate in CI/CD |
| S8-D8 | Monitoring stack | Prometheus + Alertmanager + Grafana |
| S8-D9 | Disaster recovery | Backup/restore tested, RTO < 1h |

---

## Technical Implementation

### S8-D1: Kubernetes Manifests

```yaml
# k8s/namespace.yaml
apiVersion: v1
kind: Namespace
metadata:
  name: investor-os
  labels:
    app.kubernetes.io/name: investor-os
    app.kubernetes.io/version: "1.0"

---
# k8s/postgres.yaml
apiVersion: apps/v1
kind: StatefulSet
metadata:
  name: postgres
  namespace: investor-os
spec:
  serviceName: postgres
  replicas: 1
  selector:
    matchLabels:
      app: postgres
  template:
    metadata:
      labels:
        app: postgres
    spec:
      containers:
      - name: postgres
        image: timescale/timescaledb:latest-pg15
        ports:
        - containerPort: 5432
        env:
        - name: POSTGRES_USER
          valueFrom:
            secretKeyRef:
              name: postgres-secret
              key: username
        - name: POSTGRES_PASSWORD
          valueFrom:
            secretKeyRef:
              name: postgres-secret
              key: password
        volumeMounts:
        - name: postgres-storage
          mountPath: /var/lib/postgresql/data
  volumeClaimTemplates:
  - metadata:
      name: postgres-storage
    spec:
      accessModes: ["ReadWriteOnce"]
      resources:
        requests:
          storage: 50Gi

---
# k8s/api-deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: investor-api
  namespace: investor-os
spec:
  replicas: 3
  strategy:
    type: RollingUpdate
    rollingUpdate:
      maxSurge: 1
      maxUnavailable: 0
  selector:
    matchLabels:
      app: investor-api
  template:
    metadata:
      labels:
        app: investor-api
    spec:
      containers:
      - name: api
        image: ghcr.io/neurocod/investor-os-api:latest
        ports:
        - containerPort: 3000
        env:
        - name: DATABASE_URL
          valueFrom:
            secretKeyRef:
              name: api-secret
              key: database-url
        - name: RUST_LOG
          value: "info"
        resources:
          requests:
            memory: "512Mi"
            cpu: "500m"
          limits:
            memory: "2Gi"
            cpu: "2000m"
        livenessProbe:
          httpGet:
            path: /api/health
            port: 3000
          initialDelaySeconds: 30
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /api/ready
            port: 3000
          initialDelaySeconds: 5
          periodSeconds: 5
        lifecycle:
          preStop:
            exec:
              command: ["/bin/sh", "-c", "sleep 15"]

---
# k8s/api-service.yaml
apiVersion: v1
kind: Service
metadata:
  name: investor-api
  namespace: investor-os
spec:
  selector:
    app: investor-api
  ports:
  - port: 80
    targetPort: 3000
  type: ClusterIP

---
# k8s/ingress.yaml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: investor-os-ingress
  namespace: investor-os
  annotations:
    nginx.ingress.kubernetes.io/ssl-redirect: "true"
    nginx.ingress.kubernetes.io/rate-limit: "100"
spec:
  tls:
  - hosts:
    - api.investor-os.io
    secretName: investor-os-tls
  rules:
  - host: api.investor-os.io
    http:
      paths:
      - path: /
        pathType: Prefix
        backend:
          service:
            name: investor-api
            port:
              number: 80
```

### S8-D2: GitHub Actions CI/CD

```yaml
# .github/workflows/ci.yml
name: CI

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main]

jobs:
  test:
    runs-on: ubuntu-latest
    services:
      postgres:
        image: timescale/timescaledb:latest-pg15
        env:
          POSTGRES_PASSWORD: postgres
        options: >-
          --health-cmd pg_isready
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5
        ports:
        - 5432:5432
      redis:
        image: redis:7-alpine
        ports:
        - 6379:6379
    steps:
    - uses: actions/checkout@v4
    
    - name: Install Rust
      uses: dtolnay/rust-action@stable
    
    - name: Cache cargo
      uses: actions/cache@v3
      with:
        path: |
          ~/.cargo/registry
          ~/.cargo/git
          target
        key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
    
    - name: Run tests
      env:
        DATABASE_URL: postgres://postgres:postgres@localhost:5432/postgres
        REDIS_URL: redis://localhost:6379
      run: cargo test -- --test-threads=1
    
    - name: Run clippy
      run: cargo clippy -- -D warnings
    
    - name: Check formatting
      run: cargo fmt -- --check

  build:
    needs: test
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v4
    
    - name: Set up Docker Buildx
      uses: docker/setup-buildx-action@v3
    
    - name: Log in to GitHub Container Registry
      uses: docker/login-action@v3
      with:
        registry: ghcr.io
        username: ${{ github.actor }}
        password: ${{ secrets.GITHUB_TOKEN }}
    
    - name: Build and push
      uses: docker/build-push-action@v5
      with:
        context: .
        push: ${{ github.event_name != 'pull_request' }}
        tags: |
          ghcr.io/${{ github.repository }}/api:${{ github.sha }}
          ghcr.io/${{ github.repository }}/api:latest
        cache-from: type=gha
        cache-to: type=gha,mode=max

  e2e:
    needs: build
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v4
    
    - name: Set up Node
      uses: actions/setup-node@v4
      with:
        node-version: '20'
    
    - name: Install dependencies
      run: |
        cd frontend/investor-dashboard
        npm ci
    
    - name: Install Playwright
      run: |
        cd frontend/investor-dashboard
        npx playwright install --with-deps
    
    - name: Run E2E tests
      run: |
        cd frontend/investor-dashboard
        npm run test:e2e

---
# .github/workflows/deploy-staging.yml
name: Deploy to Staging

on:
  push:
    branches: [develop]

jobs:
  deploy:
    runs-on: ubuntu-latest
    environment: staging
    steps:
    - uses: actions/checkout@v4
    
    - name: Set up kubectl
      uses: azure/setup-kubectl@v3
    
    - name: Configure kubectl
      run: |
        echo "${{ secrets.KUBE_CONFIG_STAGING }}" | base64 -d > ~/.kube/config
    
    - name: Deploy to staging
      run: |
        kubectl set image deployment/investor-api \
          api=ghcr.io/${{ github.repository }}/api:${{ github.sha }} \
          -n investor-os-staging
        kubectl rollout status deployment/investor-api -n investor-os-staging

---
# .github/workflows/deploy-production.yml
name: Deploy to Production

on:
  release:
    types: [published]

jobs:
  deploy:
    runs-on: ubuntu-latest
    environment: production
    steps:
    - uses: actions/checkout@v4
    
    - name: Set up kubectl
      uses: azure/setup-kubectl@v3
    
    - name: Configure kubectl
      run: |
        echo "${{ secrets.KUBE_CONFIG_PRODUCTION }}" | base64 -d > ~/.kube/config
    
    - name: Run database migrations
      run: |
        kubectl run migrate-${{ github.run_id }} \
          --image=ghcr.io/${{ github.repository }}/api:${{ github.sha }} \
          --rm -i --restart=Never \
          --env="DATABASE_URL=${{ secrets.DATABASE_URL }}" \
          -- ./investor-api migrate
    
    - name: Deploy to production
      run: |
        kubectl set image deployment/investor-api \
          api=ghcr.io/${{ github.repository }}/api:${{ github.event.release.tag_name }} \
          -n investor-os
        kubectl rollout status deployment/investor-api -n investor-os
```

### S8-D3: Secrets Management

```rust
// crates/investor-core/src/secrets.rs
use hashicorp_vault::Client;

pub struct SecretManager {
    vault: Client,
}

impl SecretManager {
    pub async fn new(config: &VaultConfig) -> Result<Self> {
        let vault = Client::new(
            &config.address,
            &config.token,
        )?;
        
        Ok(Self { vault })
    }
    
    pub async fn get_database_url(&self) -> Result<String> {
        let secret: Secret = self.vault
            .get_secret("investor-os/data/database")
            .await?;
        
        Ok(secret.data.url)
    }
    
    pub async fn rotate_database_password(&self) -> Result<String> {
        // Generate new password
        let new_password = generate_secure_password(32);
        
        // Update in Vault
        self.vault.set_secret("investor-os/data/database", 
            json!({ "password": new_password })
        ).await?;
        
        // Update in database (requires admin connection)
        self.update_db_password(&new_password).await?;
        
        // Rolling restart of API pods
        self.restart_api_pods().await?;
        
        Ok(new_password)
    }
}
```

### S8-D4: Rate Limiting

```rust
// crates/investor-api/src/middleware/rate_limit.rs
use redis::AsyncCommands;

pub struct RateLimiter {
    redis: redis::aio::MultiplexedConnection,
    max_requests: u32,
    window_secs: u64,
}

impl RateLimiter {
    pub async fn check_rate_limit(&self, key: &str) -> Result<RateLimitStatus> {
        let mut conn = self.redis.clone();
        let window_key = format!("rate_limit:{}", key);
        
        let current: u32 = conn.get(&window_key).await.unwrap_or(0);
        
        if current >= self.max_requests {
            let ttl: i64 = conn.ttl(&window_key).await?;
            return Ok(RateLimitStatus::Exceeded { retry_after: ttl });
        }
        
        // Increment counter
        let _: () = conn.incr(&window_key, 1).await?;
        let _: () = conn.expire(&window_key, self.window_secs as i64).await?;
        
        let remaining = self.max_requests - current - 1;
        
        Ok(RateLimitStatus::Allowed { remaining })
    }
}

// Middleware implementation
pub async fn rate_limit_middleware<B>(
    State(limiter): State<Arc<RateLimiter>>,
    req: Request<B>,
    next: Next<B>,
) -> Result<Response, StatusCode> {
    // Get client IP or API key
    let key = extract_client_key(&req);
    
    match limiter.check_rate_limit(&key).await {
        Ok(RateLimitStatus::Allowed { remaining }) => {
            let mut response = next.run(req).await;
            response.headers_mut().insert(
                "X-RateLimit-Remaining",
                remaining.to_string().parse().unwrap(),
            );
            Ok(response)
        }
        Ok(RateLimitStatus::Exceeded { retry_after }) => {
            let mut response = Response::builder()
                .status(StatusCode::TOO_MANY_REQUESTS)
                .body(Body::empty())
                .unwrap();
            response.headers_mut().insert(
                "Retry-After",
                retry_after.to_string().parse().unwrap(),
            );
            Ok(response)
        }
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}
```

### S8-D5: Health Checks

```rust
// crates/investor-api/src/health.rs
use std::sync::atomic::{AtomicBool, Ordering};

pub struct HealthChecker {
    shutting_down: AtomicBool,
    db_pool: PgPool,
    redis: redis::aio::MultiplexedConnection,
}

impl HealthChecker {
    /// Liveness probe - is the process running?
    pub async fn liveness(&self) -> HealthStatus {
        HealthStatus::Healthy
    }
    
    /// Readiness probe - can it serve traffic?
    pub async fn readiness(&self) -> HealthStatus {
        // Check database
        if sqlx::query("SELECT 1").fetch_one(&self.db_pool).await.is_err() {
            return HealthStatus::Unhealthy("Database unavailable".into());
        }
        
        // Check Redis
        if self.redis.ping::<String>().await.is_err() {
            return HealthStatus::Unhealthy("Redis unavailable".into());
        }
        
        // Check if shutting down
        if self.shutting_down.load(Ordering::SeqCst) {
            return HealthStatus::Unhealthy("Shutting down".into());
        }
        
        HealthStatus::Healthy
    }
    
    /// Graceful shutdown
    pub async fn shutdown(&self) {
        info!("Starting graceful shutdown...");
        
        // Signal readiness to fail
        self.shutting_down.store(true, Ordering::SeqCst);
        
        // Wait for in-flight requests to complete
        tokio::time::sleep(Duration::from_secs(15)).await;
        
        // Close database connections
        self.db_pool.close().await;
        
        info!("Shutdown complete");
    }
}

// Signal handlers
pub fn setup_signal_handlers(shutdown_tx: mpsc::Sender<()>) {
    tokio::spawn(async move {
        let mut sigterm = signal(SignalKind::terminate()).unwrap();
        let mut sigint = signal(SignalKind::interrupt()).unwrap();
        
        tokio::select! {
            _ = sigterm.recv() => info!("Received SIGTERM"),
            _ = sigint.recv() => info!("Received SIGINT"),
        }
        
        let _ = shutdown_tx.send(()).await;
    });
}
```

### S8-D6: Database Migration Automation

```rust
// crates/investor-api/src/migrate.rs
use sqlx::migrate::Migrator;

pub static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

pub async fn run_migrations(pool: &PgPool) -> Result<()> {
    info!("Running database migrations...");
    
    MIGRATOR.run(pool).await?;
    
    info!("Migrations complete");
    Ok(())
}

// Command-line interface
#[derive(Parser)]
enum Cli {
    Serve,
    Migrate,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    match cli {
        Cli::Serve => serve().await,
        Cli::Migrate => {
            let pool = create_pool().await?;
            run_migrations(&pool).await
        }
    }
}
```

### S8-D8: Monitoring Stack

```yaml
# k8s/prometheus.yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: prometheus-config
  namespace: investor-os
data:
  prometheus.yml: |
    global:
      scrape_interval: 15s
    
    alerting:
      alertmanagers:
      - static_configs:
        - targets: ['alertmanager:9093']
    
    rule_files:
      - /etc/prometheus/rules/*.yml
    
    scrape_configs:
    - job_name: 'kubernetes-pods'
      kubernetes_sd_configs:
      - role: pod
        namespaces:
          names:
          - investor-os
      relabel_configs:
      - source_labels: [__meta_kubernetes_pod_annotation_prometheus_io_scrape]
        action: keep
        regex: true

---
# k8s/alertmanager.yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: alertmanager-config
  namespace: investor-os
data:
  alertmanager.yml: |
    global:
      slack_api_url: '${SLACK_WEBHOOK_URL}'
    
    route:
      receiver: 'slack'
      routes:
      - match:
          severity: critical
        receiver: 'pagerduty'
        continue: true
    
    receivers:
    - name: 'slack'
      slack_configs:
      - channel: '#alerts'
        title: 'Investor OS Alert'
    
    - name: 'pagerduty'
      pagerduty_configs:
      - service_key: '${PAGERDUTY_KEY}'
```

### S8-D9: Disaster Recovery

```bash
#!/bin/bash
# scripts/backup.sh

set -e

TIMESTAMP=$(date +%Y%m%d_%H%M%S)
BACKUP_BUCKET="gs://investor-os-backups"

# Database backup
echo "Backing up database..."
kubectl exec -n investor-os postgres-0 -- \
    pg_dump -U investor investor_os | \
    gzip > /tmp/db_backup_${TIMESTAMP}.sql.gz

# Upload to GCS
gsutil cp /tmp/db_backup_${TIMESTAMP}.sql.gz \
    ${BACKUP_BUCKET}/database/

# Redis backup
echo "Backing up Redis..."
kubectl exec -n investor-os redis-0 -- \
    redis-cli BGSAVE

# Copy RDB file
kubectl cp investor-os/redis-0:/data/dump.rdb \
    /tmp/redis_backup_${TIMESTAMP}.rdb

gsutil cp /tmp/redis_backup_${TIMESTAMP}.rdb \
    ${BACKUP_BUCKET}/redis/

# Cleanup
rm /tmp/*_backup_${TIMESTAMP}*

echo "Backup complete: ${TIMESTAMP}"
```

```bash
#!/bin/bash
# scripts/restore.sh

set -e

BACKUP_FILE=$1

if [ -z "$BACKUP_FILE" ]; then
    echo "Usage: $0 <backup_file>"
    exit 1
fi

echo "Restoring from ${BACKUP_FILE}..."

# Download from GCS
gsutil cp gs://investor-os-backups/database/${BACKUP_FILE} /tmp/

# Restore database
gunzip -c /tmp/${BACKUP_FILE} | \
    kubectl exec -i -n investor-os postgres-0 -- \
    psql -U investor -d investor_os

# Rolling restart
kubectl rollout restart deployment/investor-api -n investor-os

echo "Restore complete"
```

---

## Golden Path Tests

### S8-GP-01: Kubernetes Deployment
```bash
#!/bin/bash
# tests/k8s-deployment.sh

set -e

# Apply manifests
kubectl apply -f k8s/

# Wait for rollout
kubectl rollout status deployment/investor-api -n investor-os

# Test health endpoints
kubectl port-forward svc/investor-api 8080:80 -n investor-os &
sleep 5

curl -sf http://localhost:8080/api/health || exit 1
curl -sf http://localhost:8080/api/ready || exit 1

echo "K8s deployment test passed"
```

### S8-GP-02: CI/CD Pipeline
```yaml
# Test pipeline with act (local GitHub Actions)
name: Test CI
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v4
    - run: cargo test
```

### S8-GP-03: Rate Limiting
```rust
#[tokio::test]
async fn test_rate_limiting() {
    let limiter = RateLimiter::new(test_redis()).await;
    
    // Make 100 requests (limit)
    for i in 0..100 {
        let status = limiter.check_rate_limit("test-client").await.unwrap();
        assert!(matches!(status, RateLimitStatus::Allowed { .. }));
    }
    
    // 101st request should be blocked
    let status = limiter.check_rate_limit("test-client").await.unwrap();
    assert!(matches!(status, RateLimitStatus::Exceeded { .. }));
}
```

### S8-GP-04: Graceful Shutdown
```rust
#[tokio::test]
async fn test_graceful_shutdown() {
    let (tx, mut rx) = mpsc::channel(1);
    let health = Arc::new(HealthChecker::new(test_pool()));
    
    // Start server
    let server_health = health.clone();
    tokio::spawn(async move {
        serve_with_shutdown(server_health, rx.recv()).await;
    });
    
    // Check it's healthy
    assert!(health.readiness().await.is_healthy());
    
    // Trigger shutdown
    tx.send(()).await.unwrap();
    
    // Give it time to shut down
    sleep(Duration::from_secs(2)).await;
    
    // Should be unhealthy now
    assert!(!health.readiness().await.is_healthy());
}
```

### S8-GP-05: Database Migration
```rust
#[tokio::test]
async fn test_migration() {
    let pool = create_test_pool().await;
    
    // Run migrations
    run_migrations(&pool).await.unwrap();
    
    // Verify schema
    let tables: Vec<String> = sqlx::query_scalar(
        "SELECT table_name FROM information_schema.tables WHERE table_schema = 'public'"
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    
    assert!(tables.contains(&"prices".to_string()));
    assert!(tables.contains(&"signals".to_string()));
}
```

### S8-GP-06: Backup and Restore
```bash
#!/bin/bash
# Test backup/restore

# Create backup
./scripts/backup.sh

# Get latest backup
LATEST=$(gsutil ls gs://investor-os-backups/database/ | tail -1)

# Restore
./scripts/restore.sh $(basename $LATEST)

# Verify data
kubectl exec -n investor-os postgres-0 -- \
    psql -U investor -c "SELECT COUNT(*) FROM prices;"

echo "Backup/restore test passed"
```

---

## Schedule

| Day | Focus |
|-----|-------|
| Day 1 | Kubernetes manifests, local testing |
| Day 2 | GitHub Actions CI/CD pipeline |
| Day 3 | Secrets management, Vault setup |
| Day 4 | Rate limiting implementation |
| Day 5 | Health checks, graceful shutdown |
| Day 6 | Database migration automation |
| Day 7 | Monitoring stack, Alertmanager |
| Day 8 | Disaster recovery, backup/restore tests |

---

## Exit Criteria

Sprint 8 is **COMPLETE** when:
- ✅ All 6 Golden Path tests pass
- ✅ Deploys successfully to Kubernetes
- ✅ CI/CD pipeline runs without errors
- ✅ Rate limiting blocks excessive requests
- ✅ Graceful shutdown completes in < 30s
- ✅ Database migrations run automatically
- ✅ Backup/restore tested and RTO < 1h verified
- ✅ Monitoring alerts reach Slack/PagerDuty
