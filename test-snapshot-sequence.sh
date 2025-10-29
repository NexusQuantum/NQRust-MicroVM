#!/bin/bash
# Test script to find the correct Firecracker snapshot restore sequence
set -e

echo "========================================="
echo "Firecracker Snapshot Load Sequence Test"
echo "========================================="

# Cleanup from any previous test
sudo pkill -f "firecracker.*test-snapshot" || true
sleep 1
sudo rm -f /tmp/test-snapshot.sock

# Test 1: Snapshot load with NO prior configuration (correct order per API docs)
echo ""
echo "Test 1: Load snapshot immediately after starting Firecracker"
echo "-------------------------------------------------------------"

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

if [ ! -S /tmp/test-snapshot.sock ]; then
    echo "ERROR: Socket not created"
    sudo kill $FC_PID || true
    exit 1
fi

# Make socket accessible
sudo chmod 666 /tmp/test-snapshot.sock

echo "Socket ready, attempting snapshot load WITHOUT any prior config..."

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

echo "HTTP Status: $HTTP_CODE"
echo "Response Body: $BODY"

if [ "$HTTP_CODE" = "204" ]; then
    echo "✅ Test 1 PASSED: Snapshot loaded without prior config"
else
    echo "❌ Test 1 FAILED: Got HTTP $HTTP_CODE"
    echo "This means we need to configure something before snapshot load"
fi

# Cleanup
sudo kill $FC_PID || true
sleep 1
sudo rm -f /tmp/test-snapshot.sock

# Test 2: Snapshot load with machine-config first
echo ""
echo "Test 2: Machine config → Snapshot load"
echo "---------------------------------------"

sudo firecracker --api-sock /tmp/test-snapshot.sock &
FC_PID=$!

for i in {1..50}; do
    if [ -S /tmp/test-snapshot.sock ]; then
        break
    fi
    sleep 0.1
done

sudo chmod 666 /tmp/test-snapshot.sock

# Machine config
curl -s --unix-socket /tmp/test-snapshot.sock \
  -H "Content-Type: application/json" \
  -d '{"vcpu_count":1,"mem_size_mib":512,"smt":false}' \
  -XPUT http://dummy/machine-config
echo "Machine config set"

# Snapshot load
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

echo "HTTP Status: $HTTP_CODE"
echo "Response Body: $BODY"

if [ "$HTTP_CODE" = "204" ]; then
    echo "✅ Test 2 PASSED: Snapshot loaded after machine-config"
else
    echo "❌ Test 2 FAILED: Machine-config before snapshot doesn't work"
fi

sudo kill $FC_PID || true
sleep 1
rm -f /tmp/test-snapshot.sock

# Test 3: Snapshot load with drives first
echo ""
echo "Test 3: Drive config → Snapshot load"
echo "-------------------------------------"

sudo firecracker --api-sock /tmp/test-snapshot.sock &
FC_PID=$!

for i in {1..50}; do
    if [ -S /tmp/test-snapshot.sock ]; then
        break
    fi
    sleep 0.1
done

sudo chmod 666 /tmp/test-snapshot.sock

# Drive config
curl -s --unix-socket /tmp/test-snapshot.sock \
  -H "Content-Type: application/json" \
  -d '{
    "drive_id": "rootfs",
    "path_on_host": "/srv/images/node-runtime.ext4",
    "is_root_device": true,
    "is_read_only": false
  }' \
  -XPUT http://dummy/drives/rootfs
echo "Drive attached"

# Snapshot load
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

echo "HTTP Status: $HTTP_CODE"
echo "Response Body: $BODY"

if [ "$HTTP_CODE" = "204" ]; then
    echo "✅ Test 3 PASSED: Snapshot loaded after drive config"
else
    echo "❌ Test 3 FAILED: Drive config before snapshot doesn't work"
fi

sudo kill $FC_PID || true
sleep 1
rm -f /tmp/test-snapshot.sock

# Test 4: The full sequence (machine + drives + snapshot)
echo ""
echo "Test 4: Machine config → Drives → Snapshot load"
echo "------------------------------------------------"

sudo firecracker --api-sock /tmp/test-snapshot.sock &
FC_PID=$!

for i in {1..50}; do
    if [ -S /tmp/test-snapshot.sock ]; then
        break
    fi
    sleep 0.1
done

sudo chmod 666 /tmp/test-snapshot.sock

# Machine config
curl -s --unix-socket /tmp/test-snapshot.sock \
  -H "Content-Type: application/json" \
  -d '{"vcpu_count":1,"mem_size_mib":512,"smt":false}' \
  -XPUT http://dummy/machine-config
echo "Machine config set"

# Drive
curl -s --unix-socket /tmp/test-snapshot.sock \
  -H "Content-Type: application/json" \
  -d '{
    "drive_id": "rootfs",
    "path_on_host": "/srv/images/node-runtime.ext4",
    "is_root_device": true,
    "is_read_only": false
  }' \
  -XPUT http://dummy/drives/rootfs
echo "Drive attached"

# Snapshot load
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

echo "HTTP Status: $HTTP_CODE"
echo "Response Body: $BODY"

if [ "$HTTP_CODE" = "204" ]; then
    echo "✅ Test 4 PASSED: Full sequence works"
else
    echo "❌ Test 4 FAILED: Full sequence doesn't work"
fi

sudo kill $FC_PID || true
sleep 1
rm -f /tmp/test-snapshot.sock

echo ""
echo "========================================="
echo "Summary"
echo "========================================="
echo "This test reveals which configuration sequence Firecracker accepts"
echo "for snapshot restore operations."
