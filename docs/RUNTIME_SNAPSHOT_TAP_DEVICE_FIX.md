# Runtime Snapshot Tap Device Fix

## Problem

Runtime snapshot restore was failing with Firecracker error:
```
Loading a microVM snapshot not allowed after configuring boot-specific resources.
```

This happened because the agent was trying to configure the network interface BEFORE loading the snapshot, which Firecracker doesn't allow.

Additionally, there was a tap device name mismatch:
- Snapshot creation: Uses `fc-tap-{temp-vm-id}` (random)
- Snapshot restore: Uses `fc-tap-{container-vm-id}` (different for each container)
- **Problem**: The snapshot contains network config referencing the creation-time tap device name

## Solution

### Part 1: Agent Fix (DONE)

Modified `apps/agent/src/features/vm/snapshot.rs` to:
1. Create TAP device on host
2. Load snapshot directly (WITHOUT configuring network first)
3. Resume VM

The snapshot already contains the network configuration from when it was created.

### Part 2: Manager Fix (DONE)

Modified `apps/manager/src/features/containers/vm.rs` to:
1. Use predictable tap device name pattern: `fc-rt-{snapshot-id[..12]}`
2. Update guest agent config after restore with new VM ID

### Part 3: Snapshot Rebuild (NEEDED)

**Current Status**: Existing snapshots are incompatible because:
- They were created with tap device names like `fc-tap-{random-temp-vm-id}`
- Restore code expects tap device names like `fc-rt-{snapshot-id}`
- These don't match → snapshot load will fail

**Action Required**:
1. Delete existing runtime snapshots
2. Rebuild snapshots (they will use consistent tap device names)

Note: For the final fix, we need to also modify snapshot CREATION to use the same predictable tap device name pattern. Otherwise newly created snapshots will still have mismatched tap names.

## Immediate Next Steps

1. **Stop the manager and agent** if running
2. **Restart with new binaries**:
   ```bash
   # Agent (in one terminal)
   sudo -E env AGENT_BIND=127.0.0.1:19090 MANAGER_BASE=http://127.0.0.1:18080 FC_RUN_DIR=/srv/fc FC_BRIDGE=fcbr0 ./target/release/agent

   # Manager (in another terminal)
   sudo -E env MANAGER_RECONCILER_DISABLED=1 MANAGER_ALLOW_IMAGE_PATHS=true RUST_LOG=info ./target/release/manager
   ```

3. **Delete old snapshots**:
   ```bash
   curl -X DELETE http://127.0.0.1:18080/v1/runtime-snapshots/86871106-1ea3-4be5-8e49-571f9af3084d
   curl -X DELETE http://127.0.0.1:18080/v1/runtime-snapshots/bdfa3fbd-b3d2-40c9-90f6-c95eac44a297
   ```

4. **Create new snapshot**:
   ```bash
   ./scripts/create-runtime-snapshot.sh
   ```

5. **Test warm boot**:
   ```bash
   time curl -X POST http://127.0.0.1:18080/v1/containers \
     -H "Content-Type: application/json" \
     -d '{"name": "test-fixed", "image": "nginx:alpine"}'
   ```

## Known Limitation

The current fix still has a tap device name mismatch between creation and restore. For the complete fix, we need to:

1. **Modify snapshot creation** to use predictable tap device names based on snapshot ID
2. OR store the tap device name in snapshot metadata and read it during restore
3. OR use a different approach (e.g., network reconfiguration after snapshot load)

For now, the snapshot will still have the wrong tap device name internally, which may cause issues.

## Alternative Approach (Future Work)

Instead of trying to match tap device names, we could:
1. Create snapshots WITHOUT network configuration
2. Reconfigure network dynamically after snapshot restore
3. This would allow each container to have its own unique tap device

This requires modifying the snapshot creation process to remove/disable the network interface before taking the snapshot.
