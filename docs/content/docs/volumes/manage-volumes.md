+++
title = "Manage Volumes"
description = "Attach, detach, delete, and organize storage volumes through the web interface"
weight = 73
date = 2025-01-13
+++

This guide will show you how to manage your storage volumes using the web interface. You'll learn how to attach volumes to VMs, detach them safely, delete unused volumes, and keep your storage organized.

---

## Accessing Volume Management

### Navigate to Volumes Page

![Image: Volumes navigation](/images/volumes/nav-volumes.png)

Click **"Volumes"** in the sidebar (under Operations) to access the Volumes page.

### Volume Management Layout

![Image: Volume management page](/images/volumes/manage-layout.png)

The page shows:
- **Search and filters** - Find volumes quickly
- **Volume table** - List with management actions
- **Action buttons** - Attach, detach, delete operations
- **Status indicators** - Attachment status and VM count

---

## Attaching Volumes

### When to Attach

**Attach volumes when**:
- VM needs additional storage
- Adding database storage
- Mounting shared data
- Expanding VM capacity

**Requirements**:
- Volume is available (not attached elsewhere)
- VM is stopped (for some operations)
- Sufficient VM drive slots available

---

### Step 1: Select Volume

![Image: Volume list](/images/volumes/volume-list-attach.png)

Find volume to attach in table:
- Check **Status** shows "Available"
- Click **Attach** button in Actions column

---

### Step 2: Open Attach Dialog

![Image: Attach dialog](/images/volumes/attach-dialog.png)

Attach dialog appears:

**Dialog shows**:
- Volume name and size
- List of available VMs
- Attachment options
- Drive slot selection

---

### Step 3: Select Target VM

![Image: Select VM](/images/volumes/attach-select-vm.png)

Choose VM to attach volume to:

**VM list shows**:
- VM name
- Current status (Running/Stopped)
- Number of attached volumes
- Available drive slots

**Selection tips**:
- Stop VM first (recommended)
- Check VM has available slots
- Verify VM purpose matches volume

---

### Step 4: Choose Attachment Mode

![Image: Attachment mode](/images/volumes/attach-mode.png)

Select attachment mode:

**Read-Write** (default):
- VM can read and write
- Full access to volume
- Standard mode

**Read-Only**:
- VM can only read
- Cannot modify data
- Good for shared data

**When to use read-only**:
- Shared configuration
- Reference data
- Static assets
- Prevent accidental changes

---

### Step 5: Select Drive Slot

![Image: Drive slot](/images/volumes/attach-slot.png)

Choose drive slot (optional):

**Auto-select** (recommended):
- System picks next available slot
- Usually correct choice

**Manual selection**:
- /dev/vdb - Second drive
- /dev/vdc - Third drive
- /dev/vdd - Fourth drive
- etc.

---

### Step 6: Confirm Attachment

![Image: Attach confirm](/images/volumes/attach-confirm.png)

Click **"Attach"** button:

**What happens**:
1. VM configuration updated
2. Volume marked as attached
3. Drive appears in VM (if running)
4. Success notification shown
5. VM count updated

**Attachment time**: 1-5 seconds

---

### After Attachment

![Image: Attachment success](/images/volumes/attach-success.png)

**Verify attachment**:
- Volume shows "Attached" status
- VM count incremented
- Volume appears in VM details

**Next steps**:
1. Start VM (if stopped)
2. Log into VM
3. Mount the volume
4. Verify storage accessible

---

### Mounting in VM

After attachment, mount the volume inside the VM:

**Check available devices**:
```bash
# List block devices
lsblk

# Example output:
NAME   MAJ:MIN RM  SIZE RO TYPE MOUNTPOINT
vda      254:0    0   10G  0 disk
└─vda1   254:1    0   10G  0 part /
vdb      254:16   0  100G  0 disk
```

**Create mount point and mount**:
```bash
# Create directory
sudo mkdir -p /mnt/data

# Mount the volume
sudo mount /dev/vdb /mnt/data

# Verify
df -h /mnt/data
```

**Make permanent** (add to /etc/fstab):
```bash
# Get UUID
sudo blkid /dev/vdb

# Edit fstab
sudo nano /etc/fstab

# Add line (replace UUID with actual):
UUID=abc123... /mnt/data ext4 defaults 0 2

# Test
sudo mount -a
```

---

## Detaching Volumes

### When to Detach

**Detach volumes when**:
- Moving volume to another VM
- VM no longer needs the storage
- Cleaning up unused attachments
- Preparing for VM deletion

**Safe to detach when**:
- VM is stopped
- Volume is unmounted inside VM
- No active I/O operations
- Data is synced to disk

---

### Step 1: Prepare for Detachment

**Before detaching**:

![Image: Prepare detach](/images/volumes/detach-prepare.png)

1. **Stop applications** using the volume
2. **Unmount** volume inside VM:
   ```bash
   # Unmount volume
   sudo umount /mnt/data

   # Verify unmounted
   mount | grep /mnt/data
   ```
3. **Stop the VM** (recommended)

---

### Step 2: Select Volume

![Image: Detach button](/images/volumes/detach-button.png)

Find volume in table:
- Check **Status** shows "Attached"
- Click **Detach** button in Actions column

---

### Step 3: Confirm Detachment

![Image: Detach confirm](/images/volumes/detach-confirm.png)

Detach confirmation dialog appears:

**Dialog shows**:
- Volume name
- Currently attached VM
- Warning about data safety
- Confirm/Cancel buttons

**Warning checks**:
- ⚠️ VM is stopped?
- ⚠️ Volume unmounted?
- ⚠️ Data saved?

---

### Step 4: Detach Volume

![Image: Detach success](/images/volumes/detach-success.png)

Click **"Detach"** button:

**What happens**:
1. VM configuration updated
2. Volume marked as available
3. Drive removed from VM
4. Success notification shown
5. VM count decremented

**Detachment time**: 1-2 seconds

---

### After Detachment

**Verify detachment**:
- Volume shows "Available" status
- VM count shows 0
- Volume can be attached elsewhere

**Data safety**:
- ✅ All data remains intact
- ✅ Volume can be re-attached
- ✅ No data loss from detachment

---

## Deleting Volumes

### When to Delete

**Delete volumes when**:
- Volume no longer needed
- Cleaning up old storage
- Freeing disk space
- Removing test volumes

**Safe to delete when**:
- ✅ Volume is not attached to any VM
- ✅ Data is backed up (if important)
- ✅ No plans to use again
- ✅ Team confirms not needed

**Do NOT delete when**:
- ❌ Volume is attached to VM
- ❌ Contains important data (not backed up)
- ❌ Used by production system
- ❌ Uncertain about contents

---

### Step 1: Verify Not Attached

![Image: Check attachments](/images/volumes/check-attached.png)

Before deleting:
- Check **Attached VMs** column shows `0`
- If attached, detach first
- Verify no VM dependencies

---

### Step 2: Select Volume

![Image: Delete button](/images/volumes/delete-button.png)

Find volume to delete in table:
- Ensure **Status** is "Available"
- Click **Delete** button (trash icon)

---

### Step 3: Confirm Deletion

![Image: Delete confirm](/images/volumes/delete-confirm.png)

Deletion confirmation dialog appears:

**Dialog shows**:
- Volume name
- Volume size
- Warning about permanent deletion
- Data loss warning
- Confirm/Cancel buttons

**What gets deleted**:
- ⚠️ Database entry removed
- ⚠️ Volume file deleted (if configured)
- ⚠️ All data lost
- ⚠️ Cannot be undone

**File deletion note**:
- May or may not delete actual file
- Depends on server configuration
- Contact administrator for file cleanup

---

### Step 4: Delete Volume

![Image: Delete success](/images/volumes/delete-success.png)

Click **"Delete"** button:

**What happens**:
1. Volume removed from registry
2. Database entry deleted
3. File may be deleted (contact admin)
4. Success notification shown
5. Volume disappears from table

**Deletion time**: 1-5 seconds

---

### Cannot Delete Attached Volumes

![Image: Cannot delete](/images/volumes/cannot-delete.png)

**Error message**:
```
Cannot delete volume
This volume is currently attached to 1 VM
```

**To delete**:
1. Detach from all VMs first
2. Verify volume is available
3. Then delete the volume

---

## Volume Organization

### Viewing Volume Details

Click on volume name to view details:

![Image: Volume details](/images/volumes/volume-details.png)

**Details shown**:
- Volume name and type
- Size and format
- Storage path
- Creation date
- Attached VMs list
- Attachment details
- Usage statistics

---

### Tracking VM Attachments

See which VMs use each volume:

![Image: VM attachments](/images/volumes/vm-attachments.png)

**Attachment information**:
- VM name and status
- Drive slot (/dev/vdb, etc.)
- Attachment mode (RW/RO)
- Attached since date
- Link to VM details

**Use this for**:
- Identify volume usage
- Plan volume migrations
- Track volume dependencies
- Audit storage allocation

---

### Volume Usage Patterns

Monitor volume usage:

![Image: Usage patterns](/images/volumes/usage-patterns.png)

**Metrics shown**:
- Total volumes count
- Attached vs. available
- Total storage used
- Largest volumes
- Most used volumes

**Use for**:
- Capacity planning
- Cost optimization
- Performance monitoring
- Cleanup identification

---

## Storage Cleanup

### Identify Unused Volumes

Find volumes to clean up:

![Image: Unused volumes](/images/volumes/unused-volumes.png)

**Filter to find**:
1. Set status filter to "Available"
2. Sort by creation date (oldest first)
3. Review each volume
4. Delete if no longer needed

**Cleanup candidates**:
- Old test volumes
- Temporary scratch space
- Obsolete data volumes
- Duplicate volumes

---

### Bulk Cleanup Process

**Systematic cleanup**:

1. **Identify candidates**:
   - Filter "Available" status
   - Sort by date (oldest)
   - Check size (free large volumes)

2. **Verify not needed**:
   - Check with team
   - Review volume purpose
   - Confirm no dependencies

3. **Delete one by one**:
   - Delete button for each
   - Confirm each deletion
   - Verify space freed

4. **Document cleanup**:
   - Keep deletion log
   - Note freed space
   - Update team

---

## Common Tasks

### Task: Add Storage to Running Application

**Scenario**: Database needs more storage

**Steps**:
1. Create new data volume (100 GB)
2. Stop database VM
3. Attach volume to VM
4. Start database VM
5. Log into VM
6. Mount volume to `/var/lib/postgresql/data2`
7. Configure database to use new location
8. Restart database service

---

### Task: Move Volume Between VMs

**Scenario**: Transfer data volume from VM1 to VM2

**Steps**:
1. Stop VM1
2. Unmount volume inside VM1 (if mounted)
3. Stop VM1 completely
4. Detach volume from VM1
5. Attach volume to VM2
6. Start VM2
7. Mount volume inside VM2
8. Verify data accessible

---

### Task: Share Read-Only Data

**Scenario**: Multiple VMs need access to reference data

**Steps**:
1. Create data volume with shared data
2. Attach to VM1 (read-only mode)
3. Attach to VM2 (read-only mode)
4. Attach to VM3 (read-only mode)
5. Each VM can read, none can modify
6. Data remains consistent across all VMs

---

### Task: Free Up Server Storage

**Scenario**: Server running low on disk space

**Steps**:
1. Go to Volumes page
2. Filter to "Available" status
3. Sort by size (largest first)
4. Review each large unused volume
5. Backup important data (if needed)
6. Delete unused volumes
7. Verify freed space with administrator

---

## Best Practices

### Attachment Best Practices

✅ **Stop VM before attaching**:
- Prevents filesystem issues
- Ensures clean attachment
- Recommended approach

✅ **Use meaningful mount points**:
```
/mnt/data - Generic data
/mnt/postgres - Database storage
/mnt/uploads - User uploads
/mnt/logs - Log storage
```

✅ **Make mounts permanent**:
- Add to /etc/fstab
- Survives reboots
- Consistent mount points

---

### Detachment Best Practices

✅ **Always unmount first**:
```bash
# Check what's using the mount
lsof /mnt/data

# Stop applications
sudo systemctl stop myapp

# Unmount
sudo umount /mnt/data
```

✅ **Stop VM for safety**:
- Prevents data corruption
- Ensures clean detachment
- Recommended approach

✅ **Verify no processes**:
- Check with `lsof`
- Stop services first
- Sync data to disk

---

### Deletion Best Practices

✅ **Backup before deleting**:
- Copy important data
- Verify backup successful
- Test backup restore

✅ **Double-check attachments**:
- Verify 0 VMs attached
- Check no dependencies
- Confirm with team

✅ **Document deletions**:
```
Deleted: postgres-test-old
Date: 2025-01-13
Size: 100 GB freed
Reason: Obsolete test data
Deleted by: admin@company.com
```

---

## Troubleshooting

### Issue: Cannot Attach Volume

**Symptoms**:
- Attach button disabled
- Error message appears

**Possible causes**:
1. Volume already attached elsewhere
2. VM has no available slots
3. VM is running (some cases)
4. Permission denied

**Solution**:
1. Check if volume attached to another VM
2. Verify VM has available drive slots
3. Stop VM and try again
4. Contact administrator for permissions

---

### Issue: Cannot Detach Volume

**Symptoms**:
- Detach fails
- Error message appears

**Possible causes**:
1. Volume is root filesystem
2. VM is running
3. Volume is mounted
4. Active I/O operations

**Solution**:
1. Cannot detach root volumes
2. Stop the VM first
3. Unmount volume inside VM
4. Wait for I/O to complete

---

### Issue: Volume Not Visible in VM

**Symptoms**:
- Attached but not showing in VM
- `lsblk` doesn't show device

**Possible causes**:
1. VM not restarted
2. Hotplug not supported
3. Wrong drive slot
4. VM configuration issue

**Solution**:
1. Stop and start VM
2. Check VM supports hotplug
3. Verify drive slot configuration
4. Contact administrator

---

### Issue: Mount Fails

**Symptoms**:
- Cannot mount volume
- Mount command errors

**Possible causes**:
1. Wrong filesystem type
2. Volume not formatted
3. Permission denied
4. Already mounted

**Solution**:
```bash
# Check filesystem type
sudo blkid /dev/vdb

# Try specific filesystem
sudo mount -t ext4 /dev/vdb /mnt/data

# Check if already mounted
mount | grep vdb

# Check permissions
ls -ld /mnt/data
```

---

### Issue: Accidental Deletion

**Symptoms**:
- Deleted wrong volume
- Need to recover data

**Solution**:
1. Contact administrator immediately
2. Check if file still on server
3. Restore from backup (if available)
4. Volume may be recoverable (ask admin)

**Prevention**:
- Read confirmation dialog carefully
- Double-check volume name
- Keep backups of important data
- Use naming conventions

---

## Quick Reference

### Volume Actions

| Action | Requires | Cannot Do If |
|--------|----------|--------------|
| Attach | Available volume, VM | Already attached |
| Detach | VM stopped | Root volume |
| Delete | Not attached | VMs attached |
| View Details | Any time | - |

### Attachment Modes

| Mode | Access | Use Case |
|------|--------|----------|
| Read-Write | Full | Data storage |
| Read-Only | Read | Shared data |

### Common Mount Points

| Purpose | Mount Point |
|---------|-------------|
| Database | /var/lib/{dbname}/data |
| Web uploads | /var/www/uploads |
| Logs | /var/log/{app} |
| Shared data | /mnt/shared |
| Generic data | /mnt/data |

---

## Next Steps

- **[Browse Volumes](browse-volumes/)** - Find volumes to manage
- **[Create Volumes](create-volumes/)** - Add new storage
- **[Volumes Overview](../)** - Learn about volume types
- **[Users](/docs/users/)** - Manage user accounts and access control
- **[VM Management](/docs/vm/manage-vm/)** - Manage VM storage settings
