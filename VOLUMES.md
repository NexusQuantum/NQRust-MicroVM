# NQRust MicroVM - Volume Management Documentation

**Last Updated:** 2025-11-03

This document provides comprehensive information about volume management in NQRust-MicroVM, including the central volume registry, auto-registration, volume lifecycle, and best practices.

---

## Table of Contents

- [Overview](#overview)
- [Volume Architecture](#volume-architecture)
- [Volume Types](#volume-types)
- [Volume Registry](#volume-registry)
- [Auto-Registration](#auto-registration)
- [Volume Lifecycle](#volume-lifecycle)
- [Volume Attachments](#volume-attachments)
- [Storage Paths](#storage-paths)
- [API Reference](#api-reference)
- [UI Features](#ui-features)
- [Common Use Cases](#common-use-cases)
- [Troubleshooting](#troubleshooting)

---

## Overview

NQRust-MicroVM provides production-ready volume management with:

- **Central Registry**: Track all volumes across hosts
- **Auto-Registration**: Automatically register rootfs volumes when VMs are created
- **Attachment Tracking**: Monitor which VMs use which volumes
- **Multiple Types**: Support for ext4, qcow2, raw disk images
- **Isolation**: Each VM gets its own rootfs copy (no sharing)
- **Lifecycle Management**: Create, attach, detach, delete volumes
- **Status Tracking**: Available, attached, in-use states

---

## Volume Architecture

### Components

```
┌─────────────────────────────────────────────────────────────┐
│                      Manager (Database)                      │
│                                                               │
│  ┌────────────────────────────────────────────────────┐     │
│  │              Volume Registry                       │     │
│  │                                                     │     │
│  │  - Volume metadata (name, path, size, type)        │     │
│  │  - Host association                                │     │
│  │  - Status (available, attached, in-use)            │     │
│  │  - Attachments (volume → VM mapping)               │     │
│  └────────────────────────────────────────────────────┘     │
│                                                               │
└───────────────────────────┬───────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                    Host Filesystem                           │
│                                                               │
│  /srv/images/                  (Image Registry)              │
│    ├── vmlinux-5.10.bin        (Kernel images)               │
│    ├── alpine-3.18.ext4        (Rootfs templates)            │
│    └── ubuntu-22.04.ext4       (Rootfs templates)            │
│                                                               │
│  /srv/fc/vms/                  (VM Storage)                  │
│    ├── {vm-id-1}/                                            │
│    │   └── storage/                                          │
│    │       ├── rootfs-{uuid}.ext4     (VM rootfs copy)       │
│    │       └── disk-{uuid}.img        (Additional disks)     │
│    └── {vm-id-2}/                                            │
│        └── storage/                                          │
│            └── rootfs-{uuid}.ext4     (VM rootfs copy)       │
│                                                               │
│  /srv/images/containers/       (Container VM rootfs)         │
│    └── {vm-id}.ext4            (Container runtime rootfs)    │
│                                                               │
│  /srv/images/functions/        (Function VM rootfs)          │
│    └── {vm-id}.ext4            (Function runtime rootfs)     │
│                                                               │
└───────────────────────────────────────────────────────────────┘
```

### Volume Flow

1. **Source Image**: Template image in `/srv/images/`
2. **Copy on Create**: Manager copies image to VM-specific directory
3. **Volume Registration**: Volume record created in database
4. **Attachment**: Volume linked to VM via `volume_attachment` table
5. **Status Updates**: Volume status changes based on attachments

---

## Volume Types

### Supported Formats

- **ext4**: Linux ext4 filesystem images (most common)
- **qcow2**: QEMU Copy-On-Write format (supports snapshots, compression)
- **raw**: Raw disk images (1:1 disk representation)

### Volume Categories

#### 1. Rootfs Volumes
- **Purpose**: VM root filesystems
- **Auto-Registered**: Yes (when VM is created)
- **Naming**: `{vm_name} (rootfs-{uuid}.ext4)`
- **Path**: `/srv/fc/vms/{vm-id}/storage/rootfs-{uuid}.ext4`
- **Drive ID**: `rootfs`

#### 2. Data Volumes
- **Purpose**: Additional storage for VMs
- **Auto-Registered**: No (manual creation)
- **Naming**: Custom
- **Path**: `/srv/fc/vms/{vm-id}/storage/disk-{uuid}.img`
- **Drive ID**: Custom (e.g., `vdb`, `vdc`)

#### 3. Container Runtime Volumes
- **Purpose**: Docker container runtime images
- **Auto-Registered**: Yes (when container is created)
- **Naming**: Container-specific
- **Path**: `/srv/images/containers/{vm-id}.ext4`
- **Drive ID**: `rootfs`

#### 4. Function Runtime Volumes
- **Purpose**: Serverless function runtime images
- **Auto-Registered**: Yes (when function is created)
- **Naming**: Function-specific
- **Path**: `/srv/images/functions/{vm-id}.ext4`
- **Drive ID**: `rootfs`

---

## Volume Registry

### Overview

The volume registry provides centralized tracking of all volumes:

- **Metadata Storage**: Name, description, path, size, type
- **Host Association**: Track which host manages which volumes
- **Status Tracking**: Available, attached, in-use
- **Attachment Tracking**: Which VMs use which volumes
- **VM Count**: Monitor volume usage

### Database Schema

```sql
CREATE TABLE volume (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL,
    description TEXT,
    path TEXT NOT NULL UNIQUE,  -- Absolute path to volume file
    size_bytes BIGINT NOT NULL,
    type VARCHAR(50) NOT NULL,  -- 'ext4', 'qcow2', 'raw'
    status VARCHAR(50) NOT NULL DEFAULT 'available',  -- 'available', 'attached', 'in-use'
    host_id UUID NOT NULL REFERENCES host(id),
    created_at TIMESTAMPTZ DEFAULT now(),
    updated_at TIMESTAMPTZ DEFAULT now()
);

CREATE TABLE volume_attachment (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    volume_id UUID NOT NULL REFERENCES volume(id) ON DELETE CASCADE,
    vm_id UUID NOT NULL REFERENCES vm(id) ON DELETE CASCADE,
    drive_id VARCHAR(255) NOT NULL,  -- 'rootfs', 'vdb', 'vdc', etc.
    attached_at TIMESTAMPTZ DEFAULT now(),
    UNIQUE(volume_id, vm_id)  -- One volume can only be attached once per VM
);
```

### Volume Status

- **`available`**: Volume is not attached to any VM
- **`attached`**: Volume is attached to one VM
- **`in-use`**: Reserved for future use (multiple attachments)

---

## Auto-Registration

### How It Works

When a VM is created, the manager automatically:

1. **Copies rootfs** from source image to VM-specific directory
2. **Creates volume record** in the registry
3. **Attaches volume** to the VM via `volume_attachment` table
4. **Updates status** to `attached`

### Auto-Registration Flow

```rust
// apps/manager/src/features/vms/service.rs

async fn ensure_volume_registered(
    st: &AppState,
    vm_id: Uuid,
    rootfs_path: &str,
    host_id: Uuid
) -> Result<()> {
    // Check if volume already exists
    let existing = volume_repo.list_by_host(host_id).await?;

    for volume in existing {
        if volume.path == rootfs_path {
            // Volume exists, attach if available
            if volume.status == "available" {
                volume_repo.attach(volume.id, vm_id, "rootfs").await?;
            }
            return Ok(());
        }
    }

    // Get VM name for descriptive volume name
    let vm_name = get_vm_name(vm_id).await?;

    // Extract filename from path
    let filename = Path::new(rootfs_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("rootfs");

    // Create descriptive name
    let name = format!("{} ({})", vm_name, filename);

    // Get file size
    let size_bytes = fs::metadata(rootfs_path)
        .ok()
        .map(|m| m.len() as i64)
        .unwrap_or(0);

    // Detect volume type from extension
    let volume_type = if rootfs_path.ends_with(".ext4") {
        "ext4"
    } else if rootfs_path.ends_with(".qcow2") {
        "qcow2"
    } else {
        "raw"
    };

    // Create volume record
    let volume = volume_repo.create(
        &name,
        Some(&format!("Rootfs for VM: {}", vm_name)),
        rootfs_path,
        size_bytes,
        volume_type,
        host_id,
    ).await?;

    // Attach to VM
    volume_repo.attach(volume.id, vm_id, "rootfs").await?;

    Ok(())
}
```

### Benefits

- **Automatic Tracking**: All VM volumes are tracked automatically
- **Visibility**: See all volumes in use across hosts
- **Resource Management**: Monitor disk usage per host
- **Cleanup Detection**: Identify orphaned volumes
- **Attachment History**: Track which VMs used which volumes

---

## Volume Lifecycle

### Creation

#### Auto-Created (Rootfs)
Volumes are automatically created when VMs are created:

```bash
# VM creation triggers volume auto-registration
POST /v1/vms
{
  "name": "my-vm",
  "rootfs_image_id": "550e8400-e29b-41d4-a716-446655440000"
}

# Result:
# - Rootfs copied to /srv/fc/vms/{vm-id}/storage/rootfs-{uuid}.ext4
# - Volume record created with name "my-vm (rootfs-{uuid}.ext4)"
# - Volume attached to VM with drive_id "rootfs"
```

#### Manual Creation
Create volumes manually via API:

```bash
POST /v1/volumes
{
  "name": "Data Volume 1",
  "description": "Additional storage",
  "path": "/srv/fc/volumes/data-vol-1.ext4",
  "size_bytes": 10737418240,
  "type": "ext4",
  "host_id": "bbab8c75-f516-47ec-987a-828422b2ee5a"
}
```

### Attachment

Attach a volume to a VM:

```bash
POST /v1/volumes/{volume_id}/attach
{
  "vm_id": "cd2d0de0-5cfb-49a5-a84d-bb01684fd988",
  "drive_id": "vdb"
}

# Result:
# - Volume status changes to "attached"
# - Attachment record created
# - VM drive configuration updated (if VM is running)
```

### Detachment

Detach a volume from a VM:

```bash
POST /v1/volumes/{volume_id}/detach/{vm_id}

# Result:
# - Attachment record deleted
# - Volume status changes to "available" (if no other attachments)
# - VM drive removed (if VM is running)
```

### Deletion

Delete a volume:

```bash
DELETE /v1/volumes/{volume_id}

# Requirements:
# - Volume must be in "available" status (not attached)
# - File will NOT be deleted from filesystem (registry only)

# Note: To delete the actual file, use:
# sudo rm {volume_path}
```

---

## Volume Attachments

### Attachment Types

#### Single Attachment (Default)
One volume attached to one VM:

```
Volume A → VM 1
```

#### Multiple Attachments (Future)
One volume attached to multiple VMs (read-only):

```
Volume A → VM 1 (read-only)
Volume A → VM 2 (read-only)
Volume A → VM 3 (read-only)
```

**Note**: Multiple attachments are not currently supported but planned for future releases.

### Attachment Constraints

- **Unique per VM**: A volume can only be attached once per VM
- **Single Writer**: Only one VM can write to a volume at a time
- **Drive ID**: Each attachment has a unique drive_id (`rootfs`, `vdb`, `vdc`, etc.)

### Attachment Lifecycle

```
┌──────────┐
│ Available│
└────┬─────┘
     │ attach()
     ▼
┌──────────┐
│ Attached │
└────┬─────┘
     │ detach()
     ▼
┌──────────┐
│ Available│
└──────────┘
```

---

## Storage Paths

### Standard Paths

#### Image Registry
```
/srv/images/
├── vmlinux-5.10.bin          # Kernel images
├── alpine-3.18.ext4          # Rootfs templates
├── ubuntu-22.04.ext4         # Rootfs templates
└── debian-12.ext4            # Rootfs templates
```

#### VM Storage (Regular VMs)
```
/srv/fc/vms/{vm-id}/
├── storage/
│   ├── rootfs-{uuid}.ext4    # VM rootfs (copy of template)
│   └── disk-{uuid}.img       # Additional data disks
├── logs/
│   ├── firecracker.log       # Firecracker logs
│   └── console.log           # Serial console output
├── sock/
│   └── fc.sock               # Firecracker API socket
└── snapshots/
    └── {snapshot-id}/        # VM snapshots
```

#### Container Storage
```
/srv/images/containers/
└── {vm-id}.ext4              # Container runtime rootfs
```

#### Function Storage
```
/srv/images/functions/
└── {vm-id}.ext4              # Function runtime rootfs
```

### Path Configuration

```bash
# Manager configuration
MANAGER_STORAGE_ROOT=/srv/fc/vms      # VM storage root
MANAGER_IMAGE_ROOT=/srv/images        # Image registry root
```

### Storage Isolation

**Critical**: Each VM gets its own rootfs copy to prevent data corruption:

```rust
// BEFORE FIX (WRONG - VMs shared rootfs):
let rootfs_path = "/srv/images/alpine-3.18.ext4";  // ❌ Shared!

// AFTER FIX (CORRECT - Each VM gets a copy):
let rootfs_path = st.storage.alloc_rootfs(
    vm_id,
    Path::new("/srv/images/alpine-3.18.ext4")
).await?;
// Result: /srv/fc/vms/{vm-id}/storage/rootfs-{uuid}.ext4  ✅ Isolated!
```

---

## API Reference

### Manager API

#### List Volumes

```http
GET /v1/volumes
```

**Response:**
```json
[
  {
    "id": "336ae2fd-1b83-4c4d-9778-d34d587374f9",
    "name": "Shiro Logic6 (rootfs-bfbb2b86.ext4)",
    "description": "Rootfs for VM: Shiro Logic6",
    "path": "/srv/fc/vms/0fa7cf15-f92d-49c7-bfb8-ccdeac8633f5/storage/rootfs-bfbb2b86.ext4",
    "size_bytes": 52428800,
    "type": "ext4",
    "status": "attached",
    "host_id": "bbab8c75-f516-47ec-987a-828422b2ee5a",
    "created_at": "2025-11-03T08:50:11Z",
    "updated_at": "2025-11-03T08:50:11Z"
  }
]
```

#### Get Volume Details

```http
GET /v1/volumes/{volume_id}
```

**Response:**
```json
{
  "id": "336ae2fd-1b83-4c4d-9778-d34d587374f9",
  "name": "Shiro Logic6 (rootfs-bfbb2b86.ext4)",
  "description": "Rootfs for VM: Shiro Logic6",
  "path": "/srv/fc/vms/0fa7cf15-f92d-49c7-bfb8-ccdeac8633f5/storage/rootfs-bfbb2b86.ext4",
  "size_bytes": 52428800,
  "type": "ext4",
  "status": "attached",
  "host_id": "bbab8c75-f516-47ec-987a-828422b2ee5a",
  "created_at": "2025-11-03T08:50:11Z",
  "updated_at": "2025-11-03T08:50:11Z"
}
```

#### Create Volume

```http
POST /v1/volumes
Content-Type: application/json

{
  "name": "Data Volume 1",
  "description": "Additional storage for production",
  "path": "/srv/fc/volumes/data-vol-1.ext4",
  "size_bytes": 10737418240,
  "type": "ext4",
  "host_id": "bbab8c75-f516-47ec-987a-828422b2ee5a"
}
```

#### Update Volume Status

```http
PATCH /v1/volumes/{volume_id}/status
Content-Type: application/json

{
  "status": "available"
}
```

#### Delete Volume

```http
DELETE /v1/volumes/{volume_id}
```

**Requirements:**
- Volume status must be `available` (not attached)

#### Attach Volume

```http
POST /v1/volumes/{volume_id}/attach
Content-Type: application/json

{
  "vm_id": "cd2d0de0-5cfb-49a5-a84d-bb01684fd988",
  "drive_id": "vdb"
}
```

#### Detach Volume

```http
POST /v1/volumes/{volume_id}/detach/{vm_id}
```

#### List Attachments

```http
GET /v1/volumes/{volume_id}/attachments
```

**Response:**
```json
[
  {
    "id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
    "volume_id": "336ae2fd-1b83-4c4d-9778-d34d587374f9",
    "vm_id": "0fa7cf15-f92d-49c7-bfb8-ccdeac8633f5",
    "drive_id": "rootfs",
    "attached_at": "2025-11-03T08:50:11Z"
  }
]
```

#### List Volumes by Host

```http
GET /v1/volumes?host_id={host_id}
```

#### List Volumes by Status

```http
GET /v1/volumes?status=available
GET /v1/volumes?status=attached
```

---

## UI Features

### Volumes Page

Location: `apps/ui/app/(dashboard)/volumes/page.tsx`

Features:
- **List all volumes** across all hosts
- **Search and filter** by name, type, status, host
- **View volume details** (path, size, type, attachments)
- **Attachment count** per volume
- **Create new volumes** (manual)
- **Update volume status**
- **Delete volumes** (if not attached)
- **Attach/detach** volumes to/from VMs

### Volume Table

Component: `apps/ui/components/volume/volume-table.tsx`

Columns:
- **Name**: Volume name
- **Size**: Volume size in GB
- **Type**: ext4, qcow2, raw
- **Status**: available, attached, in-use
- **Path**: Filesystem path
- **Host**: Associated host name
- **Attachments**: Number of VMs using this volume
- **Actions**: Attach, Detach, Delete

### Volume Creation Dialog

Component: `apps/ui/components/volume/volume-create-dialog.tsx`

Fields:
- **Name**: Volume name (required)
- **Description**: Volume description (optional)
- **Path**: Filesystem path (required)
- **Size**: Volume size in bytes (required)
- **Type**: ext4, qcow2, raw (required)
- **Host**: Associated host (required)

---

## Common Use Cases

### Use Case 1: Track All VM Rootfs Volumes

**Goal**: Monitor disk usage across all VMs

**Solution**: Volumes page automatically shows all rootfs volumes with sizes.

```bash
# List all volumes
GET /v1/volumes

# Result: All VM rootfs volumes with names like:
# - "VM-1 (rootfs-abc123.ext4)"
# - "VM-2 (rootfs-def456.ext4)"
```

**Benefits**:
- See total disk usage per host
- Identify large rootfs volumes
- Track volume growth over time

### Use Case 2: Add Data Disk to Running VM

**Goal**: Add additional storage to a VM

**Solution**:

```bash
# Step 1: Create volume file
sudo dd if=/dev/zero of=/srv/fc/volumes/data-vol-1.ext4 bs=1M count=10240
sudo mkfs.ext4 /srv/fc/volumes/data-vol-1.ext4

# Step 2: Register volume
POST /v1/volumes
{
  "name": "Data Volume 1",
  "path": "/srv/fc/volumes/data-vol-1.ext4",
  "size_bytes": 10737418240,
  "type": "ext4",
  "host_id": "bbab8c75-f516-47ec-987a-828422b2ee5a"
}

# Step 3: Attach to VM
POST /v1/volumes/{volume_id}/attach
{
  "vm_id": "{vm_id}",
  "drive_id": "vdb"
}

# Step 4: Inside VM, mount the disk
mount /dev/vdb /mnt/data
```

### Use Case 3: Move Volume Between VMs

**Goal**: Detach volume from one VM and attach to another

**Solution**:

```bash
# Step 1: Detach from VM 1
POST /v1/volumes/{volume_id}/detach/{vm_id_1}

# Step 2: Attach to VM 2
POST /v1/volumes/{volume_id}/attach
{
  "vm_id": "{vm_id_2}",
  "drive_id": "vdb"
}
```

**Use Cases**:
- Move data between VMs
- Backup/restore workflows
- Blue-green deployments

### Use Case 4: Cleanup Orphaned Volumes

**Goal**: Find and delete volumes not attached to any VM

**Solution**:

```bash
# Step 1: List available volumes
GET /v1/volumes?status=available

# Step 2: Review list, identify orphaned volumes

# Step 3: Delete orphaned volumes
DELETE /v1/volumes/{volume_id}

# Step 4: Optionally delete files
sudo rm {volume_path}
```

### Use Case 5: Volume Snapshots (via VM Snapshots)

**Goal**: Create point-in-time snapshot of volume

**Solution**:

```bash
# Create VM snapshot (includes all attached volumes)
POST /v1/vms/{vm_id}/snapshots
{
  "name": "Before Update",
  "snapshot_type": "Full"
}

# Restore from snapshot (creates new VM with volume copy)
POST /v1/snapshots/{snapshot_id}/instantiate
{
  "vm_name": "Restored VM"
}
```

---

## Troubleshooting

### Issue: Volume Size Showing as 0 GB

**Symptoms**: Volumes appear in registry but size shows 0

**Diagnosis**:
```bash
# Check actual file size
ls -lh /srv/fc/vms/{vm-id}/storage/rootfs-{uuid}.ext4

# Check database
psql $DATABASE_URL -c "SELECT name, size_bytes FROM volume WHERE id = '{volume_id}';"
```

**Root Cause**: File size calculation may fail during auto-registration if file is being copied.

**Solution**:
- File size is calculated during registration using `fs::metadata()`
- If file is still being copied, size may be 0 or partial
- Volume registration happens after copy completes, so this should not occur
- If it does occur, update volume size manually:

```bash
# Get actual file size
FILE_SIZE=$(stat -f%z /path/to/volume.ext4)  # macOS
FILE_SIZE=$(stat -c%s /path/to/volume.ext4)  # Linux

# Update database
psql $DATABASE_URL -c "UPDATE volume SET size_bytes = $FILE_SIZE WHERE id = '{volume_id}';"
```

### Issue: Cannot Delete Volume

**Symptoms**: Volume deletion fails with error

**Diagnosis**:
```bash
# Check volume status
GET /v1/volumes/{volume_id}

# Check attachments
GET /v1/volumes/{volume_id}/attachments
```

**Solution**:
- Detach volume from all VMs first
- Ensure status is `available`
- Then delete volume

### Issue: Volume Not Auto-Registered

**Symptoms**: VM created but volume doesn't appear in registry

**Diagnosis**:
```bash
# Check manager logs
grep "auto-register.*volume" /var/log/manager.log

# Check database
psql $DATABASE_URL -c "SELECT * FROM volume WHERE path LIKE '%{vm-id}%';"
```

**Solution**:
- Check manager logs for auto-registration errors
- Verify rootfs path exists and is readable
- Ensure host_id is valid

### Issue: Duplicate Volume Paths

**Symptoms**: Cannot create volume with error "duplicate key value violates unique constraint"

**Root Cause**: Volume path must be unique across all volumes.

**Solution**:
- Check for existing volume with same path:
  ```bash
  GET /v1/volumes?path={path}
  ```
- Delete existing volume or use different path

### Issue: Volume File Missing

**Symptoms**: Volume exists in registry but file is missing from filesystem

**Diagnosis**:
```bash
# Check if file exists
ls -l {volume_path}

# Check volume record
GET /v1/volumes/{volume_id}
```

**Solution**:
- Volume registry tracks metadata only
- If file is deleted from filesystem, volume record becomes orphaned
- Options:
  1. Restore file from backup
  2. Delete volume record from registry

```bash
# Delete orphaned volume record
DELETE /v1/volumes/{volume_id}
```

---

## Performance Considerations

### Storage Performance

- **Local Storage**: Best performance (NVMe > SSD > HDD)
- **Network Storage**: NFS, iSCSI (higher latency, good for shared storage)
- **Image Format**:
  - **ext4**: Best performance, no overhead
  - **qcow2**: Good for snapshots, slight overhead
  - **raw**: Maximum performance, no features

### Optimization Tips

1. **Use Local Storage**: Keep VM rootfs on local SSD/NVMe
2. **Preallocate Volumes**: Use `fallocate` instead of `dd`
   ```bash
   fallocate -l 10G /srv/fc/volumes/data.ext4
   ```
3. **Avoid qcow2 for IO-Heavy**: Use ext4 or raw for databases
4. **Tune Filesystem**: Use `noatime` mount option
5. **Monitor IOPS**: Track disk I/O with `iostat`

### Storage Sizing

- **Rootfs**: 1-10 GB (depends on OS and applications)
- **Data Volumes**: Application-specific
- **Overprovisioning**: Plan for 20-30% overhead
- **Monitoring**: Track disk usage per host

---

## Best Practices

### Volume Management

1. **Descriptive Names**: Use meaningful volume names
   - ✅ "postgres-db-prod (data-vol-1.ext4)"
   - ❌ "volume-123"

2. **Tag Volumes**: Use description field for metadata
   ```json
   {
     "description": "Production database | Team: Backend | Owner: john@example.com"
   }
   ```

3. **Regular Cleanup**: Remove unused volumes
   ```bash
   # Find available volumes older than 30 days
   SELECT * FROM volume
   WHERE status = 'available'
   AND created_at < now() - interval '30 days';
   ```

4. **Backup Critical Volumes**: Use VM snapshots or file-level backups

5. **Monitor Disk Usage**: Track volume growth
   ```bash
   # Check total volume size per host
   SELECT host_id, SUM(size_bytes) / 1024 / 1024 / 1024 AS total_gb
   FROM volume
   GROUP BY host_id;
   ```

### Security

1. **File Permissions**: Restrict access to volume files
   ```bash
   chmod 600 /srv/fc/vms/{vm-id}/storage/rootfs-{uuid}.ext4
   chown root:root /srv/fc/vms/{vm-id}/storage/rootfs-{uuid}.ext4
   ```

2. **Encryption**: Use encrypted volumes for sensitive data
   ```bash
   # Create encrypted volume
   cryptsetup luksFormat /srv/fc/volumes/encrypted.ext4
   cryptsetup luksOpen /srv/fc/volumes/encrypted.ext4 encrypted-vol
   mkfs.ext4 /dev/mapper/encrypted-vol
   ```

3. **Access Control**: Limit who can attach/detach volumes (future RBAC)

---

## Future Enhancements

Planned volume management features:

1. **Volume Snapshots**: Create snapshots of individual volumes
2. **Volume Cloning**: Clone volumes without VM snapshots
3. **Volume Migration**: Move volumes between hosts
4. **Volume Encryption**: Built-in encryption support
5. **Volume Resize**: Grow/shrink volumes dynamically
6. **Volume Templates**: Pre-configured volume templates
7. **Volume Metering**: Track IOPS, bandwidth per volume
8. **Volume Quotas**: Limit disk usage per user/team
9. **Volume Backup**: Automated backup schedules
10. **Multi-Attach**: Read-only multi-attach support

---

## References

- **ext4 Filesystem**: https://ext4.wiki.kernel.org/
- **qcow2 Format**: https://www.qemu.org/docs/master/system/images.html
- **Firecracker Block Devices**: https://github.com/firecracker-microvm/firecracker/blob/main/docs/api_requests/block.md
- **Linux Storage Management**: https://www.kernel.org/doc/html/latest/admin-guide/devices.html

---

**End of Volume Management Documentation**
