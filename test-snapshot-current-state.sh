#!/bin/bash
# Test snapshot load with the current golden snapshot
set -e

echo "========================================="
echo "Test: Load Current Golden Snapshot"
echo "========================================="

# Cleanup
sudo pkill -f "firecracker.*test-snapshot" || true
sleep 1
sudo rm -f /tmp/test-snapshot.sock

# Check if golden snapshot exists
if [ ! -f /srv/snapshots/node-golden-vm.snap ]; then
    echo "❌ Golden snapshot not found at /srv/snapshots/node-golden-vm.snap"
    echo "You need to create it first!"
    exit 1
fi

if [ ! -f /srv/snapshots/node-golden-mem.snap ]; then
    echo "❌ Memory snapshot not found at /srv/snapshots/node-golden-mem.snap"
    exit 1
fi

if [ ! -f /srv/images/node-runtime.ext4 ]; then
    echo "❌ Base rootfs not found at /srv/images/node-runtime.ext4"
    exit 1
fi

echo "✅ All snapshot files exist"
echo ""

# Start firecracker
echo "Starting fresh Firecracker instance..."
sudo firecracker --api-sock /tmp/test-snapshot.sock &
FC_PID=$!
echo "Firecracker PID: $FC_PID"

# Wait for socket
for i in {1..50}; do
    if [ -S /tmp/test-snapshot.sock ]; then
        break
    fi
    sleep 0.1
done

if [ ! -S /tmp/test-snapshot.sock ]; then
    echo "❌ Socket not created"
    exit 1
fi

sudo chmod 666 /tmp/test-snapshot.sock
echo "✅ Socket ready"
echo ""

echo "Attempting to load golden snapshot..."
echo "Snapshot VM: /srv/snapshots/node-golden-vm.snap"
echo "Snapshot Mem: /srv/snapshots/node-golden-mem.snap"
echo ""

RESPONSE=$(curl -s -w "\nHTTP_CODE:%{http_code}" --unix-socket /tmp/test-snapshot.sock \
  -H "Content-Type: application/json" \
  -d '{
    "snapshot_path": "/srv/snapshots/node-golden-vm.snap",
    "mem_file_path": "/srv/snapshots/node-golden-mem.snap",
    "enable_diff_snapshots": false,
    "resume_vm": false
  }' \
  -XPUT http://dummy/snapshot/load 2>&1)

HTTP_CODE=$(echo "$RESPONSE" | grep "HTTP_CODE" | cut -d: -f2)
BODY=$(echo "$RESPONSE" | grep -v "HTTP_CODE")

echo "HTTP Status: $HTTP_CODE"
echo ""

if [ "$HTTP_CODE" = "204" ]; then
    echo "✅ SUCCESS! Snapshot loaded"
    echo ""
    echo "Now trying to start the VM..."

    START_RESPONSE=$(curl -s -w "\nHTTP_CODE:%{http_code}" --unix-socket /tmp/test-snapshot.sock \
      -H "Content-Type: application/json" \
      -d '{"action_type": "InstanceStart"}' \
      -XPUT http://dummy/actions 2>&1)

    START_CODE=$(echo "$START_RESPONSE" | grep "HTTP_CODE" | cut -d: -f2)
    START_BODY=$(echo "$START_RESPONSE" | grep -v "HTTP_CODE")

    echo "Start VM HTTP Status: $START_CODE"
    if [ "$START_CODE" = "204" ]; then
        echo "✅ VM started successfully!"
        echo ""
        echo "The snapshot restore is working correctly!"
    else
        echo "❌ Failed to start VM"
        echo "Response: $START_BODY"
    fi
else
    echo "❌ FAILED to load snapshot"
    echo ""
    echo "Response Body:"
    echo "$BODY" | jq '.' 2>/dev/null || echo "$BODY"
    echo ""
    echo "This tells us why Firecracker rejected the snapshot."
    echo ""
    echo "Common causes:"
    echo "1. Rootfs was modified after snapshot creation"
    echo "2. Snapshot was created with different Firecracker version"
    echo "3. Snapshot file is corrupted"
    echo "4. Snapshot CPU vendor mismatch"
fi

# Cleanup
echo ""
echo "Cleaning up..."
sudo kill $FC_PID 2>/dev/null || true
sleep 1
sudo rm -f /tmp/test-snapshot.sock

echo "========================================="
