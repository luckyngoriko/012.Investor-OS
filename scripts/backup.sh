#!/bin/bash
# S8-D9: Disaster Recovery - Backup Script
# Usage: ./backup.sh [namespace] [local|gcs]
# Environment: BACKUP_BUCKET (default: gs://investor-os-backups)

set -euo pipefail

NAMESPACE="${1:-investor-os}"
STORAGE_BACKEND="${2:-gcs}"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
BACKUP_BUCKET="${BACKUP_BUCKET:-gs://investor-os-backups}"
RETENTION_DAYS=30
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LOG_FILE="${SCRIPT_DIR}/../logs/backup_${TIMESTAMP}.log"

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

log "========================================="
log "Investor OS Backup - ${TIMESTAMP}"
log "Namespace: ${NAMESPACE}"
log "Storage: ${STORAGE_BACKEND}"
log "========================================="

# Create backup directory
BACKUP_DIR="/tmp/backup_${TIMESTAMP}"
mkdir -p "${BACKUP_DIR}"

# Cleanup on exit
trap 'rm -rf "${BACKUP_DIR}"' EXIT

# ==================== Pre-flight Checks ====================
log "[0/5] Running pre-flight checks..."

# Check kubectl
if ! command -v kubectl &> /dev/null; then
    error "kubectl is not installed"
fi

# Check namespace exists
if ! kubectl get namespace "${NAMESPACE}" &> /dev/null; then
    error "Namespace '${NAMESPACE}' does not exist"
fi

# Check GCS if using gcs backend
if [ "$STORAGE_BACKEND" = "gcs" ]; then
    if ! command -v gsutil &> /dev/null; then
        error "gsutil is not installed (required for GCS backend)"
    fi
    
    # Test GCS access
    if ! gsutil ls "${BACKUP_BUCKET}" &> /dev/null; then
        log "WARNING: Cannot access GCS bucket ${BACKUP_BUCKET}. Will save locally."
        STORAGE_BACKEND="local"
    fi
fi

log "✓ Pre-flight checks passed"

# ==================== PostgreSQL Backup ====================
log "[1/5] Backing up PostgreSQL..."

# Get PostgreSQL pod name
POSTGRES_POD=$(kubectl get pod -n "${NAMESPACE}" -l app=postgres -o jsonpath='{.items[0].metadata.name}' 2>/dev/null) \
    || error "PostgreSQL pod not found"

# Check if pod is ready
if ! kubectl exec -n "${NAMESPACE}" "${POSTGRES_POD}" -- pg_isready -q; then
    error "PostgreSQL is not ready"
fi

# Get database stats before backup
DB_STATS=$(kubectl exec -n "${NAMESPACE}" "${POSTGRES_POD}" -- psql -U investor -d investor_os -t -c "
    SELECT 
        (SELECT COUNT(*) FROM prices) as prices,
        (SELECT COUNT(*) FROM signals) as signals,
        (SELECT COUNT(*) FROM orders) as orders,
        pg_database_size('investor_os') as size
" 2>/dev/null || echo "0|0|0|0")

log "Database stats: ${DB_STATS}"

# Create database dump with custom format (compressed)
kubectl exec -n "${NAMESPACE}" "${POSTGRES_POD}" -- \
    pg_dump -U investor -d investor_os -Fc --verbose 2>> "$LOG_FILE" | \
    gzip > "${BACKUP_DIR}/db_backup_${TIMESTAMP}.sql.gz" \
    || error "Database dump failed"

# Calculate checksum
DB_CHECKSUM=$(md5sum "${BACKUP_DIR}/db_backup_${TIMESTAMP}.sql.gz" | cut -d' ' -f1)
echo "${DB_CHECKSUM}" > "${BACKUP_DIR}/db_backup_${TIMESTAMP}.sql.gz.md5"

# Verify backup file
if [ ! -s "${BACKUP_DIR}/db_backup_${TIMESTAMP}.sql.gz" ]; then
    error "Database backup file is empty"
fi

DB_SIZE=$(du -h "${BACKUP_DIR}/db_backup_${TIMESTAMP}.sql.gz" | cut -f1)
log "✓ Database backup successful (${DB_SIZE}, checksum: ${DB_CHECKSUM})"

# ==================== Redis Backup ====================
log "[2/5] Backing up Redis..."

REDIS_POD=$(kubectl get pod -n "${NAMESPACE}" -l app=redis -o jsonpath='{.items[0].metadata.name}' 2>/dev/null) \
    || error "Redis pod not found"

# Check Redis is responsive
if ! kubectl exec -n "${NAMESPACE}" "${REDIS_POD}" -- redis-cli PING | grep -q PONG; then
    error "Redis is not responding"
fi

# Get Redis info
REDIS_INFO=$(kubectl exec -n "${NAMESPACE}" "${REDIS_POD}" -- redis-cli INFO persistence 2>/dev/null || echo "")
log "Redis persistence: $(echo "$REDIS_INFO" | grep rdb_last_save_time || echo 'N/A')"

# Trigger BGSAVE and wait
kubectl exec -n "${NAMESPACE}" "${REDIS_POD}" -- redis-cli BGSAVE \
    || error "Redis BGSAVE failed"

# Wait for save to complete (check every second, timeout 30s)
for i in {1..30}; do
    if kubectl exec -n "${NAMESPACE}" "${REDIS_POD}" -- redis-cli LASTSAVE | grep -q "[0-9]"; then
        break
    fi
    sleep 1
done

# Copy RDB file
kubectl cp "${NAMESPACE}/${REDIS_POD}:/data/dump.rdb" "${BACKUP_DIR}/redis_backup_${TIMESTAMP}.rdb" \
    || error "Redis backup copy failed"

# Calculate checksum
REDIS_CHECKSUM=$(md5sum "${BACKUP_DIR}/redis_backup_${TIMESTAMP}.rdb" | cut -d' ' -f1)
echo "${REDIS_CHECKSUM}" > "${BACKUP_DIR}/redis_backup_${TIMESTAMP}.rdb.md5"

log "✓ Redis backup successful (checksum: ${REDIS_CHECKSUM})"

# ==================== Kubernetes Config Backup ====================
log "[3/5] Backing up Kubernetes configuration..."

# Export critical resources
kubectl get secret -n "${NAMESPACE}" -o yaml > "${BACKUP_DIR}/secrets_${TIMESTAMP}.yaml" 2>/dev/null || true
kubectl get configmap -n "${NAMESPACE}" -o yaml > "${BACKUP_DIR}/configmaps_${TIMESTAMP}.yaml" 2>/dev/null || true

# Mask sensitive data in secrets
sed -i 's/\(password:\|token:\|key:\).*/\1 [MASKED]/g' "${BACKUP_DIR}/secrets_${TIMESTAMP}.yaml" 2>/dev/null || true

log "✓ K8s config backup completed"

# ==================== Create Backup Manifest ====================
log "[4/5] Creating backup manifest..."

cat > "${BACKUP_DIR}/manifest_${TIMESTAMP}.json" << EOF
{
  "backup_timestamp": "${TIMESTAMP}",
  "namespace": "${NAMESPACE}",
  "version": "$(git -C "${SCRIPT_DIR}/.." describe --tags --always 2>/dev/null || echo 'unknown')",
  "components": {
    "postgresql": {
      "file": "db_backup_${TIMESTAMP}.sql.gz",
      "checksum": "${DB_CHECKSUM}",
      "size": "${DB_SIZE}",
      "stats": "${DB_STATS}"
    },
    "redis": {
      "file": "redis_backup_${TIMESTAMP}.rdb",
      "checksum": "${REDIS_CHECKSUM}"
    },
    "kubernetes": {
      "secrets": "secrets_${TIMESTAMP}.yaml",
      "configmaps": "configmaps_${TIMESTAMP}.yaml"
    }
  },
  "storage_backend": "${STORAGE_BACKEND}"
}
EOF

log "✓ Backup manifest created"

# ==================== Upload to Storage ====================
log "[5/5] Uploading backup..."

if [ "$STORAGE_BACKEND" = "gcs" ]; then
    # Upload to Google Cloud Storage
    gsutil -h "Content-Type:application/gzip" cp "${BACKUP_DIR}/db_backup_${TIMESTAMP}.sql.gz" "${BACKUP_BUCKET}/database/" \
        || error "Failed to upload database backup to GCS"
    gsutil cp "${BACKUP_DIR}/db_backup_${TIMESTAMP}.sql.gz.md5" "${BACKUP_BUCKET}/database/" || true
    
    gsutil cp "${BACKUP_DIR}/redis_backup_${TIMESTAMP}.rdb" "${BACKUP_BUCKET}/redis/" \
        || error "Failed to upload Redis backup to GCS"
    gsutil cp "${BACKUP_DIR}/redis_backup_${TIMESTAMP}.rdb.md5" "${BACKUP_BUCKET}/redis/" || true
    
    gsutil cp "${BACKUP_DIR}/manifest_${TIMESTAMP}.json" "${BACKUP_BUCKET}/manifests/" \
        || error "Failed to upload manifest to GCS"
    
    BACKUP_URL="${BACKUP_BUCKET}"
else
    # Local storage
    LOCAL_BACKUP_DIR="${SCRIPT_DIR}/../backups"
    mkdir -p "${LOCAL_BACKUP_DIR}/database" "${LOCAL_BACKUP_DIR}/redis" "${LOCAL_BACKUP_DIR}/manifests"
    
    cp "${BACKUP_DIR}/db_backup_${TIMESTAMP}.sql.gz" "${LOCAL_BACKUP_DIR}/database/"
    cp "${BACKUP_DIR}/db_backup_${TIMESTAMP}.sql.gz.md5" "${LOCAL_BACKUP_DIR}/database/"
    cp "${BACKUP_DIR}/redis_backup_${TIMESTAMP}.rdb" "${LOCAL_BACKUP_DIR}/redis/"
    cp "${BACKUP_DIR}/redis_backup_${TIMESTAMP}.rdb.md5" "${LOCAL_BACKUP_DIR}/redis/"
    cp "${BACKUP_DIR}/manifest_${TIMESTAMP}.json" "${LOCAL_BACKUP_DIR}/manifests/"
    cp "${BACKUP_DIR}/secrets_${TIMESTAMP}.yaml" "${LOCAL_BACKUP_DIR}/manifests/" 2>/dev/null || true
    cp "${BACKUP_DIR}/configmaps_${TIMESTAMP}.yaml" "${LOCAL_BACKUP_DIR}/manifests/" 2>/dev/null || true
    
    BACKUP_URL="${LOCAL_BACKUP_DIR}"
fi

log "✓ Backup uploaded to ${BACKUP_URL}"

# ==================== Cleanup Old Backups ====================
log "Cleaning up backups older than ${RETENTION_DAYS} days..."

if [ "$STORAGE_BACKEND" = "gcs" ]; then
    gsutil ls -r "${BACKUP_BUCKET}/database/" 2>/dev/null | \
        grep "db_backup_" | \
        while read -r file; do
            if [[ $file =~ ([0-9]{8})_([0-9]{6}) ]]; then
                FILE_DATE="${BASH_REMATCH[1]}"
                FILE_TIME="${BASH_REMATCH[2]}"
                FILE_TIMESTAMP=$(date -d "${FILE_DATE:0:4}-${FILE_DATE:4:2}-${FILE_DATE:6:2} ${FILE_TIME:0:2}:${FILE_TIME:2:2}:${FILE_TIME:4:2}" +%s 2>/dev/null || echo 0)
                CURRENT_TIMESTAMP=$(date +%s)
                AGE_DAYS=$(( (CURRENT_TIMESTAMP - FILE_TIMESTAMP) / 86400 ))
                
                if [ "$AGE_DAYS" -gt "$RETENTION_DAYS" ]; then
                    log "Deleting old backup: $file"
                    gsutil rm "$file" 2>/dev/null || true
                fi
            fi
        done
else
    # Local cleanup
    find "${LOCAL_BACKUP_DIR}/database" -name "db_backup_*.sql.gz" -mtime +${RETENTION_DAYS} -delete 2>/dev/null || true
    find "${LOCAL_BACKUP_DIR}/redis" -name "redis_backup_*.rdb" -mtime +${RETENTION_DAYS} -delete 2>/dev/null || true
fi

log "========================================="
log "Backup complete: ${TIMESTAMP}"
log "Location: ${BACKUP_URL}"
log "Log file: ${LOG_FILE}"
log "========================================="

# Output manifest for automation
cat "${BACKUP_DIR}/manifest_${TIMESTAMP}.json"

exit 0
