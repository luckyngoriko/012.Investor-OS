# Investor OS - Makefile
# Quick commands for development and deployment

.PHONY: help dev build test docker-up docker-down db-optimize db-migrate frontend-dev e2e-test

# Default target
help:
	@echo "Investor OS - Available Commands"
	@echo "================================="
	@echo ""
	@echo "Development:"
	@echo "  make dev              - Start all services with docker compose"
	@echo "  make dev-api          - Run API locally (cargo run)"
	@echo "  make dev-frontend     - Run frontend dev server (npm run dev)"
	@echo ""
	@echo "Build:"
	@echo "  make build            - Build Rust release binary"
	@echo "  make build-frontend   - Build frontend for production"
	@echo ""
	@echo "Database:"
	@echo "  make db-up            - Start PostgreSQL and Redis"
	@echo "  make db-migrate       - Run SQL migrations"
	@echo "  make db-optimize      - Apply PostgreSQL performance optimizations"
	@echo "  make db-reset         - Reset database (WARNING: deletes data)"
	@echo "  make db-psql          - Open PostgreSQL shell"
	@echo ""
	@echo "Testing:"
	@echo "  make test             - Run all Rust tests"
	@echo "  make test-sprint1     - Run Sprint 1 Golden Path tests"
	@echo "  make test-sprint2     - Run Sprint 2 tests"
	@echo "  make test-sprint3     - Run Sprint 3 tests"
	@echo "  make e2e-test         - Run Playwright E2E tests"
	@echo ""
	@echo "Code Quality:"
	@echo "  make lint             - Run clippy lints"
	@echo "  make fmt              - Format Rust code"
	@echo ""
	@echo "Monitoring:"
	@echo "  make logs             - Show all container logs"
	@echo "  make logs-api         - Show API logs only"
	@echo "  make status           - Check container status"
	@echo ""
	@echo "Production:"
	@echo "  make deploy           - Deploy to production"
	@echo "  make backup           - Backup database"
	@echo ""

# Development
dev:
	docker compose up -d

dev-api:
	cargo run --bin investor-api

dev-frontend:
	cd frontend/investor-dashboard && npm run dev

# Build
build:
	cargo build --release

build-frontend:
	cd frontend/investor-dashboard && npm run build

# Testing
test:
	cargo test -- --test-threads=1

test-sprint1:
	cargo test -p investor-tests golden_path_sprint1 -- --nocapture

test-sprint2:
	cargo test -p investor-tests golden_path_sprint2 -- --nocapture

test-sprint3:
	cargo test -p investor-tests golden_path_sprint3 -- --nocapture

test-golden-path: test-sprint1 test-sprint2 test-sprint3

e2e-test:
	cd frontend/investor-dashboard && npm run test:e2e

e2e-test-ui:
	cd frontend/investor-dashboard && npm run test:e2e:ui

# Database
db-up:
	docker compose up -d postgres redis

db-migrate:
	docker compose exec -T postgres psql -U investor -d investor_os < migrations/001_postgres_optimization.sql

db-optimize: db-migrate
	@echo "PostgreSQL optimizations applied!"

db-reset:
	docker compose down -v
	docker volume rm investor-os_postgres_data || true
	docker compose up -d postgres redis
	@echo "Waiting for PostgreSQL to start..."
	@sleep 5
	@echo "Database reset complete!"

db-psql:
	docker compose exec postgres psql -U investor -d investor_os

db-stats:
	docker compose exec postgres psql -U investor -c "SELECT * FROM timescaledb_information.hypertables;"

db-slow-queries:
	docker compose exec postgres psql -U investor -c "SELECT query, mean_exec_time FROM pg_stat_statements ORDER BY mean_exec_time DESC LIMIT 10;"

db-refresh-views:
	docker compose exec postgres psql -U investor -c "SELECT refresh_dashboard_views();"

# Code Quality
lint:
	cargo clippy -- -D warnings

fmt:
	cargo fmt

fmt-check:
	cargo fmt -- --check

# Docker
docker-up:
	docker compose up -d --build

docker-down:
	docker compose down

docker-clean:
	docker compose down -v
	docker system prune -f

# Monitoring
logs:
	docker compose logs -f

logs-api:
	docker compose logs -f api

logs-postgres:
	docker compose logs -f postgres

status:
	docker compose ps

# Production
backup:
	docker compose exec postgres pg_dump -U investor investor_os > backup_$$(date +%Y%m%d_%H%M%S).sql
	@echo "Backup created: backup_$$(date +%Y%m%d_%H%M%S).sql"

restore:
	@if [ -z "$(FILE)" ]; then \
		echo "Usage: make restore FILE=backup_20260101_120000.sql"; \
		exit 1; \
	fi
	docker compose exec -T postgres psql -U investor -d investor_os < $(FILE)
	@echo "Restore complete from $(FILE)"

# Grafana
grafana-import:
	@echo "Grafana dashboards are auto-provisioned from config/grafana/dashboards/"

# Full test suite (all 5 gates)
gates:
	@echo "=== Gate 1: Golden Path Tests ==="
	cargo test -- --test-threads=1
	@echo "=== Gate 2: Clippy ==="
	cargo clippy -- -D warnings
	@echo "=== Gate 3: Build ==="
	cargo build --release
	@echo "=== Gate 4: Format Check ==="
	cargo fmt -- --check
	@echo "=== All Gates Passed! ==="

# CI/CD
ci: fmt-check lint test build
	@echo "CI checks complete!"

# Setup for new developers
setup:
	@echo "Setting up Investor OS..."
	@cp .env.example .env
	@echo "Installing Rust dependencies..."
	cargo fetch
	@echo "Installing frontend dependencies..."
	cd frontend/investor-dashboard && npm install
	@echo "Installing Playwright browsers..."
	cd frontend/investor-dashboard && npx playwright install
	@echo "Setup complete! Edit .env and run 'make dev'"

# Clean build artifacts
clean:
	cargo clean
	rm -rf frontend/investor-dashboard/.next
	rm -rf frontend/investor-dashboard/dist
