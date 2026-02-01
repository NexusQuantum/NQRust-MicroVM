# Container Runtime Snapshots (Warm Boot)

## Overview

The Runtime Snapshot feature enables fast container startup by using Firecracker VM snapshots to bypass Docker daemon initialization. This reduces container creation time from **60-120 seconds to 5-15 seconds** on resource-constrained machines.

## How It Works

### Traditional Cold Boot (~60-120s)
1. Create new VM with container runtime image
2. Boot VM (~3-5s)
3. Wait for guest IP (~5-30s)
4. **Wait for Docker daemon to start (~60-120s)** ← Bottleneck
5. Pull container image
6. Start container

### Warm Boot with Snapshots (~5-15s)
1. Restore VM from runtime snapshot (Docker already running)
2. Docker auto-starts via init system (~2-5s)
3. Guest gets new IP via DHCP (~2-5s)
4. Docker is ready (~1s health check)
5. Pull container image
6. Start container

**Time saved:** ~60-90 seconds per container creation

## Architecture

### Components

1. **Runtime Snapshots** - Pre-built Firecracker snapshots with Docker daemon ready
2. **Snapshot Builder** - Service that creates snapshots from container runtime images
3. **Warm Boot Path** - Container creation flow that attempts snapshot restore first
4. **Fallback Logic** - Graceful degradation to cold boot when snapshots unavailable

### Database Schema

```sql
CREATE TABLE runtime_snapshots (
    id UUID PRIMARY KEY,
    runtime_image_id UUID REFERENCES images(id),
    snapshot_path TEXT NOT NULL,
    state TEXT NOT NULL,              -- 'creating', 'ready', 'unhealthy', 'deleted'
    fc_version TEXT NOT NULL,         -- Firecracker version compatibility
    created_at TIMESTAMPTZ NOT NULL,
    success_count INT DEFAULT 0,
    failure_count INT DEFAULT 0,
    last_used_at TIMESTAMPTZ,
    metadata JSONB                    -- size_bytes, compressed_size_bytes, etc.
);

ALTER TABLE containers ADD COLUMN boot_method TEXT;  -- 'warm' or 'cold'
```

### File Structure

```
apps/manager/src/features/runtime_snapshots/
├── builder.rs          # Snapshot creation logic
├── repo.rs            # Database operations
├── routes.rs          # API endpoints
├── service.rs         # Business logic
└── mod.rs

/srv/fc/runtime-snapshots/
└── {runtime-image-id}/
    ├── snapshot.mem    # Memory snapshot (~512MB, compressible to ~200MB)
    ├── snapshot.state  # VM state
    ├── rootfs.ext4     # Runtime rootfs
    └── metadata.json   # Snapshot metadata
```

## API Endpoints

### Create Runtime Snapshot

```bash
POST /v1/runtime-snapshots
Content-Type: application/json

{
  "runtime_image_id": "uuid"
}

Response:
{
  "id": "uuid"
}
```

Creates a snapshot asynchronously. Status tracked via state field.

### List Runtime Snapshots

```bash
GET /v1/runtime-snapshots

Response:
{
  "items": [
    {
      "id": "uuid",
      "runtime_image_id": "uuid",
      "snapshot_path": "/srv/fc/runtime-snapshots/...",
      "state": "ready",
      "fc_version": "v1.9.0",
      "created_at": "2026-01-24T12:00:00Z",
      "success_count": 42,
      "failure_count": 0,
      "last_used_at": "2026-01-24T13:00:00Z",
      "metadata": {
        "size_bytes": 900000000,
        "mem_size_bytes": 512000000,
        "compressed": false
      }
    }
  ]
}
```

### Get Runtime Snapshot

```bash
GET /v1/runtime-snapshots/{id}

Response:
{
  "item": { ... }
}
```

### Delete Runtime Snapshot

```bash
DELETE /v1/runtime-snapshots/{id}

Response:
{
  "ok": true
}
```

Soft deletes (marks as 'deleted'). Physical cleanup happens via garbage collection.

### Rebuild Runtime Snapshot

```bash
POST /v1/runtime-snapshots/{id}/rebuild

Response:
{
  "id": "uuid",
  "message": "Snapshot rebuild initiated"
}
```

Useful when snapshot becomes unhealthy or needs updating.

## Container Boot Method Tracking

Containers now include a `boot_method` field:

```bash
GET /v1/containers/{id}

Response:
{
  "item": {
    "id": "uuid",
    "name": "my-container",
    "boot_method": "warm",  // or "cold"
    ...
  }
}
```

## Snapshot Lifecycle

### 1. Creation

**Triggered by:**
- Manual: `POST /v1/runtime-snapshots`
- Automatic: When container runtime image is registered (future)

**Process:**
1. Create temporary VM with runtime image
2. Wait for Docker daemon to be ready (~60-120s)
3. Verify Docker configured for auto-start
4. Flush network configuration
5. Stop guest agent
6. Pause VM
7. Take Firecracker snapshot
8. Store artifacts and metadata
9. Destroy temporary VM

**Duration:** ~10-30 seconds (runs asynchronously)

**State:** `creating` → `ready`

### 2. Usage (Warm Boot)

**When container is created:**
1. Check if runtime snapshot exists and is `ready`
2. If `creating`, wait up to 60s for it to become `ready`
3. Restore VM from snapshot
4. VM boots with Docker already running
5. Track as "warm" boot

**Fallback to cold boot if:**
- No snapshot exists
- Snapshot state is `unhealthy`
- Snapshot state is `creating` and timeout (60s) reached
- Firecracker version mismatch
- Restore fails
- Docker verification fails

### 3. Health Tracking

**Automatic health monitoring:**
- Success/failure counters updated on each use
- After 3+ consecutive failures → marked `unhealthy`
- Unhealthy snapshots trigger automatic rebuild
- Firecracker version validated before restore

**Health states:**
- `creating` - Being built
- `ready` - Available for use
- `unhealthy` - Failed multiple times, needs rebuild
- `deleted` - Soft deleted, pending cleanup

### 4. Cleanup

**Automatic cleanup (planned):**
- Daily garbage collection
- Snapshots unused for 30+ days marked for deletion
- Physical file deletion for 'deleted' snapshots

**Manual cleanup:**
- `DELETE /v1/runtime-snapshots/{id}` - Soft delete
- Garbage collector removes files later

## Logging

The system provides detailed logs for observability:

```
INFO Container abc123 VM created: vm-def456 via warm boot in 8.23s
INFO Container abc123 fully provisioned in 12.45s via warm boot (saved ~60s)

WARN Warm boot failed for container abc123, falling back to cold boot: No runtime snapshot available
INFO Container abc123 VM created: vm-def456 via cold boot in 67.89s
INFO Container abc123 fully provisioned in 89.12s via cold boot
```

## Performance Metrics

### Expected Performance

**Warm Boot:**
- VM restore: ~2-3s
- Docker auto-start: ~2-5s
- IP assignment: ~2-5s
- **Total: 5-15s**

**Cold Boot:**
- VM boot: ~3-5s
- IP assignment: ~5-30s
- Docker startup: ~60-120s
- **Total: 60-150s**

**Time Savings:** 45-135 seconds per container

### Storage Requirements

Per runtime snapshot:
- Memory: ~512MB (uncompressed) / ~200MB (compressed)
- Disk: ~386MB (Alpine + Docker)
- Metadata: <1KB
- **Total: ~900MB uncompressed / ~600MB compressed**

**Limits:**
- Max 10 active snapshots (configurable)
- Auto-cleanup after 30 days unused

## Configuration

### Environment Variables

**Manager:**
```bash
MANAGER_STORAGE_ROOT=/srv/fc              # Snapshot storage location
MANAGER_MAX_RUNTIME_SNAPSHOTS=10          # Max concurrent snapshots
```

**For snapshot creation:**
```bash
CONTAINER_RUNTIME_KERNEL=/srv/images/vmlinux-5.10.fc.bin
CONTAINER_RUNTIME_ROOTFS=/srv/images/container-runtime.ext4
```

## Current Implementation Status

### ✅ Implemented (Phases 1-4)

- [x] Database schema and migrations
- [x] Runtime snapshot CRUD operations
- [x] Async snapshot builder
- [x] Firecracker version detection (manager & agent)
- [x] Agent snapshot restore endpoint
- [x] NIC configuration on restore
- [x] Container VM warm boot implementation
- [x] Fallback to cold boot on failure
- [x] Boot method tracking
- [x] Health tracking (success/failure counts)
- [x] Auto-unhealthy marking (3+ failures)
- [x] Wait logic for creating snapshots
- [x] Comprehensive logging
- [x] Snapshot rebuild API

### ⚠️ Pending (Future Phases)

- [ ] **End-to-end testing** - Verify warm boot works in real environment
- [ ] Snapshot compression (zstd)
- [ ] Garbage collection service
- [ ] Auto-snapshot on image registration
- [ ] UI components (registry page)
- [ ] Metrics instrumentation (Prometheus)
- [ ] Guest command execution for network flush
- [ ] Snapshot version migration

## Known Limitations

1. **Not Yet Tested** - Warm boot implementation is complete but needs end-to-end testing in a real environment

2. **No Compression** - Memory snapshots stored uncompressed (~512MB). Compression would reduce to ~200MB.

3. **Manual Snapshot Creation** - Snapshots must be manually created via API. Auto-creation planned.

4. **Network Flush Placeholder** - Network configuration flush uses placeholder logic. Needs guest agent integration.

5. **Single Runtime Image** - System finds runtime by path. Better tracking of runtime image IDs needed.

## Troubleshooting

### Snapshot Creation Fails

**Check:**
1. VM can boot with runtime image
2. Docker daemon starts successfully
3. Sufficient disk space in `MANAGER_STORAGE_ROOT`
4. Firecracker version compatible

**Logs:**
```bash
# Check manager logs
journalctl -u manager -f | grep -i snapshot

# Check snapshot state
curl http://localhost:18080/v1/runtime-snapshots
```

### Warm Boot Always Falls Back to Cold Boot

**Causes:**
1. No snapshot exists for runtime image
2. Snapshot state is not 'ready'
3. Firecracker version mismatch
4. Agent doesn't support snapshot restore

**Solution:**
```bash
# Create snapshot
curl -X POST http://localhost:18080/v1/runtime-snapshots \
  -H "Content-Type: application/json" \
  -d '{"runtime_image_id": "your-runtime-image-uuid"}'

# Wait for snapshot to be ready
curl http://localhost:18080/v1/runtime-snapshots/{id}
# Check state field == "ready"
```

### Snapshot Marked Unhealthy

**Cause:** 3+ consecutive restore failures

**Solution:**
```bash
# Rebuild snapshot
curl -X POST http://localhost:18080/v1/runtime-snapshots/{id}/rebuild

# Or delete and recreate
curl -X DELETE http://localhost:18080/v1/runtime-snapshots/{id}
curl -X POST http://localhost:18080/v1/runtime-snapshots \
  -d '{"runtime_image_id": "uuid"}'
```

## Future Enhancements

1. **Compression** - zstd compression for memory snapshots (50-70% reduction)
2. **Auto-Creation** - Create snapshots when runtime images registered
3. **Warm-to-Warm** - Snapshot a running container for instant clones
4. **Multi-Version** - Support multiple snapshots per runtime (A/B testing)
5. **Metrics Dashboard** - Visualize boot method distribution, time saved
6. **Incremental Snapshots** - Differential snapshots to reduce storage

## References

- Design Document: [docs/plans/2026-01-24-container-runtime-snapshot-design.md](../plans/2026-01-24-container-runtime-snapshot-design.md)
- Container Documentation: [CONTAINER.md](../CONTAINER.md)
- Features Matrix: [FEATURES.md](../FEATURES.md)
