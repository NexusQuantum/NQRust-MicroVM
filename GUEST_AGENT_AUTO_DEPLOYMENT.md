# Automatic Guest Agent Deployment - COMPLETE

## Overview
The guest agent is now automatically deployed to every VM during creation, providing real CPU and memory metrics from inside the VM instead of host-side Firecracker process metrics.

## What Was Implemented

### 1. Guest Agent Binary
- **Location**: `apps/guest-agent/`
- **Size**: 2.2MB static musl binary
- **Features**: 
  - CPU usage from `/proc/stat`
  - Memory usage from `/proc/meminfo`
  - HTTP API on `:8080` with `/metrics` and `/health` endpoints
  - Auto-starts via OpenRC service

### 2. Database Schema
- **Migration**: `0011_guest_ip.sql`
- **Added**: `guest_ip VARCHAR(45)` column to vm table
- **Purpose**: Track each VM's guest IP for metrics queries

### 3. Manager Integration
- **Modified**: `apps/manager/src/features/vms/service.rs`
  - `get_process_stats()` tries guest agent first, fallback to host metrics
  - Added guest agent installation in both VM creation functions
- **Added**: `apps/manager/src/features/vms/guest_agent.rs`
  - `install_to_rootfs()` mounts VM rootfs and injects guest agent
  - Creates OpenRC service for auto-start
  - Creates IP reporting script for auto-registration
- **Added**: `/v1/vms/{id}/guest-ip` endpoint for IP registration

### 4. Automatic Deployment Flow
1. VM creation starts (`create_vm_from_spec()` or `create_vm_from_snapshot()`)
2. Rootfs is prepared (copied from template or restored from snapshot)
3. **NEW**: `guest_agent::install_to_rootfs()` is called:
   - Mounts VM rootfs
   - Copies guest-agent binary to `/usr/local/bin/`
   - Creates OpenRC service file `/etc/init.d/guest-agent`
   - Enables service for auto-start on boot
   - Creates IP reporting script `/usr/local/bin/report-ip.sh`
   - Unmounts rootfs
4. VM starts with guest agent automatically running
5. Guest agent detects IP and reports to manager
6. Manager queries guest agent for real metrics

## Key Files Modified/Created

### Created Files
- `apps/guest-agent/src/main.rs` - Complete guest agent implementation
- `apps/guest-agent/Cargo.toml` - Build configuration
- `apps/manager/migrations/0011_guest_ip.sql` - Database migration
- `apps/manager/src/features/vms/guest_agent.rs` - Automatic installation module

### Modified Files
- `apps/manager/src/features/vms/service.rs` - Integration and metrics logic
- `apps/manager/src/features/vms/routes.rs` - Guest IP endpoint
- `apps/manager/src/features/vms/repo.rs` - Database operations
- `apps/manager/src/features/vms/mod.rs` - Module declarations

## Installation Commands

```bash
# Build guest agent binary
cargo build --release --bin guest-agent --target x86_64-unknown-linux-musl

# Run database migration (if not already applied)
cd apps/manager && sqlx migrate run

# Build manager with new integration
cargo build --bin manager
```

## Verification

The automatic deployment is now ready. To test:

1. Create a new VM through the frontend or API
2. The guest agent will be automatically installed during VM creation
3. Check VM metrics - they should show real guest CPU/memory usage instead of 0.2-0.3%
4. Verify guest agent is running inside the VM:
   ```bash
   # Inside the VM
   curl http://localhost:8080/health
   curl http://localhost:8080/metrics
   ```

## Benefits

✅ **Zero Manual Steps**: No scripts to run per VM  
✅ **Real Metrics**: Actual guest CPU/memory usage instead of host process metrics  
✅ **Automatic**: Works on every VM creation without intervention  
✅ **Graceful Fallback**: Falls back to host metrics if guest agent unavailable  
✅ **Small Footprint**: 2.2MB static binary, minimal resource usage  
✅ **Auto-Discovery**: Guest agent automatically reports IP to manager  

## Architecture

```
Manager (Host)                    VM (Guest)
-----------                       ----------
    |                               |
    | 1. Creates VM                 |
    | 2. Mounts rootfs              |
    | 3. Installs guest-agent ----->|
    | 4. Starts VM                  |
    |                               | 5. Guest agent starts
    |                               | 6. Reports IP to manager
    | 7. Queries metrics <----------|
    | 8. Displays real usage        |
```

The automatic guest agent deployment is now complete and integrated into the VM creation flow!