# Sprint 35: Production Deployment & CI/CD

**Status**: Complete ✅  
**Tests**: 627 unit tests + 18 integration tests passing  
**Date**: February 2026

## Overview

Implemented production-ready CI/CD pipeline with GitHub Actions, Kubernetes manifests for multi-environment deployment, and comprehensive health checks.

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                           CI/CD Pipeline                                 │
├─────────────────────────────────────────────────────────────────────────┤
│  ┌──────────┐   ┌──────────┐   ┌──────────┐   ┌──────────┐             │
│  │   Test   │ → │ Security │ → │  Build   │ → │  Deploy  │             │
│  │  Suite   │   │  Audit   │   │   Push   │   │          │             │
│  └──────────┘   └──────────┘   └──────────┘   └──────────┘             │
│       │              │              │              │                    │
│   Unit Tests    cargo-audit     Docker Build   Dev/Stage/Prod           │
│   Integration   Trivy Scan     Multi-arch      Canary Deploy            │
│   Clippy        Hadolint       ghcr.io         Rollback                 │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                        Kubernetes Clusters                               │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐                 │
│  │ Development │    │   Staging   │    │ Production  │                 │
│  │  1 replica  │    │  2 replicas │    │  5+ replicas │                 │
│  │ Paper mode  │    │ Paper mode  │    │ Live trading │                 │
│  │ Debug logs  │    │ Info logs   │    │ Warn logs    │                 │
│  └─────────────┘    └─────────────┘    └─────────────┘                 │
│       develop            main               v* tags                      │
└─────────────────────────────────────────────────────────────────────────┘
```

## CI/CD Pipeline

### GitHub Actions Workflow (`.github/workflows/ci.yml`)

#### Stage 1: Test Suite
- **Formatting**: `cargo fmt --check`
- **Linting**: `cargo clippy --all-targets --all-features -- -D warnings`
- **Unit Tests**: `cargo test --lib` (parallel execution)
- **Integration Tests**: `cargo test --test '*'`

#### Stage 2: Security Audit
- **cargo-audit**: Check for vulnerable dependencies
- **Trivy**: Container and filesystem vulnerability scanning
- **Hadolint**: Dockerfile best practices linting

#### Stage 3: Build & Push
- **Multi-arch builds**: linux/amd64, linux/arm64
- **Cache optimization**: BuildKit with GitHub Actions cache
- **Registry**: GitHub Container Registry (ghcr.io)
- **Tagging**: Semantic versioning, SHA, branch names

#### Stage 4: Deploy
| Environment | Trigger | Replicas | Strategy |
|-------------|---------|----------|----------|
| Development | `develop` branch | 1 | Direct |
| Staging | `main` branch | 2 | Rolling update |
| Production | `v*` tags | 5+ | Canary (10% → 100%) |

## Kubernetes Configuration

### Base Resources (`k8s/base/`)

```yaml
api.yaml          # Main API deployment
configmap.yaml    # Configuration (non-sensitive)
secrets.yaml      # Secrets (encrypted)
postgres.yaml     # PostgreSQL database
redis.yaml        # Redis cache
ingress.yaml      # HTTP ingress with TLS
hpa.yaml          # Horizontal pod autoscaling
pdb.yaml          # Pod disruption budget
monitoring.yaml   # Prometheus/Grafana
```

### Environment Overlays

#### Development (`k8s/overlays/dev/`)
```yaml
replicas: 1
resources:
  requests:
    memory: "256Mi"
    cpu: "250m"
env:
  PAPER_TRADING: "true"
  RUST_LOG: "debug"
  RATE_LIMIT_ENABLED: "false"
```

#### Staging (`k8s/overlays/staging/`)
```yaml
replicas: 2
resources:
  requests:
    memory: "512Mi"
    cpu: "500m"
env:
  PAPER_TRADING: "true"
  RUST_LOG: "info"
  RATE_LIMIT_ENABLED: "true"
```

#### Production (`k8s/overlays/prod/`)
```yaml
replicas: 5
resources:
  requests:
    memory: "1Gi"
    cpu: "1000m"
  limits:
    memory: "4Gi"
    cpu: "4000m"
env:
  PAPER_TRADING: "false"
  RUST_LOG: "warn"
  RATE_LIMIT_ENABLED: "true"
  CIRCUIT_BREAKER_ENABLED: "true"
```

### Production Features

#### Canary Deployment
- Initial: 10% traffic to canary pods
- Gradual increase based on metrics
- Automatic rollback on error threshold

```yaml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  annotations:
    nginx.ingress.kubernetes.io/canary: "true"
    nginx.ingress.kubernetes.io/canary-weight: "10"
```

#### Horizontal Pod Autoscaling
```yaml
minReplicas: 5
maxReplicas: 20
metrics:
  - type: Resource
    resource:
      name: cpu
      target:
        averageUtilization: 60
  - type: Pods
    pods:
      metric:
        name: http_requests_per_second
      target:
        averageValue: "1000"
```

#### Pod Disruption Budget
```yaml
apiVersion: policy/v1
kind: PodDisruptionBudget
spec:
  minAvailable: 60%  # Ensure at least 60% of pods available
```

## Health Checks

### Liveness Probe
- **Purpose**: Detect if application is hung
- **Endpoint**: `/api/health`
- **Interval**: 20s
- **Failure Threshold**: 3
- **Action**: Restart container

### Readiness Probe
- **Purpose**: Detect if application is ready to serve traffic
- **Endpoint**: `/api/ready`
- **Interval**: 10s
- **Failure Threshold**: 3
- **Action**: Remove from service endpoints

### Startup Probe
- **Purpose**: Allow slow-starting containers time to initialize
- **Endpoint**: `/api/health`
- **Failure Threshold**: 30 (5 minutes max)

## Deployment Module (`src/deployment/`)

### HealthCheckManager
```rust
let mut health = HealthCheckManager::new("1.0.0");

// Register custom checks
health.register_check(Box::new(DatabaseCheck::new()));
health.register_check(Box::new(RedisCheck::new()));

// Get overall status
let status = health.check_health();
println!("Status: {}", status.status); // healthy, degraded, unhealthy
```

### Environment Configuration
```rust
let env = Environment::from_str("production").unwrap();
let config = DeploymentConfig {
    environment: env,
    log_level: "warn".to_string(),
    paper_trading: false,
    rate_limit_enabled: true,
    circuit_breaker_enabled: true,
};
```

### Graceful Shutdown
```rust
let shutdown = ShutdownHandler::new(30); // 30s timeout

// On SIGTERM
shutdown.request_shutdown();
// Wait for active connections to complete...
```

## Testing

### Deployment Integration Tests (`tests/sprint35_deployment_test.rs`)

```bash
cargo test --test sprint35_deployment_test
```

**18 test cases covering:**
- Health check validation
- Resource limit validation
- Canary deployment testing
- Rollback detection
- Production readiness checks

### Unit Tests (`src/deployment/mod.rs`)

```bash
cargo test --lib deployment::
# 16 tests passing
```

## Deployment Commands

### Deploy to Development
```bash
cd k8s/overlays/dev
kustomize build . | kubectl apply -f -
kubectl rollout status deployment/investor-api -n investor-os-dev
```

### Deploy to Staging
```bash
cd k8s/overlays/staging
kustomize edit set image api=ghcr.io/neurocod/investor-os/api:main
kustomize build . | kubectl apply -f -
kubectl rollout status deployment/investor-api -n investor-os-staging
```

### Deploy to Production (Manual)
```bash
# 1. Tag release
git tag -a v1.0.0 -m "Release v1.0.0"
git push origin v1.0.0

# 2. Monitor canary
kubectl get pods -n investor-os -l version=canary
kubectl logs -f deployment/investor-api-canary -n investor-os

# 3. Promote or rollback
# Auto-promotion happens after canary passes health checks
# Manual rollback:
kubectl delete -f k8s/overlays/prod/canary-deployment.yaml
```

## Monitoring & Observability

### Prometheus Metrics
- `http_requests_total`: Total HTTP requests
- `http_request_duration_seconds`: Request latency
- `investor_os_health_score`: Overall health (0-1)
- `investor_os_uptime_seconds`: Service uptime

### Grafana Dashboards
- Service health overview
- Request rate and latency
- Error rates by endpoint
- Kubernetes resource usage

### Alerts
- **Critical**: Health score < 0.9
- **Warning**: Error rate > 0.1%
- **Info**: Deployment completed

## Security

### Container Security
- Non-root user (UID 1000)
- Read-only root filesystem
- Dropped all capabilities
- Distroless base image

### Secrets Management
- Kubernetes Secrets encrypted at rest
- External Secrets Operator for cloud providers
- Sealed Secrets for GitOps

### Network Security
- TLS 1.3 for all ingress
- Network policies between services
- Service mesh (Istio) ready

## Rollback Procedures

### Automatic Rollback
Triggers:
- Error rate > 5%
- P99 latency > 2s
- Health score < 0.95

### Manual Rollback
```bash
# Rollback to previous version
kubectl rollout undo deployment/investor-api -n investor-os

# Rollback to specific revision
kubectl rollout undo deployment/investor-api -n investor-os --to-revision=3
```

## Best Practices

1. **Always use Kustomize** for environment-specific changes
2. **Never commit secrets** to Git (use Sealed Secrets or external providers)
3. **Test in staging** before production deployment
4. **Monitor canary metrics** before full promotion
5. **Keep resource limits** within 2x of requests
6. **Use graceful shutdown** for zero-downtime deployments
7. **Enable circuit breakers** in production
8. **Set up proper PDBs** for high availability

## Troubleshooting

### Pod not starting
```bash
kubectl describe pod <pod-name> -n investor-os
kubectl logs <pod-name> -n investor-os --previous
```

### Health check failures
```bash
kubectl exec -it <pod-name> -n investor-os -- curl localhost:3000/api/health
```

### High memory usage
```bash
kubectl top pods -n investor-os
kubectl logs <pod-name> -n investor-os | grep -i "out of memory"
```

## Future Enhancements

- [ ] GitOps with ArgoCD/Flux
- [ ] Feature flags for gradual rollout
- [ ] A/B testing infrastructure
- [ ] Chaos engineering (Litmus)
- [ ] Cost optimization recommendations
- [ ] Multi-region deployment
- [ ] Blue-green deployments
