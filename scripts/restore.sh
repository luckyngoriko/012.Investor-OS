#!/bin/bash
# S8-D9: Disaster Recovery - Restore Script
# Usage: ./restore.sh <backup_file> [namespace]

set -euo pipefail

if [ $# -lt 1 ]; then
    echo "Usage: $0 <backup_file> [namespace]"
    echo "Example: $0 db_backup_20240208_120000.sql.gz investor-os"
    exit 1
fi

BACKUP_FILE="$1"
NAMESPACE="${2:-investor-os}"
BACKUP_BUCKET="gs://investor-os-backups"

echo "========================================="
echo "Investor OS Restore"
echo "Backup: ${BACKUP_FILE}"
echo "Namespace: ${NAMESPACE}"
echo "========================================="

# Confirm restore
read -p "⚠️  This will OVERWRITE the current database. Are you sure? (yes/no): " confirm
if [ "$confirm" != "yes" ]; then
    echo "Restore cancelled"
    exit 0
fi

# Get PostgreSQL pod
POSTGRES_POD=$(kubectl get pod -n "${NAMESPACE}" -l app=postgres -o jsonpath='{.items[0].metadata.name}')

echo ""
echo "[1/4] Scaling down API to prevent writes..."
kubectl scale deployment investor-api --replicas=0 -n "${NAMESPACE}"
sleep 10

echo ""
echo "[2/4] Downloading backup from GCS..."
gsutil cp "${BACKUP_BUCKET}/database/${BACKUP_FILE}" /tmp/

echo ""
echo "[3/4] Restoring database..."

# Extract and restore
if [[ $BACKUP_FILE == *.gz ]]; then
    gunzip -c "/tmp/${BACKUP_FILE}" | \
        kubectl exec -i -n "${NAMESPACE}" "${POSTGRES_POD}" -- \
        psql -U investor -d investor_os
else
    kubectl cp "/tmp/${BACKUP_FILE}" "${NAMESPACE}/${POSTGRES_POD}:/tmp/restore.sql"
    kubectl exec -n "${NAMESPACE}" "${POSTGRES_POD}" -- \
        psql -U investor -d investor_os -f /tmp/restore.sql
fi

echo "✓ Database restored successfully"

# Cleanup
echo ""
echo "[4/4] Cleaning up..."
rm -f "/tmp/${BACKUP_FILE}"
kubectl exec -n "${NAMESPACE}" "${POSTGRES_POD}" -- rm -f /tmp/restore.sql 2>/dev/null || true

echo ""
echo "Scaling API back up..."
kubectl scale deployment investor-api --replicas=3 -n "${NAMESPACE}"

# Wait for rollout
kubectl rollout status deployment/investor-api -n "${NAMESPACE}"

echo ""
echo "========================================="
echo "Restore complete!"
echo "========================================="
echo ""
echo "Verify database integrity:"
echo "  kubectl exec -n ${NAMESPACE} ${POSTGRES_POD} -- psql -U investor -c 'SELECT COUNT(*) FROM signals;'"
