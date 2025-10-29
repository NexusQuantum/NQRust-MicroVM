#!/bin/bash
set -e

RUNTIME="${1:-node}"

echo "=========================================="
echo "Creating Golden Snapshot for $RUNTIME"
echo "=========================================="
echo ""
echo "This will:"
echo "  1. Boot a VM with $RUNTIME runtime"
echo "  2. Wait for runtime server to be ready"
echo "  3. Take a Firecracker snapshot"
echo "  4. Store snapshot in /srv/snapshots/"
echo ""
echo "Future $RUNTIME functions will restore from this snapshot"
echo "Expected time: 30-60 seconds one-time"
echo ""

# Create snapshot directory
sudo mkdir -p /srv/snapshots
sudo chown -R $USER:$USER /srv/snapshots

# Call manager API to create golden snapshot
echo "Calling manager API to create golden snapshot..."
echo ""

curl -X POST "http://localhost:18080/v1/functions/snapshots/$RUNTIME" \
  -H "Content-Type: application/json" \
  -w "\n\nHTTP Status: %{http_code}\n"

echo ""
echo "=========================================="
echo "Golden Snapshot Creation Complete!"
echo "=========================================="
echo ""
echo "Verify snapshots exist:"
echo "  ls -lh /srv/snapshots/"
echo ""
echo "Test with a new function:"
echo "  curl -X POST http://localhost:18080/v1/functions \\"
echo "    -H 'Content-Type: application/json' \\"
echo "    -d '{\"name\": \"test-snapshot\", \"runtime\": \"$RUNTIME\", ...}'"
echo ""
