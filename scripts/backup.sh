#!/bin/bash
# S8-D9: Disaster Recovery - Backup Script
# Usage: ./backup.sh [namespace]

set -euo pipefail

NAMESPACE="${1:-investor-os}"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
BACKUP_BUCKET="gs://investor-os-backups"
RETENTION_DAYS=30

echo "========================================="
echo "Investor OS Backup - ${TIMESTAMP}"
echo "Namespace: ${NAMESPACE}"
echo "========================================="

# Create backup directory
BACKUP_DIR="/tmp/backup_${TIMESTAMP}"
mkdir -p "${BACKUP_DIR}"

# ==================== PostgreSQL Backup ====================
echo "[1/3] Backing up PostgreSQL..."

# Get PostgreSQL pod name
POSTGRES_POD=$(kubectl get pod -n "${NAMESPACE}" -l app=postgres -o jsonpath='{.items[0].metadata.name}')

# Create database dump
kubectl exec -n "${NAMESPACE}" "${POSTGRES_POD}" -- \
    pg_dump -U investor investor_os | \
    gzip > "${BACKUP_DIR}/db_backup_${TIMESTAMP}.sql.gz"

# Verify backup
if [ -s "${BACKUP_DIR}/db_backup_${TIMESTAMP}.sql.gz" ]; then
    echo "✓ Database backup successful ($(du -h ${BACKUP_DIR}/db_backup_${TIMESTAMP}.sql.gz | cut -f1))"
else
    echo "✗ Database backup failed"
    exit 1
fi

# ==================== Redis Backup ====================
echo "[2/3] Backing up Redis..."

REDIS_POD=$(kubectl get pod -n "${NAMESPACE}" -l app=redis -o jsonpath='{.items[0].metadata.name}')

# Trigger BGSAVE
kubectl exec -n "${NAMESPACE}" "${REDIS_POD}" -- redis-cli BGSAVE

# Wait for save to complete
sleep 5

# Copy RDB file
kubectl cp "${NAMESPACE}/${REDIS_POD}:/data/dump.rdb" "${BACKUP_DIR}/redis_backup_${TIMESTAMP}.rdb"

echo "✓ Redis backup successful"

# ==================== Upload to Cloud Storage ====================
echo "[3/3] Uploading to GCS..."

# Upload database
gsutil cp "${BACKUP_DIR}/db_backup_${TIMESTAMP}.sql.gz" "${BACKUP_BUCKET}/database/"

# Upload Redis
gsutil cp "${BACKUP_DIR}/redis_backup_${TIMESTAMP}.rdb" "${BACKUP_BUCKET}/redis/"

# Cleanup local files
rm -rf "${BACKUP_DIR}"

# ==================== Cleanup Old Backups ====================
echo "Cleaning up backups older than ${RETENTION_DAYS} days..."

gsutil ls -r "${BACKUP_BUCKET}/database/" | \
    while read file; do
        # Extract date from filename
        if [[ $file =~ ([0-9]{8}_[0-9]{6}) ]]; then
            FILE_DATE="${BASH_REMATCH[1]}"
            FILE_TIMESTAMP=$(date -d "${FILE_DATE:0:8} ${FILE_DATE:9:2}:${FILE_DATE:11:2}:${FILE_DATE:13:2}" +%s 2>/dev/null || echo 0)
            CURRENT_TIMESTAMP=$(date +%s)
            AGE_DAYS=$(( (CURRENT_TIMESTAMP - FILE_TIMESTAMP) / 86400 ))
            
            if [ $AGE_DAYS -gt $RETENTION_DAYS ]; then
                echo "Deleting old backup: $file"
                gsutil rm "$file"
            fi
        fi
    done

echo "========================================="
echo "Backup complete: ${TIMESTAMP}"
echo "========================================="
