#!/bin/bash
# Test snapshot load with network override
set -e

echo "========================================="
echo "Test: Snapshot Load with Network Override"
echo "========================================="

# Cleanup
sudo pkill -f "firecracker.*test-snapshot" || true
sleep 1
sudo rm -f /tmp/test-snapshot.sock

# Create a test tap device
TEST_TAP="tap-testsnap"
echo "Creating test tap device: $TEST_TAP"
sudo ip tuntap add $TEST_TAP mode tap 2>/dev/null || echo "Tap already exists or failed to create"
sudo ip link set $TEST_TAP up
sudo ip addr add 192.168.100.1/24 dev $TEST_TAP 2>/dev/null || true

echo "✅ Test tap device ready: $TEST_TAP"
echo ""

# Start firecracker
echo "Starting Firecracker..."
sudo firecracker --api-sock /tmp/test-snapshot.sock &
FC_PID=$!

# Wait for socket
for i in {1..50}; do
    if [ -S /tmp/test-snapshot.sock ]; then
        break
    fi
    sleep 0.1
done

sudo chmod 666 /tmp/test-snapshot.sock
echo "✅ Firecracker ready"
echo ""

echo "Loading snapshot with network override..."
echo "  Old tap (in snapshot): tap-d25f70df (doesn't exist)"
echo "  New tap (override): $TEST_TAP"
echo ""

RESPONSE=$(curl -s -w "\nHTTP_CODE:%{http_code}" --unix-socket /tmp/test-snapshot.sock \
  -H "Content-Type: application/json" \
  -d "{
    \"snapshot_path\": \"/srv/snapshots/node-golden-vm.snap\",
    \"mem_file_path\": \"/srv/snapshots/node-golden-mem.snap\",
    \"enable_diff_snapshots\": false,
    \"resume_vm\": false,
    \"network_overrides\": [
      {
        \"iface_id\": \"eth0\",
        \"host_dev_name\": \"$TEST_TAP\"
      }
    ]
  }" \
  -XPUT http://dummy/snapshot/load 2>&1)

HTTP_CODE=$(echo "$RESPONSE" | grep "HTTP_CODE" | cut -d: -f2)
BODY=$(echo "$RESPONSE" | grep -v "HTTP_CODE")

echo "HTTP Status: $HTTP_CODE"
echo ""

if [ "$HTTP_CODE" = "204" ]; then
    echo "✅ SUCCESS! Snapshot loaded with network override"
    echo ""
    echo "Now trying to start the VM..."

    START_RESPONSE=$(curl -s -w "\nHTTP_CODE:%{http_code}" --unix-socket /tmp/test-snapshot.sock \
      -H "Content-Type: application/json" \
      -d '{"action_type": "InstanceStart"}' \
      -XPUT http://dummy/actions 2>&1)

    START_CODE=$(echo "$START_RESPONSE" | grep "HTTP_CODE" | cut -d: -f2)

    echo "Start VM HTTP Status: $START_CODE"
    if [ "$START_CODE" = "204" ]; then
        echo "✅ VM started successfully!"
        echo ""
        echo "🎉 Snapshot restore with network override is working!"
    else
        echo "❌ Failed to start VM"
        START_BODY=$(echo "$START_RESPONSE" | grep -v "HTTP_CODE")
        echo "Response: $START_BODY"
    fi
else
    echo "❌ FAILED to load snapshot"
    echo ""
    echo "Response Body:"
    echo "$BODY" | jq '.' 2>/dev/null || echo "$BODY"
fi

# Cleanup
echo ""
echo "Cleaning up..."
sudo kill $FC_PID 2>/dev/null || true
sleep 1
sudo rm -f /tmp/test-snapshot.sock
sudo ip link set $TEST_TAP down 2>/dev/null || true
sudo ip tuntap del $TEST_TAP mode tap 2>/dev/null || true

echo "========================================="
