#!/bin/bash
# Deploy Investor OS to Kubernetes
# Sprint 50: Auto-Scaling

set -e

NAMESPACE="investor-os"
ENVIRONMENT=${1:-staging}

echo "╔═══════════════════════════════════════════════════════════════╗"
echo "║       Investor OS - Kubernetes Deployment (Sprint 50)         ║"
echo "╚═══════════════════════════════════════════════════════════════╝"
echo ""
echo "Environment: $ENVIRONMENT"
echo "Namespace: $NAMESPACE"
echo ""

# Check prerequisites
echo "Checking prerequisites..."
command -v kubectl >/dev/null 2>&1 || { echo "kubectl required but not installed."; exit 1; }
command -v docker >/dev/null 2>&1 || { echo "docker required but not installed."; exit 1; }

# Build Docker image
echo ""
echo "Building Docker image..."
docker build -t investor-os:latest -t investor-os:v3.0 .

# Deploy to Kubernetes
echo ""
echo "Deploying to Kubernetes..."
kubectl apply -k k8s/autoscaling/

# Wait for deployment
echo ""
echo "Waiting for deployment to be ready..."
kubectl rollout status deployment/hrm-inference -n $NAMESPACE --timeout=300s

# Verify deployment
echo ""
echo "Verifying deployment..."
echo ""
echo "Pods:"
kubectl get pods -n $NAMESPACE

echo ""
echo "Services:"
kubectl get svc -n $NAMESPACE

echo ""
echo "HPA:"
kubectl get hpa -n $NAMESPACE

echo ""
echo "═══════════════════════════════════════════════════════════════"
echo "✅ Deployment complete!"
echo ""
echo "Access the service:"
echo "  kubectl port-forward svc/hrm-inference 8080:8080 -n $NAMESPACE"
echo ""
echo "View logs:"
echo "  kubectl logs -f deployment/hrm-inference -n $NAMESPACE"
echo ""
echo "Check HPA status:"
echo "  kubectl get hpa -n $NAMESPACE -w"
echo "═══════════════════════════════════════════════════════════════"
