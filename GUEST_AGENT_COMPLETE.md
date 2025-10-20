# Guest Agent Integration - Complete Guide

## Overview

The guest agent is now **fully integrated** with the manager! Here's what was implemented:

### ✅ What's Done

1. **Rust Guest Agent** (`apps/guest-agent/`)
   - Lightweight HTTP server (2.2MB)
   - Reads real CPU% from `/proc/stat`
   - Reads real memory% from `/proc/meminfo`
   - Exposes `/metrics` and `/health` endpoints

2. **Database Schema**
   - Added `guest_ip` column to `vm` table
   - Migration: `0011_guest_ip.sql`

3. **Manager Integration**
   - Modified `get_process_stats()` to query guest agent if `guest_ip` is set
   - Falls back to host-side process stats if guest agent unavailable
   - Added `/v1/vms/{id}/guest-ip` endpoint to register VM IP
   - Automatic detection of guest agent during metrics collection

4. **Deployment Tools**
   - `scripts/deploy-guest-agent.sh` - Interactive deployment script
   - `GUEST_AGENT_SETUP.md` - Comprehensive manual
   - `apps/guest-agent/README.md` - API documentation

## Quick Start

### 1. Run Database Migration

```bash
cd apps/manager
sqlx migrate run
```

### 2. Build Guest Agent

```bash
cargo build --release --bin guest-agent --target x86_64-unknown-linux-musl
```

### 3. Deploy to VM

**Option A: Using deployment script**

```bash
./scripts/deploy-guest-agent.sh <vm-id>
```

Follow the interactive prompts.

**Option B: Manual deployment**

```bash
# On host - serve the binary
cd target/x86_64-unknown-linux-musl/release
python3 -m http.server 8000
```

**In VM terminal (via manager UI shell):**

```bash
# Get host IP (gateway)
GATEWAY=$(ip route | grep default | awk '{print $3}')

# Download guest agent
wget http://$GATEWAY:8000/guest-agent -O /usr/local/bin/guest-agent
chmod +x /usr/local/bin/guest-agent

# Create auto-start service
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

# Verify it's running
curl http://localhost:8080/health
curl http://localhost:8080/metrics
```

### 4. Register Guest IP with Manager

**Get VM IP (run in VM):**
```bash
MY_IP=$(ip addr show eth0 | grep 'inet ' | awk '{print $2}' | cut -d/ -f1)
echo $MY_IP
```

**Register IP (run on host):**
```bash
curl -X POST http://localhost:8080/v1/vms/<VM_ID>/guest-ip \
  -H 'Content-Type: application/json' \
  -d '{"guest_ip": "<VM_IP>"}'
```

Replace `<VM_ID>` with your VM's UUID and `<VM_IP>` with the IP from the VM.

### 5. Watch Real Metrics!

Open the metrics tab in your manager UI. You should now see:
- ✅ **Real CPU usage** from inside the guest
- ✅ **Real memory usage** from inside the guest
- ✅ **Network I/O** from Firecracker
- ✅ **Disk I/O** from Firecracker

## How It Works

```
┌─────────────────────────────────────────┐
│          VM (Guest)                     │
│                                         │
│  ┌────────────────────────────────┐    │
│  │  Guest Agent :8080              │    │
│  │  GET /metrics                   │    │
│  │  {                              │    │
│  │    cpu_usage_percent: 45.2,     │    │
│  │    memory_usage_percent: 62.5   │    │
│  │  }                              │    │
│  └────────────────────────────────┘    │
│              ▲                          │
└──────────────┼──────────────────────────┘
               │ HTTP GET
        ┌──────▼────────┐
        │    Manager    │
        │               │
        │ 1. Checks     │
        │    guest_ip   │
        │ 2. Queries    │
        │    guest agent│
        │ 3. Combines   │
        │    with FC    │
        │    metrics    │
        └───────────────┘
```

**Metrics Flow:**

1. Manager checks if VM has `guest_ip` set
2. If yes: Query `http://{guest_ip}:8080/metrics` (guest agent)
3. If no/fail: Query agent's `/metrics/process-stats` (host-side fallback)
4. Combine with Firecracker metrics (network/disk from FIFO)
5. Send to frontend via WebSocket

## Testing

**Run stress test in VM:**
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

**Expected Results:**
- CPU: 40-90% (real guest CPU usage!)
- Memory: Increases as processes run
- Disk: ~20-40 MB/sec write + read
- Network: ~500-1500 bytes/sec (ping traffic)

## Troubleshooting

**Guest agent not starting:**
```bash
# Run manually to see errors
/usr/local/bin/guest-agent

# Check logs
tail -f /var/log/guest-agent.log
```

**Manager can't connect:**
```bash
# In VM - verify agent is listening
netstat -tlnp | grep 8080

# Test from VM
curl http://localhost:8080/metrics

# Get VM IP
ip addr show eth0 | grep inet
```

**Metrics still showing host-side values:**
```bash
# Check if guest_ip is set in database
psql $DATABASE_URL -c "SELECT id, name, guest_ip FROM vm WHERE id = '<VM_ID>';"

# If NULL, register it:
curl -X POST http://localhost:8080/v1/vms/<VM_ID>/guest-ip \
  -H 'Content-Type: application/json' \
  -d '{"guest_ip": "<VM_IP>"}'
```

## API Endpoints

### Guest Agent (runs in VM)

- `GET http://{vm_ip}:8080/metrics` - Get current metrics
- `GET http://{vm_ip}:8080/health` - Health check

### Manager

- `POST /v1/vms/{id}/guest-ip` - Register guest IP address
  ```json
  {
    "guest_ip": "192.168.1.100"
  }
  ```

## Files Changed

- `apps/guest-agent/` - New guest agent crate
- `apps/manager/migrations/0011_guest_ip.sql` - Database migration
- `apps/manager/src/features/vms/repo.rs` - Added guest_ip field
- `apps/manager/src/features/vms/service.rs` - Guest agent integration
- `apps/manager/src/features/vms/routes.rs` - New guest-ip endpoint
- `scripts/deploy-guest-agent.sh` - Deployment helper

## Next Steps

1. Deploy guest agent to all VMs
2. Register guest IPs with manager
3. Enjoy real CPU and memory metrics! 🎉

For future VMs, consider adding auto-deployment during VM creation via cloud-init or startup scripts.
