# Container Runtime Warm Boot Design

**Date:** 2026-01-24
**Status:** Approved
**Author:** Brainstorming session

## Problem Statement

When creating containers on slow machines (1 CPU, 512MB RAM), the Docker daemon inside the container runtime VM takes too long to start, often exceeding the 120-second timeout. This results in container creation failures.

**Current timing breakdown:**
- VM boot: 3-5 seconds
- Guest IP detection: 5-30 seconds
- Docker daemon startup: 60-120+ seconds (the bottleneck)
- Image pull + container start: 10-60+ seconds

On constrained resources, Docker daemon initialization (cgroups, overlay2, plugins) can exceed the timeout.

## Solution Overview

Use Firecracker's native snapshot capability to restore from a pre-warmed state where Docker is already running, instead of cold booting every time.

**Two paths for container creation:**

```
Fast Path (default):
  Restore snapshot → Docker already ready → Pull image → Start container
  (~5-10 seconds total)

Fallback Path (if snapshot unavailable or fails):
  Boot fresh VM → Wait for Docker → Pull image → Start container
  (current behavior, 60-120+ seconds)
```

The fast path is an optimization layer. If anything fails, we fall back to the proven cold-boot path transparently.

## Networking Considerations

Snapshotting a running VM with active networking causes problems:
- MAC address collisions between restored VMs
- IP address conflicts from duplicate DHCP leases
- Stale Docker network state
- ARP cache pollution

**Solution: Snapshot with "dormant" networking**

Take the snapshot at a specific point:
- Docker daemon running and healthy
- Guest agent installed but not reporting
- Network interface exists but has no IP assigned

**On restore:**
1. Restore snapshot with new NIC config (fresh MAC assigned by agent)
2. Docker daemon auto-starts via init system (already configured in runtime image)
3. DHCP runs, gets new unique IP
4. Guest agent starts and begins reporting with new IP
5. Manager detects guest IP (existing flow)
6. Container creation proceeds

**Result:** ~5-10 seconds instead of 60-120+, with no network conflicts.

**Note:** Firecracker snapshots exclude network device config, allowing new MAC assignment on restore.

## Snapshot Lifecycle

### Creation Triggers

1. **On-demand** - Admin triggers via API/UI
2. **Automatic** - When a new container runtime image is registered

### Creation Process

```
1. Boot fresh container runtime VM (internal, not user-visible)
2. Wait for Docker daemon ready (use existing health check)
3. Verify Docker is configured for auto-start via init system
4. Flush network config (remove IP, reset interface to down)
5. Stop guest agent to prevent reporting during snapshot
6. Take Firecracker snapshot (memory + disk, excludes NIC config)
7. Store snapshot artifacts with metadata
8. Destroy temporary VM
```

**Docker State:** The daemon is left running but the init system (OpenRC/systemd) is configured to restart it on boot. This ensures Docker is available immediately after restore without stale process state.

**Guest Agent:** Stopped before snapshot to prevent premature reporting. Will start fresh after restore when VM has valid IP.

**Snapshot Duration:** ~10-30 seconds (VM boot + Docker ready + snapshot operations). This process runs asynchronously and does not block image registration.

### Storage Structure

```
/srv/fc/runtime-snapshots/
  └── {runtime-image-hash}/
      ├── snapshot.mem      # Memory snapshot
      ├── snapshot.state    # VM state
      ├── metadata.json     # Version, created_at, runtime_image_id, fc_version
      └── rootfs.ext4       # Copy of rootfs at snapshot time
```

### Versioning

Each snapshot is tied to:
- **Runtime image hash** - When the runtime image changes, old snapshots are invalidated
- **Firecracker version** - Snapshots are only compatible with the same Firecracker version that created them

On restore, if Firecracker version mismatch detected, snapshot is marked unhealthy and rebuild triggered.

## Container Creation Flow

### Current Flow (Cold Boot)

```
create_container()
  → create_vm(runtime_image)
  → start_vm()
  → wait_for_guest_ip()          # 5-30 seconds
  → wait_for_docker_ready()      # 60-120+ seconds
  → pull_image()
  → create_and_start_container()
```

### New Flow (Warm Boot with Fallback)

```
create_container()
  → find_runtime_snapshot(runtime_image)

  IF snapshot exists AND ready:
    → restore_vm_from_snapshot(with_fresh_mac=true)  # NIC excluded from snapshot
    → start_vm()
    → wait_for_docker_autostart() # 2-5 seconds (init system restarts Docker)
    → wait_for_guest_ip()         # 2-5 seconds (DHCP + guest agent)
    → verify_docker_ready()       # ~1 second (quick health check)

    IF verify fails:
      → destroy_vm()
      → GOTO fallback

  ELSE (fallback):
    → create_vm(runtime_image)   # existing cold boot path
    → start_vm()
    → wait_for_guest_ip()
    → wait_for_docker_ready()

  → pull_image()
  → create_and_start_container()
```

State transitions remain unchanged: `creating` → `booting` → `initializing` → `running`.

## Error Handling & Fallback

### Fallback Triggers

1. No snapshot exists for runtime image
2. Snapshot is in 'creating' state and timeout (60s) reached
3. Snapshot is marked 'unhealthy'
4. Firecracker version mismatch detected
5. Snapshot restore fails (Firecracker error)
6. Docker verify fails (not responding within 10 seconds)
7. Network setup fails (DHCP or guest IP detection)

### Fallback Behavior

```rust
async fn create_container_vm(...) -> Result<Vm> {
    // Try fast path first
    if let Some(snapshot) = find_runtime_snapshot(&runtime_image_id).await? {
        match try_warm_boot(&snapshot).await {
            Ok(vm) => return Ok(vm),
            Err(e) => {
                tracing::warn!("Warm boot failed, falling back to cold boot: {}", e);
                record_snapshot_failure(&snapshot.id).await?;
            }
        }
    }

    // Fallback: existing cold boot path (unchanged)
    cold_boot_container_vm(&runtime_image_id).await
}
```

### Snapshot Health Tracking

- Track success/failure counts per snapshot
- If a snapshot fails 3+ times consecutively, mark as unhealthy
- Unhealthy snapshots trigger automatic rebuild
- Admin can manually trigger rebuild via API

## Database Changes

### New Table: `runtime_snapshots`

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
```

### Container Table Addition

```sql
ALTER TABLE containers ADD COLUMN boot_method TEXT;  -- 'warm' or 'cold'
```

## API Changes

### New Endpoints

```
POST   /v1/runtime-snapshots              # Create snapshot for a runtime image
GET    /v1/runtime-snapshots              # List all runtime snapshots
GET    /v1/runtime-snapshots/{id}         # Get snapshot details
DELETE /v1/runtime-snapshots/{id}         # Delete a snapshot
POST   /v1/runtime-snapshots/{id}/rebuild # Force rebuild unhealthy snapshot
```

### Modified Endpoints

```
GET /v1/containers/{id}
  # Add optional field: boot_method: "warm" | "cold"
```

Container creation endpoint (`POST /v1/containers`) remains unchanged - optimization is transparent.

## UI Changes

### Registry Page (`/registry`)

Add "Runtime Snapshots" section:
- List runtime images with snapshot status (ready / creating / none)
- "Create Snapshot" button for images without one
- "Rebuild" button for unhealthy snapshots
- Success/failure counts and last used timestamp

### Container Detail Page (`/containers/[id]`)

- Show `boot_method` badge: "Warm Boot" or "Cold Boot"

### Container Creation Wizard

No changes - users don't need to think about snapshots.

## Testing Plan

### Unit Tests

- `find_runtime_snapshot()` returns correct snapshot or None
- Fallback logic triggers on various error conditions
- Snapshot health tracking increments correctly

### Integration Tests

1. **Happy path** - Create snapshot, create container, verify warm boot used
2. **Fallback path** - Delete snapshot, create container, verify cold boot works
3. **Network isolation** - Create two containers from same snapshot, verify unique MACs and IPs
4. **Failure recovery** - Corrupt snapshot, verify fallback triggers and snapshot marked unhealthy

### Manual Testing Checklist

- [ ] Container creation on fast machine with warm boot
- [ ] Container creation on slow machine (1 CPU, 512MB) with warm boot
- [ ] Container creation when no snapshot exists (fallback)
- [ ] Container creation while snapshot is creating (wait or fallback)
- [ ] Five containers simultaneously - no network conflicts, unique MACs/IPs
- [ ] Rebuild snapshot after runtime image update
- [ ] Snapshot with Firecracker version mismatch triggers fallback
- [ ] Docker overlay2 state works correctly after restore
- [ ] Storage compression reduces snapshot size by >50%
- [ ] Unused snapshot auto-deleted after 30 days

## Implementation Phases

### Phase 1: Database & Core Infrastructure

1. Add `runtime_snapshots` migration
2. Add `boot_method` column to containers table
3. Create `RuntimeSnapshotRepo` for database operations
4. Create `RuntimeSnapshotService` with basic CRUD

### Phase 2: Snapshot Creation

5. Implement snapshot builder - boots VM, waits for Docker, flushes network, stops guest agent, takes snapshot
6. Add Firecracker version detection and storage in metadata
7. Integrate with agent's existing snapshot capabilities (exclude NIC from snapshot)
8. Implement async snapshot creation (non-blocking for image registration)
9. Add snapshot compression (zstd for memory snapshots)
10. Add API endpoints for manual snapshot management
11. Test snapshot creation works end-to-end

### Phase 3: Warm Boot Path

12. Implement `restore_vm_from_snapshot()` in agent with NIC config generation
13. Add Firecracker version validation before restore
14. Add `try_warm_boot()` to container service
15. Add wait logic for snapshot in 'creating' state (60s timeout)
16. Add quick `verify_docker_ready()` (10s timeout)
17. Wire up fallback logic with all triggers

### Phase 4: Health Tracking & Polish

18. Implement success/failure counting
19. Add automatic snapshot rebuild on failures (3+ consecutive failures)
20. Add metrics instrumentation (Prometheus format)
21. Add `boot_method` to container API responses
22. UI additions (registry page, container detail badge)
23. Add logging for boot method and time saved

### Phase 5: Automation & Optimization

24. Auto-create snapshot when runtime image is registered (async, background task)
25. Daily garbage collection of deleted snapshots
26. Storage usage monitoring and alerts
27. Admin settings for tuning behavior (max snapshots, compression level, cleanup age)
28. Dashboard metrics for warm boot performance
29. Test Docker overlay2 behavior thoroughly under various scenarios

## Storage Management

### Size Estimates

Per runtime snapshot:
- Memory snapshot: ~512MB (uncompressed) / ~200MB (compressed with zstd)
- Disk snapshot: ~386MB (Alpine + Docker layers)
- Metadata: <1KB
- **Total per runtime: ~900MB uncompressed / ~600MB compressed**

### Storage Limits

- **Max snapshots:** 10 active runtime snapshots (configurable via `MANAGER_MAX_RUNTIME_SNAPSHOTS`)
- **Compression:** Enabled by default for memory snapshots (zstd level 3)
- **Auto-cleanup:** Snapshots unused for 30+ days are marked for deletion
- **Garbage collection:** Runs daily to remove deleted snapshots

### Cleanup Policy

```
When runtime image deleted → Mark snapshot as 'deleted'
Daily GC task → Remove files for deleted snapshots
Manual cleanup → DELETE /v1/runtime-snapshots/{id}
```

### Concurrent Creation

If container creation requested while snapshot is being created:
1. Check snapshot state: `creating`
2. Wait up to 60 seconds for snapshot to become `ready`
3. If timeout, fallback to cold boot
4. If ready, proceed with warm boot

This prevents multiple simultaneous snapshot creation attempts.

## Observability

### Metrics

```
container_boot_method{method="warm|cold"}                    # Counter
container_boot_duration_seconds{method="warm|cold"}          # Histogram
snapshot_restore_success_total                               # Counter
snapshot_restore_failure_total{reason="..."}                 # Counter
snapshot_health_status{snapshot_id, status="ready|unhealthy"} # Gauge
snapshot_storage_bytes{compressed="true|false"}              # Gauge
```

### Logging

```
INFO  Container vm-abc123 created via warm boot (saved 94s)
WARN  Snapshot snap-def456 restore failed: incompatible FC version, falling back to cold boot
INFO  Runtime snapshot created for image img-789abc in 23s (compressed 512MB → 198MB)
```

### Dashboard Metrics

Manager should track and expose:
- Warm boot success rate (%)
- Average time saved per warm boot
- Total storage used by snapshots
- Snapshot health status per runtime image

## Success Criteria

- Container creation completes in <15 seconds on slow machines (1 CPU, 512MB)
- Zero network conflicts when creating multiple containers simultaneously
- Fallback to cold boot works transparently when snapshots unavailable
- No changes required to existing container creation API or UI wizard
- Storage usage stays under 10GB with auto-cleanup enabled
- Warm boot success rate >95% under normal conditions
