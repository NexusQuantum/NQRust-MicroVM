#!/bin/bash
# Test if drives can be attached AFTER snapshot load
set -e

echo "========================================="
echo "Test: Snapshot Load → Drive Attachment"
echo "========================================="

# Cleanup
sudo pkill -f "firecracker.*test-snapshot" || true
sleep 1
sudo rm -f /tmp/test-snapshot.sock

# Start firecracker
sudo firecracker --api-sock /tmp/test-snapshot.sock &
FC_PID=$!
echo "Started Firecracker (PID: $FC_PID)"

# Wait for socket
for i in {1..50}; do
    if [ -S /tmp/test-snapshot.sock ]; then
        break
    fi
    sleep 0.1
done

sudo chmod 666 /tmp/test-snapshot.sock

echo ""
echo "Step 1: Load snapshot"
echo "---------------------"
RESPONSE=$(curl -s -w "\nHTTP_CODE:%{http_code}" --unix-socket /tmp/test-snapshot.sock \
  -H "Content-Type: application/json" \
  -d '{
    "snapshot_path": "/srv/snapshots/node-golden-vm.snap",
    "mem_file_path": "/srv/snapshots/node-golden-mem.snap",
    "enable_diff_snapshots": false,
    "resume_vm": false
  }' \
  -XPUT http://dummy/snapshot/load)

HTTP_CODE=$(echo "$RESPONSE" | grep "HTTP_CODE" | cut -d: -f2)
BODY=$(echo "$RESPONSE" | grep -v "HTTP_CODE")

echo "Snapshot load - HTTP Status: $HTTP_CODE"
if [ "$HTTP_CODE" != "204" ]; then
    echo "❌ Snapshot load failed!"
    echo "Response: $BODY"
    sudo kill $FC_PID || true
    sudo rm -f /tmp/test-snapshot.sock
    exit 1
fi
echo "✅ Snapshot loaded successfully"

echo ""
echo "Step 2: Attach drive AFTER snapshot load"
echo "-----------------------------------------"
RESPONSE=$(curl -s -w "\nHTTP_CODE:%{http_code}" --unix-socket /tmp/test-snapshot.sock \
  -H "Content-Type: application/json" \
  -d '{
    "drive_id": "rootfs",
    "path_on_host": "/srv/images/node-runtime.ext4",
    "is_root_device": true,
    "is_read_only": false
  }' \
  -XPUT http://dummy/drives/rootfs)

HTTP_CODE=$(echo "$RESPONSE" | grep "HTTP_CODE" | cut -d: -f2)
BODY=$(echo "$RESPONSE" | grep -v "HTTP_CODE")

echo "Drive attachment - HTTP Status: $HTTP_CODE"
echo "Response Body: $BODY"

if [ "$HTTP_CODE" = "204" ]; then
    echo "✅ Drive attached successfully after snapshot!"
    echo ""
    echo "This means snapshots DON'T include drive configuration."
    echo "Drives must be attached AFTER snapshot load."
else
    echo "❌ Cannot attach drive after snapshot (HTTP $HTTP_CODE)"
    echo ""
    echo "This means snapshots include drive configuration."
    echo "We need a different approach..."
fi

# Cleanup
sudo kill $FC_PID || true
sleep 1
sudo rm -f /tmp/test-snapshot.sock

echo ""
echo "========================================="
