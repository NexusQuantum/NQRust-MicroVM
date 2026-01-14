+++
title = "Volumes"
description = "Complete guide to managing storage volumes for VMs through the web interface"
weight = 70
date = 2025-01-13
+++

The Volumes registry is your central hub for managing all storage volumes used by VMs. This guide will show you how to browse, create, attach, and manage volumes using the web interface.

---

## What are Volumes?

Volumes are **persistent storage devices** that can be attached to VMs. They provide additional storage capacity beyond the VM's root filesystem.

**Types of volumes**:
- **Data Volumes** - Additional storage for applications and data
- **Rootfs Volumes** - Root filesystem images (auto-registered)
- **Scratch Volumes** - Temporary storage for computation

**Benefits**:
- Persistent data storage
- Reusable across multiple VMs
- Easy volume discovery and management
- Centralized volume tracking
- Attach/detach flexibility

---

## Common Use Cases

### Database Storage

Separate database data from the VM:

```
VM: database-server
- Root filesystem: Ubuntu 22.04 (10 GB)
- Data volume: postgres-data (100 GB)
```

Database data persists even if VM is recreated.

---

### Shared Storage

Share volumes across multiple VMs:

```
Volume: shared-assets (50 GB)
- VM1: web-server-1 (read-only)
- VM2: web-server-2 (read-only)
- VM3: asset-processor (read-write)
```

Multiple VMs can access the same data.

---

### Development Workspaces

Separate code and environment:

```
VM: dev-environment
- Root filesystem: Minimal Ubuntu (5 GB)
- Workspace volume: project-code (20 GB)
```

Preserve code when rebuilding environment.

---

## Volume Types

### Data Volumes

**Purpose**: Store application data, files, databases

**Format**:
- ext4 (recommended)
- xfs
- btrfs

**Typical size**: 10 GB - 500 GB
**Usage**: Attached to VMs as secondary drives

**Common uses**:
- Database storage
- Log files
- User uploads
- Application data
- Cache storage

---

### Rootfs Volumes

**Purpose**: VM root filesystems

**Format**:
- ext4 (recommended)
- qcow2 (compressed)
- raw (uncompressed)

**Typical size**: 2 GB - 20 GB
**Usage**: Boot device for VMs

**Auto-registration**:
- Automatically registered when VM is created
- Tracked in volume registry
- Shows which VM uses it

---

## Volume Properties

Each volume tracks:

**Basic Information**:
- Volume name
- Volume type (data, rootfs)
- File size
- Format (ext4, qcow2, raw)
- Storage path

**Usage Information**:
- Attached VMs count
- VM attachment details
- Mount points
- Read-only status
- Creation date

---

## Accessing the Volumes Registry

### Navigate to Volumes Page

Click **"Volumes"** in the sidebar (under Operations) to access the Volumes page.

![Image: Volumes navigation](/images/volumes/nav-volumes.png)

### Volumes Page Layout

The volumes page displays:
- **Search bar** - Find volumes quickly
- **Filter dropdown** - Filter by type or status
- **Action buttons** - Create new volumes
- **Volume table** - List of all volumes

![Image: Volumes page layout](/images/volumes/page-layout.png)

---

## Volume Lifecycle

### 1. Automatic Volume Registration

Rootfs volumes are automatically registered when you create VMs:

**What happens**:
1. You create a VM with rootfs through the web interface
2. System registers the rootfs as a volume
3. Volume appears in the Volumes page
4. Volume is linked to the VM
5. Attachment is tracked

**Example**:
```
When creating a VM:
- Rootfs: /srv/images/ubuntu-22.04.ext4

Result:
- Volume "ubuntu-22.04" is automatically registered
- Shows 1 VM attached
- Volume appears in your Volumes page
```

---

### 2. Manual Volume Creation

You can create volumes manually before attaching:

**When to use**:
- Planning storage architecture
- Pre-creating data volumes
- Setting up shared storage
- Preparing for VM deployment

**How**:
1. Navigate to Volumes page
2. Click "Create Volume" button
3. Specify size, format, and name
4. Volume is created and ready for attachment

---

### 3. Volume Attachment

Attach volumes to VMs:

**Process**:
1. Create or select volume
2. Attach to target VM
3. Volume appears in VM's storage
4. Mount inside VM (if needed)
5. Start using the storage

**Attachment modes**:
- Read-write (default)
- Read-only (for shared data)

---

### 4. Volume Detachment

Detach volumes from VMs:

**Safe to detach when**:
- VM is stopped
- Data is not in use
- No active file operations

**Process**:
1. Stop the VM
2. Detach volume through web interface
3. Volume becomes available
4. Can be attached to another VM

---

## Volume Formats

### ext4 Format

**Most common format**:
- Native Linux filesystem
- Best performance
- Full feature support
- Direct mounting

**Use for**:
- Data volumes
- Root filesystems
- General purpose storage

**Size**: Any size supported

---

### qcow2 Format

**QEMU Copy-On-Write format**:
- Compressed storage
- Smaller file size
- Snapshot support
- Thin provisioning

**Use for**:
- Root filesystems
- Templates
- Space-constrained storage

**Note**: Slightly slower than ext4

---

### raw Format

**Uncompressed disk image**:
- Simple format
- Fast performance
- No overhead
- Fixed size

**Use for**:
- High-performance needs
- Simple scenarios
- Direct disk access

---

## Storage Location

Volumes are stored on the server:

**Default location**: `/srv/volumes/`

**Organization**:
```
/srv/volumes/
├── data/
│   ├── postgres-data-01.ext4
│   └── shared-assets.ext4
├── rootfs/
│   ├── ubuntu-22.04.ext4
│   └── alpine-3.18.ext4
└── scratch/
    └── temp-storage.ext4
```

**Note**: Storage location is managed by your system administrator. Contact them for storage capacity and management questions.

---

## Volume Registry Features

### Browse and Search

- View all available volumes
- Search by name or VM
- Filter by type or status
- Sort by size or date

### Create Volumes

Multiple creation methods:
- **Create New** - Create empty volume
- **Import Existing** - Register existing files
- **Clone Volume** - Duplicate existing volume

### Manage Volumes

- **Attach** - Connect volume to VM
- **Detach** - Disconnect from VM
- **Delete** - Remove unused volumes
- **Resize** - Expand volume capacity (future)
- **View Details** - See volume properties

---

## Best Practices

### 1. Organize by Purpose

Use clear naming conventions:

```
Good names:
- postgres-data-prod
- webapp-uploads
- logs-archive-2025
- shared-assets-cdn

Avoid:
- volume1
- data
- disk
- storage
```

---

### 2. Separate Data from VMs

**Application data on volumes**:
- VM root: Operating system only
- Data volume: Application data
- Easy to backup data separately
- Rebuild VM without losing data

**Example**:
```
VM: web-server
- Root: ubuntu-22.04 (10 GB)
- Data: webapp-storage (50 GB)
```

---

### 3. Regular Backups

**Backup important volumes**:
- Database volumes
- User data
- Configuration data
- Log archives

**Backup strategies**:
- Scheduled snapshots
- Offsite copies
- Automated backup tools
- Contact administrator for backup setup

---

### 4. Monitor Volume Usage

**Track storage consumption**:
- Check volume sizes regularly
- Identify growing volumes
- Plan capacity expansion
- Delete unused volumes

**Volume registry helps with**:
- Capacity planning
- Cost optimization
- Performance monitoring
- Resource allocation

---

## Security Considerations

### Access Control

**Volume management**:
- Only authorized users should create volumes
- Document volume ownership
- Track who attaches to what
- Regular security audits

**VM access**:
- VMs can only access attached volumes
- Use read-only for shared data
- Encrypt sensitive volumes
- Monitor volume access

---

### Data Protection

**Protect important data**:
- Backup regularly
- Use redundant storage
- Encrypt sensitive volumes
- Control attachment permissions

**Avoid**:
- Sharing volumes between untrusted VMs
- Storing secrets in volumes
- Unencrypted sensitive data
- Detaching volumes while in use

---

## Troubleshooting

### Issue: Volume Not Appearing

**Symptoms**:
- Created volume doesn't show in registry
- Volume list is empty

**Possible causes**:
1. Page not loading
2. Creation failed
3. Filter hiding the volume

**Solution**:
1. Refresh the page (press F5)
2. Check all filters are set to "All"
3. Search for the volume name
4. Check if creation success notification appeared
5. Contact your system administrator if issue persists

---

### Issue: Cannot Attach Volume

**Symptoms**:
- Attach operation fails
- Error message appears

**Possible causes**:
- Volume already attached to another VM
- VM is running (some volumes require stopped VM)
- Insufficient permissions
- Storage path not accessible

**Solution**:
1. Check if volume is attached to another VM
2. Stop the target VM before attaching
3. Verify volume path exists on server
4. Contact your system administrator if issue persists

---

### Issue: Cannot Detach Volume

**Symptoms**:
- Detach button disabled
- Error message appears

**Possible causes**:
- Volume is the root filesystem (cannot detach)
- VM is running
- Volume is in use

**Solution**:
1. Stop the VM first
2. Unmount volume inside VM (if mounted)
3. Root volumes cannot be detached
4. Contact your system administrator if issue persists

---

## Quick Reference

### Volume Types

| Type | Format | Typical Size | Used For |
|------|--------|--------------|----------|
| Data Volume | ext4 | 10-500 GB | Application data |
| Rootfs Volume | ext4/qcow2 | 2-20 GB | VM root filesystem |
| Scratch Volume | ext4/raw | 5-100 GB | Temporary storage |

### Volume Formats

| Format | Pros | Cons | Best For |
|--------|------|------|----------|
| ext4 | Fast, reliable | No compression | General use |
| qcow2 | Compressed, snapshots | Slower | Space saving |
| raw | Simple, fast | No features | High performance |

### Volume Actions

| Action | Status | Notes |
|--------|--------|-------|
| Browse volumes | Available | View all volumes |
| Search volumes | Available | Filter by name/type |
| Create volume | Available | New empty volume |
| Import volume | Available | Register existing file |
| Attach volume | Available | Connect to VM |
| Detach volume | Available | Disconnect from VM |
| Delete volume | Available | If not attached |
| View details | Available | See properties |

---

## Next Steps

- **[Browse Volumes](browse-volumes/)** - Search and explore available volumes
- **[Create Volumes](create-volumes/)** - Create and import volumes
- **[Manage Volumes](manage-volumes/)** - Attach, detach, and organize volumes
- **[Create VM](/docs/vm/create-vm/)** - Use volumes when creating VMs

---

## FAQ

**Q: How many volumes can I attach to a VM?**
A: This depends on your VM configuration and the hypervisor. Typically you can attach multiple volumes (8-16) to a single VM. Contact your system administrator for limits.

**Q: Can I attach the same volume to multiple VMs?**
A: Generally no, for read-write volumes. However, you can attach volumes as read-only to multiple VMs for shared data access. Check with your administrator.

**Q: What happens if I delete a volume?**
A: You cannot delete volumes that are attached to VMs. You must first detach the volume. Deleting a volume permanently removes it and all data.

**Q: How do I resize a volume?**
A: Volume resizing is not yet available through the web interface. Contact your system administrator to resize volumes on the server.

**Q: Can I convert volume formats?**
A: Not directly through the web interface. Your system administrator can convert volumes on the server using tools like qemu-img.

**Q: What if I accidentally detach a volume?**
A: You can re-attach it immediately. No data is lost when detaching volumes. Just attach it back to the VM.

**Q: Do volumes persist after VM deletion?**
A: Yes! Volumes are independent of VMs. When you delete a VM, attached data volumes remain available for reuse. Root filesystem volumes may be deleted depending on configuration.

**Q: How do I backup volumes?**
A: Contact your system administrator for backup solutions. They can set up automated backups using snapshots or copy tools.
