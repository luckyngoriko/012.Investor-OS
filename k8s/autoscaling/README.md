# Kubernetes Auto-Scaling Setup

## Sprint 50: Auto-Scaling Configuration

### Quick Start

```bash
# Deploy everything
kubectl apply -k .

# Or step by step
kubectl apply -f namespace.yaml
kubectl apply -f configmap.yaml
kubectl apply -f service-account.yaml
kubectl apply -f service.yaml
kubectl apply -f deployment.yaml
kubectl apply -f hpa.yaml
kubectl apply -f pod-disruption-budget.yaml
```

### Check Status

```bash
# View pods
kubectl get pods -n investor-os

# View HPA
kubectl get hpa -n investor-os

# View metrics
kubectl top pods -n investor-os

# View HRM-specific metrics
curl http://hrm-inference:9090/metrics
```

### Scaling Behavior

| Metric | Target | Min Replicas | Max Replicas |
|--------|--------|--------------|--------------|
| CPU | 70% | 2 | 20 |
| Memory | 80% | 2 | 20 |
| RPS | 1000 | 2 | 20 |
| Latency p99 | 2ms | 2 | 20 |

### Cost Optimization

- **Night Scale Down**: Automatic scaling to 1 replica at 10 PM
- **Morning Scale Up**: Automatic scaling to 3 replicas at 8 AM (weekdays)
- **Spot Instances**: Toleration for spot instance nodes

### Manual Scaling

```bash
# Scale manually
kubectl scale deployment hrm-inference --replicas=10 -n investor-os

# Update HPA
kubectl patch hpa hrm-hpa -p '{"spec":{"maxReplicas":50}}' -n investor-os
```

### Troubleshooting

```bash
# Check pod logs
kubectl logs -f deployment/hrm-inference -n investor-os

# Describe pod
kubectl describe pod <pod-name> -n investor-os

# Check HPA events
kubectl describe hpa hrm-hpa -n investor-os

# Port forward for testing
kubectl port-forward svc/hrm-inference 8080:8080 -n investor-os
```
