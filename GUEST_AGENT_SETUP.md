# Guest Agent Setup Guide

This guide explains how to deploy and use the Rust guest agent for real CPU and memory metrics.

## Overview

The guest agent runs **inside each VM** and reports real guest metrics (CPU%, memory%) to the manager. The manager combines these with Firecracker's network/disk metrics to provide complete monitoring.

## Architecture

```
┌─────────────────────────────────────────┐
│          VM (Alpine Linux)              │
│                                         │
│  ┌────────────────────────────────┐    │
│  │  Guest Agent                    │    │
│  │  - HTTP Server on :8080         │    │
│  │  - Reads /proc/stat (CPU)       │    │
│  │  - Reads /proc/meminfo (Memory) │    │
│  └────────────────────────────────┘    │
│              ▲                          │
│              │ GET /metrics             │
└──────────────┼──────────────────────────┘
               │
        ┌──────▼────────┐
        │    Manager    │
        │ (queries VM   │
        │  via network) │
        └───────────────┘
```

## Step 1: Build the Guest Agent

```bash
cd /home/shiro/nexus/nqrust-microvm
cargo build --release --bin guest-agent --target x86_64-unknown-linux-musl
```

Binary location: `target/x86_64-unknown-linux-musl/release/guest-agent` (2.2MB)

## Step 2: Deploy to VM

### Option A: HTTP Transfer (Easiest)

**On host:**
```bash
cd target/x86_64-unknown-linux-musl/release
python3 -m http.server 8000
```

**In VM terminal (via manager UI):**
```bash
# Get host IP (usually the gateway)
GATEWAY=$(ip route | grep default | awk '{print $3}')

# Download guest agent
wget http://$GATEWAY:8000/guest-agent -O /usr/local/bin/guest-agent
chmod +x /usr/local/bin/guest-agent

# Verify
/usr/local/bin/guest-agent --version
```

### Option B: Base64 Transfer (if no network access)

**On host:**
```bash
base64 target/x86_64-unknown-linux-musl/release/guest-agent > /tmp/guest-agent.b64
cat /tmp/guest-agent.b64
```

**In VM terminal:**
```bash
cat > /tmp/guest-agent.b64 << 'EOF'
<paste base64 content - may need to split into multiple parts>
EOF

base64 -d /tmp/guest-agent.b64 > /usr/local/bin/guest-agent
chmod +x /usr/local/bin/guest-agent
rm /tmp/guest-agent.b64
```

## Step 3: Configure Auto-Start

Create OpenRC service for Alpine Linux:

```bash
cat > /etc/init.d/guest-agent << 'EOF'
#!/sbin/openrc-run

name="guest-agent"
description="Guest metrics agent"
command="/usr/local/bin/guest-agent"
command_background=true
pidfile="/run/${RC_SVCNAME}.pid"
output_log="/var/log/guest-agent.log"
error_log="/var/log/guest-agent.err"

depend() {
    need net
    after firewall
}
EOF

chmod +x /etc/init.d/guest-agent
rc-update add guest-agent default
rc-service guest-agent start
```

Verify it's running:
```bash
rc-service guest-agent status
curl http://localhost:8080/health
```

## Step 4: Configure VM Network (if not already done)

The guest agent requires network connectivity so the manager can query it.

**Check current network:**
```bash
ip addr
ip route
```

**If no network, configure DHCP:**
```bash
# Start networking
rc-service networking start
rc-update add networking default

# Configure eth0 for DHCP
cat > /etc/network/interfaces << 'EOF'
auto lo
iface lo inet loopback

auto eth0
iface eth0 inet dhcp
EOF

# Restart networking
rc-service networking restart
```

**Get VM IP address:**
```bash
ip addr show eth0 | grep 'inet ' | awk '{print $2}' | cut -d/ -f1
```

Note this IP - you'll need it for the manager configuration.

## Step 5: Test Guest Agent

```bash
# Inside VM
curl http://localhost:8080/metrics
```

Should return:
```json
{
  "cpu_usage_percent": 5.2,
  "memory_usage_percent": 45.8,
  "memory_used_kb": 375000,
  "memory_total_kb": 819200,
  "memory_available_kb": 444200,
  "uptime_seconds": 12345
}
```

## Step 6: Update Manager (TODO)

The manager needs to be updated to:
1. Store VM IP addresses in the database
2. Query `http://{vm_ip}:8080/metrics` instead of process stats
3. Combine guest metrics with Firecracker metrics

**Currently, the manager uses host-side process stats.** To switch to guest agent:

1. Add `guest_ip` column to `vm` table
2. Update metrics query to use guest agent endpoint
3. Fall back to process stats if guest agent unavailable

## Testing

**Run stress test inside VM:**
```bash
cat > /tmp/stress.sh << 'EOF'
#!/bin/sh
while true; do
    yes > /dev/null &
    PID=$!
    sleep 0.8
    kill $PID 2>/dev/null
    wait 2>/dev/null

    dd if=/dev/zero of=/tmp/test.dat bs=1M count=20 2>/dev/null
    dd if=/tmp/test.dat of=/dev/null bs=1M 2>/dev/null
    rm -f /tmp/test.dat

    ping -c 5 8.8.8.8 >/dev/null 2>&1 &
    sleep 0.5
done
EOF

chmod +x /tmp/stress.sh
/tmp/stress.sh
```

**Monitor metrics:**
```bash
watch -n1 'curl -s http://localhost:8080/metrics | jq'
```

You should see:
- `cpu_usage_percent` increase to 40-90%
- `memory_usage_percent` increase as processes run
- Real-time updates every second

## Troubleshooting

**Guest agent not starting:**
```bash
/usr/local/bin/guest-agent  # Run manually to see errors
```

**Can't connect from manager:**
```bash
# In VM - test if port is listening
netstat -tlnp | grep 8080

# Check firewall
iptables -L -n
```

**Metrics showing 0:**
```bash
# Check if /proc is mounted
ls -la /proc/stat /proc/meminfo

# Run stress test to generate activity
```

## Next Steps

1. Deploy guest agent to all VMs
2. Update manager to query guest agents
3. Enjoy real CPU and memory metrics! 🎉
