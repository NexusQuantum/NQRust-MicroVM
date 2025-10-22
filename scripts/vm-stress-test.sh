#!/bin/sh
# VM stress test for Alpine Linux - generates CPU, memory, disk, and network activity

echo "Starting comprehensive VM stress test..."
echo "Press Ctrl+C to stop"
echo ""

cleanup() {
    echo ""
    echo "Stopping all background processes..."
    killall dd yes cat 2>/dev/null
    rm -f /tmp/stress-* 2>/dev/null
    echo "Done!"
    exit 0
}

trap cleanup INT TERM

iteration=0

while true; do
    iteration=$((iteration + 1))
    echo "Iteration $iteration..."

    # CPU stress - multiple concurrent processes
    yes > /dev/null &
    YES_PID=$!

    cat /dev/zero > /dev/null &
    CAT_PID=$!

    # Let CPU burners run for 0.8 seconds
    sleep 0.8

    # Stop CPU burners
    kill $YES_PID $CAT_PID 2>/dev/null

    # Disk I/O - write and read 20MB
    dd if=/dev/zero of=/tmp/stress-$iteration.dat bs=1M count=20 2>/dev/null
    dd if=/tmp/stress-$iteration.dat of=/dev/null bs=1M 2>/dev/null
    sync
    rm -f /tmp/stress-$iteration.dat

    # Network activity
    ping -c 5 -W 1 8.8.8.8 >/dev/null 2>&1 &

    # Brief pause before next iteration
    sleep 0.5
done
