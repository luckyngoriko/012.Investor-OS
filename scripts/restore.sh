#!/bin/bash
# S8-D9: Disaster Recovery - Restore Script
# Usage: ./restore.sh <backup_timestamp|backup_file> [namespace] [component]
# Components: all (default), database, redis, config

set -euo pipefail

if [ $# -lt 1 ]; then
    echo "Usage: $0 <backup_timestamp|backup_file> [namespace] [component]"
    echo ""
    echo "Examples:"
    echo "  $0 20240208_120000                    # Restore all from timestamp"
    echo "  $0 20240208_120000 investor-os database  # Restore only database"
    echo "  $0 db_backup_20240208_120000.sql.gz   # Restore specific file"
    echo ""
    echo "Components: all (default), database, redis, config"
    exit 1
fi

BACKUP_IDENTIFIER="$1"
NAMESPACE="${2:-investor-os}"
COMPONENT="${3:-all}"
BACKUP_BUCKET="${BACKUP_BUCKET:-gs://investor-os-backups}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
LOG_FILE="${SCRIPT_DIR}/../logs/restore_${TIMESTAMP}.log"

# Create logs directory
mkdir -p "$(dirname "$LOG_FILE")"

# Logging function
log() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] $*" | tee -a "$LOG_FILE"
}

error() {
    log "ERROR: $*" >&2
    exit 1
}

# Determine if identifier is a timestamp or filename
if [[ $BACKUP_IDENTIFIER =~ ^[0-9]{8}_[0-9]{6}$ ]]; then
    BACKUP_TIMESTAMP="$BACKUP_IDENTIFIER"
    BACKUP_FILE=""
    log "Using backup timestamp: ${BACKUP_TIMESTAMP}"
elif [[ $BACKUP_IDENTIFIER =~ db_backup_.*\.sql\.gz$ ]]; then
    BACKUP_FILE="$BACKUP_IDENTIFIER"
    BACKUP_TIMESTAMP=$(echo "$BACKUP_FILE" | grep -oP '[0-9]{8}_[0-9]{6}' || echo "unknown")
    log "Using backup file: ${BACKUP_FILE}"
else
    error "Invalid backup identifier. Use timestamp (YYYYMMDD_HHMMSS) or backup filename"
fi

log "========================================="
log "Investor OS Restore"
log "Timestamp: ${BACKUP_TIMESTAMP}"
log "Namespace: ${NAMESPACE}"
log "Component: ${COMPONENT}"
log "========================================="

# ==================== Pre-flight Checks ====================
log "[0/6] Running pre-flight checks..."

# Check kubectl
if ! command -v kubectl &> /dev/null; then
    error "kubectl is not installed"
fi

# Check namespace exists
if ! kubectl get namespace "${NAMESPACE}" &> /dev/null; then
    error "Namespace '${NAMESPACE}' does not exist"
fi

# Check if using GCS
USE_GCS=false
if command -v gsutil &> /dev/null && gsutil ls "${BACKUP_BUCKET}" &> /dev/null; then
    USE_GCS=true
    log "✓ GCS backend available"
else
    log "Using local backup storage"
fi

# Check if backup exists
if [ -n "$BACKUP_FILE" ]; then
    DB_BACKUP_FILE="$BACKUP_FILE"
else
    DB_BACKUP_FILE="db_backup_${BACKUP_TIMESTAMP}.sql.gz"
fi

if [ "$USE_GCS" = true ]; then
    if ! gsutil ls "${BACKUP_BUCKET}/database/${DB_BACKUP_FILE}" &> /dev/null; then
        error "Backup not found: ${BACKUP_BUCKET}/database/${DB_BACKUP_FILE}"
    fi
else
    LOCAL_BACKUP_DIR="${SCRIPT_DIR}/../backups"
    if [ ! -f "${LOCAL_BACKUP_DIR}/database/${DB_BACKUP_FILE}" ]; then
        error "Backup not found: ${LOCAL_BACKUP_DIR}/database/${DB_BACKUP_FILE}"
    fi
fi

log "✓ Pre-flight checks passed"

# ==================== Confirm Restore ====================
log ""
echo "⚠️  WARNING: This will OVERWRITE data in namespace '${NAMESPACE}'"
echo "    Component to restore: ${COMPONENT}"
echo "    Backup: ${DB_BACKUP_FILE}"
echo ""
read -p "Are you sure you want to continue? (type 'RESTORE' to confirm): " confirm
if [ "$confirm" != "RESTORE" ]; then
    log "Restore cancelled by user"
    exit 0
fi

# Get PostgreSQL pod
POSTGRES_POD=$(kubectl get pod -n "${NAMESPACE}" -l app=postgres -o jsonpath='{.items[0].metadata.name}' 2>/dev/null) \
    || error "PostgreSQL pod not found"

# ==================== Restore Database ====================
if [ "$COMPONENT" = "all" ] || [ "$COMPONENT" = "database" ]; then
    log "[1/6] Scaling down API to prevent writes..."
    kubectl scale deployment investor-api --replicas=0 -n "${NAMESPACE}" || true
    sleep 5
    
    log "[2/6] Downloading backup..."
    TEMP_DIR="/tmp/restore_${TIMESTAMP}"
    mkdir -p "$TEMP_DIR"
    
    if [ "$USE_GCS" = true ]; then
        gsutil cp "${BACKUP_BUCKET}/database/${DB_BACKUP_FILE}" "$TEMP_DIR/" \
            || error "Failed to download backup from GCS"
        
        # Download checksum if available
        gsutil cp "${BACKUP_BUCKET}/database/${DB_BACKUP_FILE}.md5" "$TEMP_DIR/" 2>/dev/null || true
    else
        cp "${LOCAL_BACKUP_DIR}/database/${DB_BACKUP_FILE}" "$TEMP_DIR/" \
            || error "Failed to copy backup from local storage"
        cp "${LOCAL_BACKUP_DIR}/database/${DB_BACKUP_FILE}.md5" "$TEMP_DIR/" 2>/dev/null || true
    fi
    
    # Verify checksum
    if [ -f "$TEMP_DIR/${DB_BACKUP_FILE}.md5" ]; then
        log "Verifying backup checksum..."
        cd "$TEMP_DIR"
        if md5sum -c "${DB_BACKUP_FILE}.md5"; then
            log "✓ Checksum verified"
        else
            log "⚠️  Checksum verification failed - continuing anyway"
        fi
        cd - > /dev/null
    fi
    
    log "[3/6] Creating pre-restore backup (safety)..."
    SAFETY_BACKUP="/tmp/safety_backup_${TIMESTAMP}.sql.gz"
    kubectl exec -n "${NAMESPACE}" "${POSTGRES_POD}" -- \
        pg_dump -U investor -d investor_os -Fc 2>/dev/null | gzip > "$SAFETY_BACKUP" \
        || log "⚠️  Safety backup failed (continuing)"
    
    log "[4/6] Terminating active connections..."
    kubectl exec -n "${NAMESPACE}" "${POSTGRES_POD}" -- psql -U investor -d postgres -c "
        SELECT pg_terminate_backend(pid) 
        FROM pg_stat_activity 
        WHERE datname = 'investor_os' 
        AND pid <> pg_backend_pid();
    " 2>/dev/null || true
    
    sleep 2
    
    log "[5/6] Restoring database..."
    
    # Drop and recreate database
    kubectl exec -n "${NAMESPACE}" "${POSTGRES_POD}" -- psql -U investor -d postgres -c "
        DROP DATABASE IF EXISTS investor_os;
        CREATE DATABASE investor_os;
    " || error "Failed to recreate database"
    
    # Restore from backup
    if [[ $DB_BACKUP_FILE == *.gz ]]; then
        gunzip -c "$TEMP_DIR/${DB_BACKUP_FILE}" | \
            kubectl exec -i -n "${NAMESPACE}" "${POSTGRES_POD}" -- \
            pg_restore -U investor -d investor_os --verbose 2>> "$LOG_FILE" \
            || log "⚠️  pg_restore had warnings (this is usually OK)"
    else
        kubectl cp "$TEMP_DIR/${DB_BACKUP_FILE}" "${NAMESPACE}/${POSTGRES_POD}:/tmp/restore.sql"
        kubectl exec -n "${NAMESPACE}" "${POSTGRES_POD}" -- \
            psql -U investor -d investor_os -f /tmp/restore.sql 2>> "$LOG_FILE" \
            || error "Database restore failed"
    fi
    
    # Verify restore
    TABLE_COUNT=$(kubectl exec -n "${NAMESPACE}" "${POSTGRES_POD}" -- \
        psql -U investor -d investor_os -t -c "SELECT COUNT(*) FROM information_schema.tables WHERE table_schema = 'public'" 2>/dev/null | tr -d ' ') \
        || TABLE_COUNT="0"
    
    if [ "$TABLE_COUNT" -gt 0 ]; then
        log "✓ Database restored successfully (${TABLE_COUNT} tables)"
    else
        error "Database restore verification failed - no tables found"
    fi
    
    # Cleanup
    rm -rf "$TEMP_DIR"
    kubectl exec -n "${NAMESPACE}" "${POSTGRES_POD}" -- rm -f /tmp/restore.sql 2>/dev/null || true
fi

# ==================== Restore Redis ====================
if [ "$COMPONENT" = "all" ] || [ "$COMPONENT" = "redis" ]; then
    log "[6/6] Restoring Redis..."
    
    REDIS_POD=$(kubectl get pod -n "${NAMESPACE}" -l app=redis -o jsonpath='{.items[0].metadata.name}' 2>/dev/null) \
        || error "Redis pod not found"
    
    REDIS_BACKUP="redis_backup_${BACKUP_TIMESTAMP}.rdb"
    TEMP_RDB="/tmp/redis_restore_${TIMESTAMP}.rdb"
    
    # Download Redis backup
    if [ "$USE_GCS" = true ]; then
        gsutil cp "${BACKUP_BUCKET}/redis/${REDIS_BACKUP}" "$TEMP_RDB" \
            || error "Failed to download Redis backup"
    else
        cp "${LOCAL_BACKUP_DIR}/redis/${REDIS_BACKUP}" "$TEMP_RDB" \
            || error "Failed to copy Redis backup"
    fi
    
    # Scale down Redis (to replace RDB file)
    kubectl scale deployment redis --replicas=0 -n "${NAMESPACE}" || true
    sleep 5
    
    # Copy RDB file to Redis pod (will be used on next start)
    kubectl cp "$TEMP_RDB" "${NAMESPACE}/${REDIS_POD}:/data/dump.rdb" \
        || error "Failed to copy RDB file"
    
    # Scale Redis back up
    kubectl scale deployment redis --replicas=1 -n "${NAMESPACE}" || true
    
    rm -f "$TEMP_RDB"
    log "✓ Redis restore completed (Redis will restart with restored data)"
fi

# ==================== Restore K8s Config ====================
if [ "$COMPONENT" = "all" ] || [ "$COMPONENT" = "config" ]; then
    log "Restoring Kubernetes configuration..."
    
    if [ "$USE_GCS" = true ]; then
        CONFIG_BACKUP="manifests/manifest_${BACKUP_TIMESTAMP}.json"
        gsutil cp "${BACKUP_BUCKET}/${CONFIG_BACKUP}" "/tmp/" 2>/dev/null || \
            log "⚠️  Config manifest not found"
    fi
    
    log "ℹ️  Note: K8s secrets/configmaps are informational only. Manual restoration may be required."
fi

# ==================== Scale API Back Up ====================
if [ "$COMPONENT" = "all" ] || [ "$COMPONENT" = "database" ]; then
    log ""
    log "Scaling API back up..."
    kubectl scale deployment investor-api --replicas=3 -n "${NAMESPACE}" || true
    
    # Wait for rollout
    log "Waiting for API rollout..."
    kubectl rollout status deployment/investor-api -n "${NAMESPACE}" --timeout=120s || \
        log "⚠️  Rollout status check failed (may still be rolling out)"
fi

# ==================== Post-Restore Verification ====================
log ""
log "========================================="
log "Post-Restore Verification"
log "========================================="

if [ "$COMPONENT" = "all" ] || [ "$COMPONENT" = "database" ]; then
    # Check table counts
    log "Table counts:"
    kubectl exec -n "${NAMESPACE}" "${POSTGRES_POD}" -- psql -U investor -d investor_os -c "
        SELECT 
            (SELECT COUNT(*) FROM prices) as prices,
            (SELECT COUNT(*) FROM signals) as signals,
            (SELECT COUNT(*) FROM orders) as orders
    " 2>/dev/null || log "⚠️  Could not get table counts"
fi

log ""
log "========================================="
log "Restore complete!"
log "========================================="
log "Backup timestamp: ${BACKUP_TIMESTAMP}"
log "Log file: ${LOG_FILE}"
if [ -f "$SAFETY_BACKUP" ]; then
    log "Safety backup: ${SAFETY_BACKUP}"
fi
log ""
log "Verification commands:"
log "  kubectl get pods -n ${NAMESPACE}"
log "  kubectl logs -n ${NAMESPACE} deployment/investor-api --tail=50"
