# Investor OS Makefile
# Sprint 8: Production Deployment Commands

.PHONY: help build test deploy deploy-staging deploy-prod backup restore

# Default target
help:
	@echo "Investor OS - Available Commands:"
	@echo ""
	@echo "Development:"
	@echo "  make build          - Build Docker image"
	@echo "  make test           - Run all tests"
	@echo "  make lint           - Run clippy and fmt"
	@echo ""
	@echo "Kubernetes:"
	@echo "  make k8s-apply      - Apply K8s manifests to current context"
	@echo "  make k8s-delete     - Delete K8s resources"
	@echo "  make k8s-status     - Show K8s status"
	@echo ""
	@echo "Deployment:"
	@echo "  make deploy-dev     - Deploy to development"
	@echo "  make deploy-staging - Deploy to staging"
	@echo "  make deploy-prod    - Deploy to production"
	@echo ""
	@echo "Database:"
	@echo "  make migrate        - Run database migrations"
	@echo "  make backup         - Create backup"
	@echo "  make restore FILE=x - Restore from backup"
	@echo ""
	@echo "Monitoring:"
	@echo "  make logs           - View API logs"
	@echo "  make logs-f         - Follow API logs"
	@echo "  make health         - Check health status"

# ==================== Development ====================

build:
	docker build -t investor-os-api:latest .

test:
	cargo test -- --test-threads=1

test-e2e:
	cd frontend/investor-dashboard && npx playwright test

lint:
	cargo clippy -- -D warnings
	cargo fmt -- --check

# ==================== Kubernetes ====================

NAMESPACE ?= investor-os
K8S_DIR = k8s/base

k8s-apply:
	kubectl apply -k $(K8S_DIR) -n $(NAMESPACE)
	kubectl apply -k $(K8S_DIR) -n $(NAMESPACE)

k8s-delete:
	kubectl delete -k $(K8S_DIR) -n $(NAMESPACE) --ignore-not-found

k8s-status:
	@echo "=== Pods ==="
	kubectl get pods -n $(NAMESPACE)
	@echo ""
	@echo "=== Services ==="
	kubectl get svc -n $(NAMESPACE)
	@echo ""
	@echo "=== Ingress ==="
	kubectl get ingress -n $(NAMESPACE)

# ==================== Deployment ====================

IMAGE_TAG ?= $(shell git rev-parse --short HEAD)
IMAGE = ghcr.io/neurocod/investor-os-api

deploy-dev:
	kubectl set image deployment/investor-api \
		api=$(IMAGE):$(IMAGE_TAG) -n investor-os-dev
	kubectl rollout status deployment/investor-api -n investor-os-dev

deploy-staging:
	kubectl set image deployment/investor-api \
		api=$(IMAGE):$(IMAGE_TAG) -n investor-os-staging
	kubectl rollout status deployment/investor-api -n investor-os-staging

deploy-prod:
	@echo "⚠️  Deploying to PRODUCTION..."
	@read -p "Are you sure? (yes/no): " confirm && [ $$confirm = yes ]
	$(MAKE) migrate-prod
	kubectl set image deployment/investor-api \
		api=$(IMAGE):$(IMAGE_TAG) -n investor-os
	kubectl rollout status deployment/investor-api -n investor-os

# ==================== Database ====================

migrate:
	sqlx migrate run

migrate-prod:
	kubectl run migrate-$$(date +%s) \
		--image=$(IMAGE):$(IMAGE_TAG) \
		--rm -i --restart=Never \
		--env="DATABASE_URL=$$(kubectl get secret api-secret -n investor-os -o jsonpath='{.data.database-url}' | base64 -d)" \
		-- ./investor-api migrate

backup:
	./scripts/backup.sh $(NAMESPACE)

restore:
ifndef FILE
	$(error FILE is required. Usage: make restore FILE=backup.sql.gz)
endif
	./scripts/restore.sh $(FILE) $(NAMESPACE)

# ==================== Monitoring ====================

logs:
	kubectl logs -l app=investor-api -n $(NAMESPACE) --tail=100

logs-f:
	kubectl logs -l app=investor-api -n $(NAMESPACE) -f

health:
	@curl -s http://localhost:3000/api/health | jq || curl -s http://localhost:3000/api/health

# ==================== Secrets Management ====================

secrets-create:
	# Create PostgreSQL secret
	kubectl create secret generic postgres-secret \
		--from-literal=username=investor \
		--from-literal=password=$$(openssl rand -base64 32) \
		-n $(NAMESPACE) --dry-run=client -o yaml | kubectl apply -f -

	# Create API secret
	kubectl create secret generic api-secret \
		--from-literal=database-url="postgres://investor:$$(kubectl get secret postgres-secret -n $(NAMESPACE) -o jsonpath='{.data.password}' | base64 -d)@postgres:5432/investor_os" \
		--from-literal=redis-url="redis://redis:6379" \
		-n $(NAMESPACE) --dry-run=client -o yaml | kubectl apply -f -

# ==================== Development Environment ====================

dev-up:
	docker-compose up -d

dev-down:
	docker-compose down

dev-logs:
	docker-compose logs -f

# ==================== Utilities ====================

port-forward:
	kubectl port-forward svc/investor-api 3000:80 -n $(NAMESPACE)

shell:
	kubectl exec -it deployment/investor-api -n $(NAMESPACE) -- /bin/sh

# Version bump
VERSION ?=
bump-version:
ifndef VERSION
	$(error VERSION is required. Usage: make bump-version VERSION=1.2.3)
endif
	sed -i 's/^version = ".*"/version = "$(VERSION)"/' Cargo.toml
	git add Cargo.toml Cargo.lock
	git commit -m "chore(release): bump version to $(VERSION)"
	git tag -a "v$(VERSION)" -m "Release v$(VERSION)"
	@echo "Created tag v$(VERSION). Push with: git push && git push origin v$(VERSION)"
