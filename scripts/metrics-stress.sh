#!/bin/sh
# Simple metrics stress test script for Alpine Linux VMs
# This generates activity across CPU, network, and disk I/O

echo "Starting metrics stress test..."
echo "Press Ctrl+C to stop"
echo ""

# Create a temp directory for disk operations
WORKDIR="/tmp/metrics-test"
mkdir -p "$WORKDIR"

# Cleanup on exit
cleanup() {
    echo ""
    echo "Cleaning up..."
    rm -rf "$WORKDIR"
    echo "Done!"
    exit 0
}

trap cleanup INT TERM

iteration=0

while true; do
    iteration=$((iteration + 1))
    echo "Iteration $iteration..."

    # CPU stress: Simple calculation loop
    i=0
    while [ $i -lt 10000 ]; do
        result=$((i * i + i / 2))
        i=$((i + 1))
    done

    # Disk I/O: Write and read files
    dd if=/dev/zero of="$WORKDIR/test-$iteration.dat" bs=1M count=5 2>/dev/null
    dd if="$WORKDIR/test-$iteration.dat" of=/dev/null bs=1M 2>/dev/null
    rm -f "$WORKDIR/test-$iteration.dat"

    # Network activity: Ping Google DNS
    ping -c 3 8.8.8.8 >/dev/null 2>&1

    # Small delay between iterations
    sleep 1
done
