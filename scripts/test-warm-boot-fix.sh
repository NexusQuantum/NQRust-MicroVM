#!/usr/bin/env bash
set -euo pipefail

MANAGER_URL="${MANAGER_URL:-http://127.0.0.1:18080}"

echo "=== Testing Warm Boot Fix ==="
echo ""

echo "Step 1: Delete existing runtime snapshots..."
SNAPSHOTS=$(curl -s "$MANAGER_URL/v1/runtime-snapshots" | jq -r '.items[].id')

if [ -z "$SNAPSHOTS" ]; then
    echo "  No snapshots to delete"
else
    for snap_id in $SNAPSHOTS; do
        echo "  Deleting snapshot: $snap_id"
        curl -s -X DELETE "$MANAGER_URL/v1/runtime-snapshots/$snap_id" > /dev/null
    done
    echo "  ✓ All snapshots deleted"
fi

echo ""
echo "Step 2: Create new runtime snapshot..."
echo "  (This will take 60-120 seconds)"
echo ""

./scripts/create-runtime-snapshot.sh

echo ""
echo "Step 3: Test container creation with warm boot..."
echo ""

time curl -X POST "$MANAGER_URL/v1/containers" \
  -H "Content-Type: application/json" \
  -d '{"name": "test-warm-boot-fixed", "image": "nginx:alpine"}'

echo ""
echo ""
echo "=== Test Complete ==="
echo "Check manager logs for detailed output"
