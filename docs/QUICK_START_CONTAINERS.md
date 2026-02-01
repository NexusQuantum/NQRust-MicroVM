# Quick Start: Fast Containers with Runtime Snapshots

Speed up your container creation from **60-120 seconds to 5-15 seconds** using runtime snapshots!

## Overview

Runtime snapshots create pre-warmed Firecracker VMs with Docker daemon already running. When you create a container, it restores from the snapshot instead of cold-booting.

**Benefits:**
- 🚀 **10x faster** container creation
- ⚡ Docker daemon pre-initialized
- 📦 Automatic fallback to cold boot if needed
- 🔄 Self-healing (auto-rebuild on failures)

## Quick Start

### 1. Create Runtime Snapshot (One-Time Setup)

```bash
# Interactive script
./scripts/create-runtime-snapshot.sh

# Or use API directly
curl -X POST http://127.0.0.1:18080/v1/runtime-snapshots \
  -H "Content-Type: application/json" \
  -d '{"runtime_image_id": "your-runtime-image-uuid"}'
```

**This takes 60-120 seconds** (one-time cost). The script will:
1. Find your container runtime image
2. Create a temporary VM with Docker
3. Wait for Docker daemon to start
4. Take a Firecracker snapshot
5. Store the snapshot (~900MB)

### 2. Create Containers (Now Fast!)

```bash
# Create container - uses warm boot automatically!
curl -X POST http://127.0.0.1:18080/v1/containers \
  -H "Content-Type: application/json" \
  -d '{
    "name": "my-nginx",
    "image": "nginx:alpine"
  }'

# Check boot method
curl http://127.0.0.1:18080/v1/containers/{id} | jq .item.boot_method
# Output: "warm"
```

## Performance Comparison

### Before Runtime Snapshots (Cold Boot)
```
VM boot:           3-5s
IP assignment:     5-30s
Docker startup:    60-120s    ← Bottleneck!
Container start:   2-5s
──────────────────────────────
Total:            70-160s
```

### With Runtime Snapshots (Warm Boot)
```
Snapshot restore:  2-3s
Docker ready:      2-5s       ← Already running!
IP assignment:     2-5s
Container start:   2-5s
──────────────────────────────
Total:            8-18s
```

**Time Saved: ~60-140 seconds per container!**

## Management Commands

### List Runtime Snapshots

```bash
curl http://127.0.0.1:18080/v1/runtime-snapshots | jq .
```

### Get Snapshot Details

```bash
curl http://127.0.0.1:18080/v1/runtime-snapshots/{id} | jq .
```

### Rebuild Snapshot

Useful if snapshot becomes unhealthy:

```bash
curl -X POST http://127.0.0.1:18080/v1/runtime-snapshots/{id}/rebuild
```

### Delete Snapshot

```bash
curl -X DELETE http://127.0.0.1:18080/v1/runtime-snapshots/{id}
```

## How It Works

### Snapshot Creation Process

1. **Temporary VM** - Creates VM with container runtime image
2. **Docker Init** - Waits for Docker daemon to start (~60-120s)
3. **Verification** - Checks Docker is healthy and auto-starts
4. **Network Flush** - Clears network config (gets new IP on restore)
5. **Pause VM** - Pauses the VM
6. **Snapshot** - Takes Firecracker memory + disk snapshot
7. **Store** - Saves to `/srv/fc/runtime-snapshots/`
8. **Cleanup** - Destroys temporary VM

### Container Creation with Warm Boot

1. **Check Snapshot** - Looks for ready snapshot for runtime image
2. **Restore** - Restores VM from snapshot (Docker already running!)
3. **Network** - VM gets new IP via DHCP
4. **Verify** - Checks Docker is responding
5. **Track** - Marks container as "warm" boot
6. **Fallback** - If restore fails, falls back to cold boot

### Automatic Fallback

Warm boot will fallback to cold boot if:
- No snapshot exists
- Snapshot is unhealthy
- Snapshot is still creating (after 60s timeout)
- Firecracker version mismatch
- Restore fails
- Docker verification fails

## Health Tracking

Snapshots track their health automatically:

```json
{
  "id": "abc-123",
  "state": "ready",
  "success_count": 42,
  "failure_count": 0,
  "last_used_at": "2026-01-27T10:00:00Z"
}
```

**States:**
- `creating` - Being built
- `ready` - Available for use (containers will use it)
- `unhealthy` - Failed 3+ times, needs rebuild
- `deleted` - Soft deleted, pending cleanup

**Auto-healing:**
- After 3 consecutive failures → marked `unhealthy`
- Unhealthy snapshots → automatic rebuild triggered
- Firecracker version validated before each restore

## Storage

Each runtime snapshot uses approximately:

```
Memory snapshot:    ~512MB (uncompressed)
Disk snapshot:      ~386MB (Alpine + Docker)
Metadata:           <1KB
───────────────────────────────
Total:             ~900MB per snapshot
```

**Location:** `/srv/fc/runtime-snapshots/{runtime-image-id}/`

## Troubleshooting

### Snapshot Creation Fails

```bash
# Check manager logs
journalctl -u nqrust-manager -f | grep snapshot

# Verify runtime image exists
curl http://127.0.0.1:18080/v1/images | jq '.items[] | select(.path | contains("container-runtime"))'

# Check disk space
df -h /srv/fc
```

### Containers Always Use Cold Boot

```bash
# Check if snapshot exists and is ready
curl http://127.0.0.1:18080/v1/runtime-snapshots | jq '.items[] | {id, state, runtime_image_id}'

# Check container boot method
curl http://127.0.0.1:18080/v1/containers/{id} | jq .item.boot_method

# Check manager logs for fallback reason
journalctl -u nqrust-manager -f | grep -i "warm boot\|cold boot"
```

### Snapshot Marked Unhealthy

```bash
# Check failure count
curl http://127.0.0.1:18080/v1/runtime-snapshots/{id} | jq '{state, success_count, failure_count}'

# Rebuild
curl -X POST http://127.0.0.1:18080/v1/runtime-snapshots/{id}/rebuild

# Or delete and recreate
./scripts/create-runtime-snapshot.sh
```

## Prerequisites

### Required

- Container runtime image (`/srv/images/container-runtime.ext4`)
  - Build: `sudo ./scripts/build-container-runtime-v2.sh`
  - Or download: `./scripts/dev-setup-images.sh`

- Manager running: `cd apps/manager && cargo run`

- Agent running with proper FC version

- Sufficient disk space: ~900MB per snapshot

### Optional

- `jq` for JSON parsing: `sudo apt install jq`

## Verification

Test warm boot is working:

```bash
# 1. Create snapshot
./scripts/create-runtime-snapshot.sh

# 2. Create test container
time curl -X POST http://127.0.0.1:18080/v1/containers \
  -H "Content-Type: application/json" \
  -d '{"name": "test-warm", "image": "nginx:alpine"}'

# Should complete in 8-18 seconds!

# 3. Check boot method
CONTAINER_ID="..." # From above response
curl http://127.0.0.1:18080/v1/containers/$CONTAINER_ID | jq .item.boot_method
# Should output: "warm"

# 4. Check manager logs
journalctl -u nqrust-manager -n 50 | grep "warm boot"
# Should see: "Container xxx fully provisioned via warm boot (saved ~60s)"
```

## Next Steps

- Read full documentation: [RUNTIME_SNAPSHOTS.md](RUNTIME_SNAPSHOTS.md)
- Container guide: [CONTAINER.md](../CONTAINER.md)
- Feature matrix: [FEATURES.md](../FEATURES.md)

## API Reference

See [RUNTIME_SNAPSHOTS.md](RUNTIME_SNAPSHOTS.md#api-endpoints) for complete API documentation.
