#!/usr/bin/env bash
set -euo pipefail

BACKEND_BASE_URL="${BACKEND_BASE_URL:-http://127.0.0.1:8080}"
FRONTEND_BASE_URL="${FRONTEND_BASE_URL:-http://127.0.0.1:3000}"
REQUIRE_BACKEND="${REQUIRE_BACKEND:-0}"
REQUIRE_FRONTEND="${REQUIRE_FRONTEND:-0}"
CURL_MAX_TIME="${CURL_MAX_TIME:-8}"

WORKDIR="$(mktemp -d)"
trap 'rm -rf "$WORKDIR"' EXIT

log() {
  printf '[runtime-smoke] %s\n' "$*"
}

check_contains() {
  local file="$1"
  local expected="$2"
  if ! grep -q "$expected" "$file"; then
    log "ASSERTION FAILED: expected body to contain: $expected"
    log "--- body ---"
    cat "$file"
    log "------------"
    return 1
  fi
}

http_get() {
  local url="$1"
  local outfile="$2"
  curl -sS -m "$CURL_MAX_TIME" -o "$outfile" -w "%{http_code}" "$url" || true
}

ensure_endpoint_200() {
  local url="$1"
  local name="$2"
  local outfile="$WORKDIR/${name}.body"
  local status
  status="$(http_get "$url" "$outfile")"
  if [[ "$status" != "200" ]]; then
    log "ASSERTION FAILED: ${name} expected HTTP 200, got ${status} (${url})"
    [[ -s "$outfile" ]] && cat "$outfile"
    return 1
  fi
  log "OK ${name} -> 200"
}

backend_probe="$WORKDIR/backend_probe.body"
backend_probe_status="$(http_get "${BACKEND_BASE_URL}/api/health" "$backend_probe")"
if [[ "$backend_probe_status" != "200" ]]; then
  if [[ "$REQUIRE_BACKEND" == "1" ]]; then
    log "Backend probe failed with HTTP ${backend_probe_status}: ${BACKEND_BASE_URL}/api/health"
    [[ -s "$backend_probe" ]] && cat "$backend_probe"
    exit 1
  fi
  log "Backend is unavailable (HTTP ${backend_probe_status}), skipping backend runtime assertions."
  exit 0
fi

ensure_endpoint_200 "${BACKEND_BASE_URL}/api/health" "api_health"
check_contains "$WORKDIR/api_health.body" '"success":true'
check_contains "$WORKDIR/api_health.body" '"runtime_contract"'
check_contains "$WORKDIR/api_health.body" '"api_base_url"'
check_contains "$WORKDIR/api_health.body" '"ws_hrm_url"'

ensure_endpoint_200 "${BACKEND_BASE_URL}/api/ready" "api_ready"
check_contains "$WORKDIR/api_ready.body" '"success":true'
check_contains "$WORKDIR/api_ready.body" '"status":"ready"'

ensure_endpoint_200 "${BACKEND_BASE_URL}/api/runtime/config" "runtime_config"
check_contains "$WORKDIR/runtime_config.body" '"success":true'
check_contains "$WORKDIR/runtime_config.body" '"api_base_url"'
check_contains "$WORKDIR/runtime_config.body" '"ws_hrm_url"'
check_contains "$WORKDIR/runtime_config.body" '"allowed_origins"'

ensure_endpoint_200 "${BACKEND_BASE_URL}/metrics" "metrics"
check_contains "$WORKDIR/metrics.body" '# HELP'

frontend_probe="$WORKDIR/frontend_probe.body"
frontend_probe_status="$(http_get "${FRONTEND_BASE_URL}/login" "$frontend_probe")"
if [[ "$frontend_probe_status" != "200" ]]; then
  if [[ "$REQUIRE_FRONTEND" == "1" ]]; then
    log "Frontend probe failed with HTTP ${frontend_probe_status}: ${FRONTEND_BASE_URL}/login"
    [[ -s "$frontend_probe" ]] && cat "$frontend_probe"
    exit 1
  fi
  log "Frontend is unavailable (HTTP ${frontend_probe_status}), skipping frontend endpoint assertions."
  log "Backend runtime contract smoke passed."
  exit 0
fi

ensure_endpoint_200 "${FRONTEND_BASE_URL}/login" "frontend_login"
ensure_endpoint_200 "${FRONTEND_BASE_URL}/monitoring" "frontend_monitoring"

log "Runtime contract smoke passed."
