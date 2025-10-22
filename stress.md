sh >> 'EOF'
#!/bin/sh

# Ultimate VM Stress Test for Guest Agent Metrics
# This will create high CPU, memory, disk I/O, and network load

echo "🔥 Starting Ultimate VM Stress Test 🔥"
echo "This will MAX OUT all metrics for testing"
echo "Press Ctrl+C to stop"
echo

# Function to cleanup on exit
cleanup() {
    echo "🧹 Cleaning up stress processes..."
    killall sh 2>/dev/null
    killall dd 2>/dev/null
    killall ping 2>/dev/null
    killall yes 2>/dev/null
    rm -f /tmp/stress-* /tmp/network-test /tmp/cpu-load /tmp/mem-load
    echo "✅ Cleanup complete"
    exit 0
}

trap cleanup INT TERM

echo "💪 Starting maximum stress tests..."

# ===== CPU STRESS (Multiple methods) =====
echo "🔥 CPU Stress: Starting CPU-intensive loops..."

# Method 1: Infinite loops (max CPU)
for i in 1 2 3 4; do
    (
        while true; do
            # CPU-intensive math operations
            i=0
            while [ $i -lt 100000 ]; do
                i=$((i + 1))
                echo $i > /dev/null
            done
        done
    ) &
done

# Method 2: 'yes' command if available
command -v yes >/dev/null 2>&1 && yes > /dev/null &

# Method 3: Fork bombs (controlled)
for i in 1 2 3; do
    (
        while true; do
            sh -c 'while true; do echo $$ > /dev/null; done' &
            sleep 0.1
        done
    ) &
done

# ===== MEMORY STRESS =====
echo "💾 Memory Stress: Allocating memory..."

# Method 1: Create large variables
for i in 1 2 3 4; do
    (
        while true; do
            # Create large data chunks
            BIG_DATA=$(dd if=/dev/zero bs=1024 count=2048 2>/dev/null | base64)
            echo "$BIG_DATA" > /tmp/mem-load-$i
            sleep 0.2
        done
    ) &
done

# Method 2: Memory allocation with dd
(
    while true; do
        dd if=/dev/zero of=/tmp/mem-stress bs=1M count=10 2>/dev/null
        dd if=/tmp/mem-stress of=/dev/null bs=1M count=10 2>/dev/null
        rm -f /tmp/mem-stress
        sleep 0.5
    done
) &

# ===== DISK I/O STRESS =====
echo "💿 Disk I/O Stress: Read/write operations..."

# Method 1: Continuous read/write
(
    while true; do
        # Write operations
        dd if=/dev/zero of=/tmp/stress-write bs=1M count=50 2>/dev/null
        sync
        # Read operations
        dd if=/tmp/stress-write of=/dev/null bs=1M count=50 2>/dev/null
        # Random I/O
        dd if=/dev/zero of=/tmp/stress-random bs=4k count=1000 2>/dev/null
        dd if=/tmp/stress-random of=/dev/null bs=4k count=1000 2>/dev/null
        sleep 0.3
    done
) &

# Method 2: Multiple parallel I/O
for i in 1 2 3; do
    (
        while true; do
            dd if=/dev/zero of=/tmp/io-test-$i bs=512k count=100 2>/dev/null
            dd if=/tmp/io-test-$i of=/dev/null bs=512k count=100 2>/dev/null
            sleep 1
        done
    ) &
done

# ===== NETWORK STRESS =====
echo "🌐 Network Stress: Generating network traffic..."

# Method 1: Continuous pings
(
    while true; do
        ping -c 5 8.8.8.8 > /dev/null 2>&1
        ping -c 5 1.1.1.1 > /dev/null 2>&1
        sleep 0.5
    done
) &

# Method 2: Local network loops
(
    while true; do
        # Generate network traffic
        echo "network-stress-data-$(date)" | nc -l 9999 >/dev/null 2>&1 &
        sleep 0.1
        echo "test-data-$(date)" | nc 127.0.0.1 9999 >/dev/null 2>&1
        sleep 0.2
    done
) &

# Method 3: Create network connections
for i in 1 2 3; do
    (
        while true; do
            # Try to connect to various services
            nc -zv 8.8.8.8 53 >/dev/null 2>&1
            nc -zv 1.1.1.1 53 >/dev/null 2>&1
            sleep 2
        done
    ) &
done

# ===== PROCESS STRESS =====
echo "⚙️  Process Stress: Creating processes..."

# Method 1: Rapid process creation
(
    while true; do
        # Create many short-lived processes
        for i in 1 2 3 4 5; do
            (ps aux > /dev/null 2>&1; ls /proc > /dev/null 2>&1) &
        done
        sleep 0.3
    done
) &

# Method 2: Fork processes
(
    while true; do
        sh -c 'exit 0' &
        sh -c 'echo $$ > /dev/null; exit 0' &
        sleep 0.1
    done
) &

echo
echo "🚀 ALL STRESS TESTS STARTED!"
echo "📊 Expected metrics:"
echo "   🔥 CPU: 80-100% usage"
echo "   💾 Memory: +50-100MB usage"
echo "   💿 Disk: High read/write activity"
echo "   🌐 Network: Continuous traffic"
echo "   ⚙️  Processes: High process count"
echo
echo "🔥 Stress test running... Press Ctrl+C to stop 🔥"
echo

# Keep the script running and show status
while true; do
    echo "[$(date)] Stress test active - $(ps aux | wc -l) processes running"
    sleep 5
done
EOF