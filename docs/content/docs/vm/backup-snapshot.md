+++
title = "Backup & Snapshot"
description = "Create backups and restore VMs using snapshots"
weight = 34
date = 2025-12-16
+++

Learn how to protect your VMs with snapshots for quick backup and recovery.

---

## What are Snapshots?

Snapshots capture the complete state of a VM at a specific point in time:

- **Full VM State**: Memory, disk, and configuration
- **Instant Creation**: Takes seconds to create
- **Quick Restore**: Restore VM in seconds
- **Multiple Snapshots**: Keep several backup points

<!-- **[IMAGE: snapshot-concept.png - Diagram showing VM → Snapshot → Restore]** -->
![IMAGE: snapshot-concept.png - Diagram showing VM → Snapshot → Restore](/images/vm/snapshot-concept.png)

### Use Cases

**Before Risky Changes**:
```
Create Snapshot → Make Changes → Success? Keep | Failure? Restore
```

**Regular Backups**:
- Daily snapshots of production VMs
- Before system updates
- Before application deployments

**Testing & Development**:
- Save clean state before testing
- Restore to clean state between tests
- Experiment safely

**Disaster Recovery**:
- Quick recovery from failures
- Rollback from bad updates
- Restore from accidental deletions

---

## Creating a Snapshot

### Step 1: Navigate to VM

1. Go to **Virtual Machines** page
2. Click on the VM you want to snapshot
3. Click the **Snapshots** tab

<!-- **[IMAGE: vm-snapshots-tab.png - Snapshots tab highlighted]** -->
![IMAGE: vm-snapshots-tab.png - Snapshots tab highlighted](/images/vm/vm-snapshots-tab.png)

### Step 2: Create Snapshot

Click the **Create Snapshot** button:

<!-- **[IMAGE: create-snapshot-button.png - Create Snapshot button in toolbar]** -->
![IMAGE: create-snapshot-button.png - Create Snapshot button in toolbar](/images/vm/create-snapshot-button.png)

A dialog will appear:

<!-- **[IMAGE: create-snapshot-dialog.png - Create snapshot dialog with fields]** -->
![IMAGE: create-snapshot-dialog.png - Create snapshot dialog with fields](/images/vm/create-snapshot-dialog.png)

### Step 3: Enter Snapshot Details

**Snapshot Name**:
- Use descriptive names
- Include date/time or purpose
- Examples:
  - `before-upgrade-2025-12-16`
  - `clean-install`
  - `before-database-migration`
  - `daily-backup-20251216`

**Description** (Optional):
```
Before upgrading to PostgreSQL 15
Installed packages: postgresql-14, nginx, nodejs
```

### Step 4: Create

Click **Create** to start the snapshot process:

<!-- **[IMAGE: snapshot-creating.png - Progress indicator]** -->
![IMAGE: snapshot-creating.png - Progress indicator](/images/vm/snapshot-creating.png)

**What happens**:
1. VM state is paused briefly
2. Memory contents are saved
3. Disk state is captured
4. VM resumes automatically

**Time**: Usually 5-15 seconds depending on VM size

### Step 5: Snapshot Created

The new snapshot appears in the list:

<!-- **[IMAGE: snapshot-list.png - List showing created snapshot]** -->
![IMAGE: snapshot-list.png - List showing created snapshot](/images/vm/snapshot-list.png)

You'll see:
- Snapshot name
- Creation date/time
- Size (disk + memory)
- Actions (Restore, Delete)

## Restoring from Snapshot

**Warning**: Restoring replaces current VM state with snapshot!

### Before Restoring

**Important considerations**:
- Current VM data will be lost
- VM will revert to snapshot time
- Create new snapshot of current state if needed
- Stop VM before restoring (recommended)

### Restore Process

1. Go to VM **Snapshots** tab
2. Find the snapshot you want to restore
3. Click **Restore** button

<!-- **[IMAGE: snapshot-restore-button.png - Restore button on snapshot entry]** -->
![IMAGE: snapshot-restore-button.png - Restore button on snapshot entry](/images/vm/snapshot-restore-button.png)

4. Confirm the restoration:

<!-- **[IMAGE: restore-confirm-dialog.png - Restore confirmation with warning]** -->
![IMAGE: restore-confirm-dialog.png - Restore confirmation with warning](/images/vm/restore-confirm-dialog.png)


**Confirmation message**:
```
⚠️  Warning: This will restore VM to snapshot state.
Current data will be lost. This cannot be undone.

Snapshot: before-upgrade-2025-12-16
Created: 2025-12-16 10:30:00

Type VM name to confirm: my-vm
```

5. Type VM name and click **Confirm Restore**

### Restoration Progress

**[IMAGE: restore-progress.png - Restoration in progress]**

The system will:
1. Stop the VM (if running)
2. Replace disk with snapshot
3. Restore memory state
4. Restart the VM

**Time**: Usually 10-30 seconds

### Verify Restoration

After restoration:

**[IMAGE: restore-complete.png - VM running after restore]**

1. Check VM is in "Running" state
2. Access console and verify data
3. Test that everything works as expected
4. Check timestamp - should match snapshot time

**Example verification**:
```bash
# Check system uptime (should show recent boot)
uptime

# Check file timestamps
ls -la /var/log/

# Verify applications are running
ps aux | grep nginx
```

---

## Managing Snapshots

### Renaming a Snapshot

1. Click **⋮** menu next to snapshot
2. Select **Rename**
3. Enter new name
4. Click **Save**

**[IMAGE: snapshot-rename.png - Rename dialog]**

### Deleting a Snapshot

**Caution**: Deleted snapshots cannot be recovered!

1. Click **⋮** menu next to snapshot
2. Select **Delete**
3. Confirm deletion

**[IMAGE: snapshot-delete-confirm.png - Delete confirmation]**

**What happens**:
- Snapshot is permanently removed
- Disk space is freed
- Cannot be restored after deletion
- VM is not affected

---

## Snapshot Types

### Full Snapshots

Captures complete VM state:
- ✅ All disk data
- ✅ Memory contents
- ✅ Configuration
- ✅ Independent restore point

**Size**: Matches VM disk + memory size

**Use when**: Creating major backup points

### Incremental Snapshots

Captures only changes since last snapshot:
- ✅ Changes since parent snapshot
- ✅ Smaller size
- ✅ Faster creation
- ⚠️ Requires parent snapshot

**Size**: Only changed data

**Use when**: Frequent backups of same VM

**[IMAGE: snapshot-types.png - Diagram showing full vs incremental]**

---

## Best Practices

### Snapshot Naming

**Good names**:
```
before-update-2025-12-16
after-install-postgres
clean-os-install
production-daily-20251216-0300
pre-migration-backup
```

**Bad names**:
```
snapshot1
backup
test
20251216
```

### Snapshot Frequency

**Production VMs**:
- Daily snapshots at off-peak hours
- Before any changes
- Keep last 7 daily snapshots
- Monthly long-term snapshots

**Development VMs**:
- Before major changes
- After successful configurations
- Clean state snapshots
- Keep 2-3 recent snapshots

**Test VMs**:
- Before each test cycle
- Clean baseline state
- Delete after testing complete

### Snapshot Retention

**Recommended retention policy**:

| Snapshot Type | Keep For | Example |
|---------------|----------|---------|
| Daily | 7 days | Last week's backups |
| Weekly | 4 weeks | Last month |
| Monthly | 3-12 months | Quarterly archives |
| Before Changes | Until verified | 1-2 weeks |

**Delete old snapshots**:
- Free up disk space
- Reduce clutter
- Focus on important backups
- Automate cleanup if possible

### Storage Management

**Monitor snapshot storage**:
1. Go to **Snapshots** page
2. Check total size
3. Review storage usage

**[IMAGE: snapshot-storage-usage.png - Storage usage dashboard]**

**Optimize storage**:
- Delete unnecessary snapshots
- Use incremental snapshots
- Compress old snapshots
- Archive to external storage

---

## Disaster Recovery

### Recovery Plan

**Scenario**: VM crashed and won't boot

**Recovery steps**:

1. **Assess damage**:
   - Try restarting VM
   - Check error messages
   - Identify last known good state

2. **Find latest snapshot**:
   - Go to VM Snapshots tab
   - Identify most recent working snapshot
   - Note what data will be lost

3. **Restore snapshot**:
   - Stop failed VM
   - Click Restore on chosen snapshot
   - Confirm restoration
   - Wait for completion

4. **Verify recovery**:
   - Check VM starts successfully
   - Test critical services
   - Verify data integrity
   - Document what was lost

5. **Prevent recurrence**:
   - Identify failure cause
   - Implement fixes
   - Create new snapshot of fixed state

**[IMAGE: disaster-recovery-flow.png - Recovery flowchart]**

### Testing Restores

**Monthly practice**:
1. Choose non-critical VM
2. Create test snapshot
3. Make some changes
4. Restore from snapshot
5. Verify restoration worked
6. Delete test snapshot

**Why test?**:
- Verify backups are valid
- Practice recovery process
- Build confidence
- Find issues before emergencies

---

## Troubleshooting

### Issue: Snapshot Creation Fails

**Problem**: Error message when creating snapshot

**[IMAGE: troubleshoot-snapshot-fail.png - Snapshot creation error]**

**Solutions**:
1. Check available disk space on host
2. Ensure VM is in stable state
3. Try stopping VM first, then snapshot
4. Reduce snapshot frequency
5. Contact administrator if disk full

---

### Issue: Restore Takes Too Long

**Problem**: Restoration stuck or very slow

**Solutions**:
1. Wait - large VMs take longer (can be minutes)
2. Check network connection to server
3. Refresh browser after 5 minutes
4. Check VM status directly
5. Contact administrator if >10 minutes

---

### Issue: Can't Delete Snapshot

**Problem**: Delete button greyed out

**Solutions**:
1. Check if snapshot is in use
2. Stop dependent VMs
3. Wait for other operations to complete
4. Refresh the page
5. Check permissions

---

### Issue: Snapshot Missing

**Problem**: Expected snapshot not in list

**Solutions**:
1. Refresh the browser page
2. Check you're looking at correct VM
3. Check All Snapshots page
4. Verify snapshot wasn't auto-deleted
5. Check with team if someone deleted it

---

## Advanced Tips

### Pre-Snapshot Checklist

Before creating important snapshots:

```bash
# In VM console/SSH

# 1. Stop services gracefully
systemctl stop nginx
systemctl stop postgresql

# 2. Sync filesystem
sync

# 3. Clear cache (optional)
sync; echo 3 > /proc/sys/vm/drop_caches

# 4. Create marker file
echo "Snapshot created at $(date)" > /root/snapshot-$(date +%Y%m%d).txt
```

Then create the snapshot.

**Why?**:
- Ensures consistent state
- Prevents corruption
- Makes restore cleaner

### Snapshot Metadata

Add useful metadata in description:

```
Created: 2025-12-16 15:30:00
Purpose: Before PostgreSQL 15 upgrade
Installed: PostgreSQL 14.5, Nginx 1.24, Node.js 20
Services Running: web-api, background-worker
IP Address: 192.168.1.100
Last Updated: 2025-12-15
```

Helps identify correct restore point later!

---

## Next Steps

- **[Monitoring](monitoring/)** - Monitor VM performance
- **[Manage VM](manage-vm/)** - VM lifecycle operations
- **[Create VM](create-vm/)** - Create new VMs
