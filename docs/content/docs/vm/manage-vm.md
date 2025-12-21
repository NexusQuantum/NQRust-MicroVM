+++
title = "Manage VMs"
description = "Start, stop, pause, resume, and delete virtual machines"
weight = 33
date = 2025-12-16
+++

Learn how to manage the lifecycle of your virtual machines through the web interface.

---

## Accessing VM Management

Navigate to **Virtual Machines** page from the sidebar to see all your VMs:

![Image: VMs List Page](/images/vm/manage-vms-page.png)

The VMs page provides:
- **Search bar** - Find VMs by name or ID
- **State filter** - Filter by All States, Running, Stopped, or Paused
- **VM table** - List of all VMs with details and actions
- **Quick create** - Create VM from template (see below)

---

## VM Lifecycle States

A VM can be in one of several states:

![Image: VM state badges](/images/vm/vm-state-badges.png)

| State | Description | Actions Available |
|-------|-------------|-------------------|
| **Stopped** | VM is created but not running | Start, Delete |
| **Running** | VM is active and using resources | Stop, Pause |
| **Paused** | VM is frozen in memory | Resume, Delete |

**State transitions**:
- Stopped → Start → Running
- Running → Stop → Stopped
- Running → Pause → Paused
- Paused → Resume → Running
- Paused → Delete → (Removed)
- Stopped → Delete → (Removed)

**Note**: You **cannot delete a running VM** - you must stop or pause it first.

---

## Starting a VM

### Start from VMs List Page

When a VM is in **Stopped** state, you can start it directly from the VM table:

![Image: Start button on VM list](/images/vm/vm-action-start.png)

**Steps**:
1. Go to **Virtual Machines** page
2. Find your stopped VM (red "Stopped" badge)
3. In the **Actions** column, click the **Start** button (▶ Play icon)


The VM state will change from "Stopped" to "Running" within 1-2 seconds.

### What Happens

Behind the scenes, the system will:
1. ✓ Allocate resources on the host
2. ✓ Boot the kernel
3. ✓ Initialize the operating system
4. ✓ Configure network interface (assigns IP via DHCP)
5. ✓ Start all services

**Time**: Usually completes in **1-2 seconds**

### Verify VM Started

After starting, check the VM status:

![Image: VM showing Running status](/images/vm/vm-running-badge.png)

You should see:
- Status badge: **Running** (green)
- **Guest IP** column shows assigned IP address
- **CPU** column shows usage percentage
- **Memory** column shows usage percentage

**Next**: Click the VM name to access the detail page for console, metrics, and more.

---

## Stopping a VM

### Stop a Running VM

**⚠️ Warning**: Stopping a VM is like pulling the power plug - any unsaved work will be lost!

From the VMs list page:

![Image: Stop button on running VM](/images/vm/vm-action-stop.png)

**Steps**:
1. Locate your running VM (green "Running" badge)
2. In the **Actions** column, you'll see two buttons:
   - **Pause** button (❚❚ icon) - freezes VM
   - **Stop** button (◼ Square icon) - stops VM
3. Click the **Stop** button

The VM will immediately stop and change to "Stopped" state.

### Safe Stop (Recommended)

**Best practice**: Shutdown the OS gracefully before stopping:

1. Click the VM name to open VM detail page
2. Go to **Terminal** tab
3. Login to the VM console
4. Run shutdown command:

```bash
# For Alpine/Debian/Ubuntu
shutdown now

# Or
poweroff
```

5. Wait 5-10 seconds for graceful shutdown
6. Return to VMs list and click **Stop** if needed

This ensures:
- All processes exit cleanly
- Filesystems are properly unmounted
- Data is saved to disk
- No corruption risks

### What Happens

When you click Stop, the system will:
1. ✓ Force terminate all VM processes
2. ✓ Release network interface
3. ✓ Deallocate CPU resources
4. ✓ Keep memory snapshot for quick restart
5. ✓ Change state to **Stopped**

**Time**: Usually instant (< 1 second)

![Image: VM in stopped state](/images/vm/vm-stopped-badge.png)

### When to Stop a VM

✅ **Stop when**:
- VM not needed for extended period
- Performing maintenance or updates
- Saving CPU resources
- Troubleshooting issues
- Preparing to delete

⚠️ **Avoid stopping when**:
- Production services are running
- Long-running tasks in progress (backups, builds, etc.)
- Other services/VMs depend on this one

---

## Pausing a VM

### Pause a Running VM

Pausing **freezes** the VM in its current state - useful for:
- Temporarily freeing CPU resources
- Debugging (pause to inspect state)
- Quick resume later without reboot

From the VMs list page:

![Image: Pause button on running VM](/images/vm/vm-action-pause.png)

**Steps**:
1. Locate your running VM (green "Running" badge)
2. In the **Actions** column, click the **Pause** button (❚❚ icon)

The VM will instantly freeze and change to "Paused" state.

### What Happens

When you pause a VM:
1. ✓ All processes freeze instantly
2. ✓ Current state saved to memory
3. ✓ CPU resources freed (0% CPU usage)
4. ✓ Memory remains allocated
5. ✓ Network connection suspended
6. ✓ State changes to **Paused**

**Time**: Instant (< 100ms)

![Image: VM in paused state](/images/vm/vm-paused-badge.png)

**Important**: A paused VM still uses **memory** but **not CPU**!

### When to Pause

✅ **Pause when**:
- Need to temporarily free CPU for other VMs
- Debugging or troubleshooting (inspect frozen state)
- Quick break (resume within hours)
- Testing pause/resume functionality

⚠️ **Don't pause for**:
- Long periods (use Stop instead to free memory)
- Production VMs (may cause timeout issues)
- Network-sensitive applications

---

## Resuming a VM

### Resume from Paused State

From the VMs list page:

![Image: Resume button on paused VM](/images/vm/vm-action-resume.png)

**Steps**:
1. Find your paused VM (orange "Paused" badge)
2. In the **Actions** column, click the **Resume** button (▶ Play icon)

The VM will instantly resume execution.

### What Happens

When you resume a VM:
1. ✓ Process execution restores
2. ✓ VM continues from exact paused point
3. ✓ CPU usage resumes
4. ✓ Network connection re-established
5. ✓ State changes to **Running**

**Time**: Instant (< 100ms)

![Image: VM resumed to running state](/images/vm/vm-running-badge.png)

**Note**: The VM continues **exactly where it left off** - no reboot, no data loss, applications continue running!

### Pause vs Stop Comparison

| Operation | Resume Time | State Preserved | Memory Used | CPU Used | Use Case |
|-----------|-------------|-----------------|-------------|----------|----------|
| **Pause** | ~100ms | ✅ Yes (exact) | ✅ Yes | ❌ No | Short break, debugging |
| **Stop** | ~2 seconds | ❌ No (reboot) | ❌ No | ❌ No | Long break, free all resources |

**Decision Guide**:
- **Need CPU now, may resume soon** → Use **Pause**
- **Won't need VM for hours/days** → Use **Stop**
- **Want to free all resources** → Use **Stop**
- **Debugging/inspection** → Use **Pause**

---

## Deleting a VM

**⚠️ Warning**: Deletion is **permanent** and **cannot be undone**!

### Important: Running VMs Cannot Be Deleted

You **cannot delete a running VM**. The delete button will be **disabled** with a tooltip:

![Image: Delete button disabled on running VM](/images/vm/vm-delete-disabled-running.png)

**"Cannot delete running VM. Stop the VM first."**

**You must either**:
- **Stop** the VM first, OR
- **Pause** the VM first

Then the delete button becomes available.

### Before Deleting

**⚠️ Recommended: Create a snapshot first**:
1. Go to VM detail page → **Snapshots** tab
2. Click **Create Snapshot**
3. Wait for snapshot to complete
4. Now you can safely delete (snapshot can restore VM later)

See [Backup & Snapshot](backup-snapshot/) guide.

**Check these before deleting**:
- ✅ Data backed up or not needed
- ✅ No other services depend on this VM
- ✅ No active connections
- ✅ Snapshot created (if you may need to restore)

### Delete a VM

From the VMs list page:

![Image: Delete button on stopped/paused VM](/images/vm/vm-action-delete.png)

**Steps**:
1. **Stop or pause the VM** (delete button won't work on running VMs)
2. In the **Actions** column, click the **Delete** button (🗑️ Trash icon)
3. A confirmation dialog will appear:

![Image: Delete confirmation dialog](/images/vm/vm-delete-confirm.png)

4. Click **Delete** to confirm

The VM will be permanently removed.

### What Gets Deleted

When you delete a VM:

- ✅ **VM configuration** - All settings removed
- ✅ **Runtime state** - Process state cleared
- ✅ **Network configuration** - TAP device released
- ⚠️ **Rootfs volume** - Deleted if not shared with other VMs
- ❌ **Snapshots** - Preserved (can still restore from them)
- ❌ **Images in registry** - Preserved (kernel/rootfs still available)

**Time**: Usually instant (< 1 second)

### Success Notification

After deletion, you'll see a success message:

![Image: VM deleted success notification](/images/vm/vm-deleted-success.png)

**"VM Deleted - [VM name] has been deleted"**

The VM will be removed from the VMs list.

### Recovery After Deletion

If you created a snapshot before deleting:

1. Go to **Snapshots** page (sidebar)
2. Find your VM's snapshot
3. Click **Restore** to create a new VM from the snapshot
4. The VM will be recreated with the same state as when snapshot was taken

**Note**: You cannot restore a deleted VM unless you created a snapshot first!

---

## Quick Create from Template

Instead of using the full wizard, you can quickly create VMs from templates:

![Image: Quick create button](/images/vm/quick-create-button.png)

**Steps**:
1. On the **Virtual Machines** page, click **Quick create** button
2. A dialog will open showing available templates:

![Image: Quick create dialog with template selection](/images/vm/quick-create-dialog.png)

3. **Select a template** by clicking on it (shows checkmark when selected)
4. **Enter a VM name** in the input field
5. Click **Create VM**

![Image: Template selected with VM name](/images/vm/quick-create-selected.png)

The VM will be created instantly with all settings from the template!

**Benefits**:
- ⚡ Much faster than full wizard
- ✅ Pre-configured settings (CPU, memory, images)
- ✅ Consistent configuration across VMs
- ✅ Perfect for creating multiple similar VMs

**Use cases**:
- Create dev environments for team members
- Spin up test VMs quickly
- Deploy standardized configurations
- Rapid prototyping

See [VM Templates](../templates/) for creating and managing templates.

---

## Filtering and Searching VMs

### Search by Name or ID

Use the search bar to quickly find VMs:

![Image: Search bar in VMs page](/images/vm/vm-search-bar.png)

- Type VM name (e.g., "web-server")
- Or type VM ID
- Results filter instantly as you type
- Search is case-insensitive

### Filter by State

Use the state filter dropdown to show only specific VM states:

![Image: State filter dropdown](/images/vm/vm-state-filter.png)

**Options**:
- **All States** - Show all VMs (default)
- **Running** - Only running VMs
- **Stopped** - Only stopped VMs
- **Paused** - Only paused VMs

**Tip**: Combine search and filter for precise results (e.g., search "prod" + filter "Running")

---

## VM Table Information

The VMs table shows detailed information for each VM:

### Columns Explained

1. **Name** - VM name (click to open detail page)
2. **State** - Current status with colored badge
3. **CPU** - vCPU count and current usage %
4. **Memory** - Allocated MiB and current usage %
5. **Guest IP** - IP address assigned to VM (via DHCP)
6. **Host** - Which host/agent is running this VM
7. **Owner** - Who created the VM:
   - **"You"** (green) - VM you created
   - **"Other User"** - Another user's VM
   - **"System"** - System-created VM
8. **Created** - Relative time (e.g., "2 hours ago")
9. **Actions** - Action buttons (Start, Stop, Pause, Resume, Delete)

### Pagination

If you have more than 10 VMs, use pagination at the bottom:

![Image: Pagination controls](/images/vm/vm-pagination.png)

- **10 VMs per page**
- Click page numbers to navigate
- Use Previous/Next arrows

---

## Troubleshooting

### Issue: Can't Start VM

**Symptoms**:
- Start button doesn't respond
- VM stuck in transitioning state
- Error notification appears

**Solutions**:

1. **Check host resources**:
   - Go to **Hosts** page (sidebar)
   - Verify host has available CPU and memory
   - If host is overloaded, stop other VMs first

2. **Verify images exist**:
   - Go to **Registry** → **Images**
   - Check kernel and rootfs images are present
   - Re-upload if missing

3. **Check agent status**:
   - Go to **Hosts** page
   - Verify agent is **Online** (green)
   - If offline, contact administrator

4. **Try deleting and recreating**:
   - Delete the problematic VM
   - Create a new one with same settings

---

### Issue: Delete Button is Disabled

**Symptoms**:
- Delete button is greyed out
- Tooltip says "Cannot delete running VM"

![Image: Disabled delete button with tooltip](/images/vm/vm-delete-disabled-running.png)

**Solution**:

This is **expected behavior** - you cannot delete a running VM.

1. Click the **Stop** button first
2. Wait for state to change to "Stopped"
3. Now the **Delete** button will be enabled
4. Click Delete to remove the VM

**Why this restriction?**
- Prevents accidental deletion of active services
- Ensures graceful shutdown
- Protects data integrity

---

### Issue: VM Won't Stop

**Symptoms**:
- Clicked Stop but VM still shows "Running"
- No state change after 30 seconds

**Solutions**:

1. **Refresh the page** (Ctrl+R or F5)
   - Sometimes the UI needs to refresh state

2. **Wait and retry**:
   - Wait 60 seconds
   - Click Stop again

3. **Graceful shutdown first**:
   - Click VM name → **Terminal** tab
   - Login to console
   - Run: `shutdown now`
   - Wait 10 seconds, then click Stop

4. **Check VM detail page**:
   - Open VM detail page
   - Check if there are errors displayed
   - Try stopping from there

---

### Issue: Paused VM Can't Resume

**Symptoms**:
- Resume button doesn't work
- Error message appears

**Solutions**:

1. **Refresh browser**:
   - Press F5 or Ctrl+R
   - Try Resume again

2. **Stop and start instead**:
   - Click **Stop** button (yes, you can stop a paused VM)
   - Wait for "Stopped" state
   - Click **Start**

3. **Check browser console**:
   - Press F12 to open DevTools
   - Go to Console tab
   - Look for error messages
   - Share with administrator if errors found

4. **Last resort - Delete and restore**:
   - If you have a snapshot, delete the VM
   - Restore from snapshot

---

## Best Practices

### Starting VMs

✅ **Do**:
- Start VMs only when actually needed (save resources)
- Use **Quick create** from templates for consistency
- Verify host has enough resources before starting multiple VMs
- Check agent is **Online** on Hosts page first
- Use search/filter to quickly find the VM you need

⚠️ **Avoid**:
- Starting all VMs at once (may overwhelm host)
- Starting VMs on offline/failed hosts

---

### Stopping VMs

✅ **Do**:
- **Graceful shutdown first** (SSH in and run `shutdown now`)
- Wait 5-10 seconds after graceful shutdown before clicking Stop
- Stop VMs when not in use to free resources
- Create snapshot before stopping critical VMs
- Stop test/dev VMs after work hours

⚠️ **Avoid**:
- Force stopping without graceful shutdown (risk of data corruption)
- Stopping production VMs during business hours
- Stopping VMs with active connections
- Stopping VMs running long-running tasks (backups, builds)

---

### Pausing VMs

✅ **Do**:
- Use pause for **temporary** resource freeing (hours, not days)
- Pause for debugging or troubleshooting
- Resume within reasonable timeframe
- Use when you need CPU but not memory

⚠️ **Avoid**:
- Pausing for extended periods (use Stop instead to free memory)
- Pausing production VMs (may cause timeout/connection issues)
- Pausing network-sensitive applications
- Forgetting to resume (wasted memory)

---

### Deleting VMs

✅ **Do**:
- **ALWAYS create snapshot before deleting** (can restore later)
- Stop or pause the VM first (can't delete running VMs)
- Verify data is backed up elsewhere
- Check no other services/VMs depend on it
- Document why you're deleting (in team notes)
- Double-check you're deleting the right VM

⚠️ **Avoid**:
- Deleting without snapshots (permanent!)
- Deleting shared service VMs
- Deleting production VMs without team approval
- Clicking Delete on wrong VM (check name carefully!)

---

### Resource Management

✅ **Best practices**:
- **Monitor regularly**: Check Hosts page for resource usage
- **Stop unused VMs**: Don't leave test VMs running overnight
- **Use filters**: Filter by "Running" to see what's consuming resources
- **Clean up regularly**: Delete old test/temp VMs
- **Use templates**: Quick create for standardized resource allocation
- **Right-size VMs**: Don't over-allocate CPU/memory

**Weekly cleanup checklist**:
1. Filter VMs by "Running"
2. Stop any unused dev/test VMs
3. Delete old temporary VMs (after creating snapshots)
4. Check Hosts page for resource usage trends

---

### Search and Organization

✅ **Tips**:
- **Naming convention**: Use `<env>-<purpose>-<number>` (e.g., `dev-web-01`, `prod-api-02`)
- **Use search**: Quickly find VMs by typing name
- **Filter by state**: View only Running/Stopped/Paused VMs
- **Owner column**: Easily see which VMs are yours
- **Pagination**: Use page numbers for large VM lists

**Example naming scheme**:
```
dev-alice-ubuntu      (developer's personal VM)
test-backend-api      (testing environment)
staging-database      (staging DB server)
prod-web-01          (production web server #1)
prod-web-02          (production web server #2)
```

---

## Quick Reference

### VM State Actions Summary

| Current State | Available Actions | Click to... |
|---------------|-------------------|-------------|
| **Stopped** | ▶ Start, 🗑️ Delete | Start: Run VM<br>Delete: Remove permanently |
| **Running** | ❚❚ Pause, ◼ Stop | Pause: Freeze instantly<br>Stop: Shutdown VM |
| **Paused** | ▶ Resume, 🗑️ Delete | Resume: Continue execution<br>Delete: Remove |

### Common Workflows

**Daily work**:
1. Morning: Start your dev VM
2. Work: Use Terminal tab for access
3. Evening: Stop your dev VM

**Testing**:
1. Quick create VM from test template
2. Run tests
3. Stop VM
4. Delete VM after confirming results

**Production deployment**:
1. Create snapshot of current prod VM
2. Stop prod VM
3. Create new VM with updated config
4. Test new VM
5. Switch traffic to new VM
6. Keep old VM snapshot for rollback

---

## Next Steps

Now that you know how to manage VMs, explore:

- **[Access VM](access-vm/)** - Connect via terminal and SSH
- **[Monitoring](monitoring/)** - View performance metrics and logs
- **[Backup & Snapshot](backup-snapshot/)** - Protect your VMs with snapshots
- **[Create VM](create-vm/)** - Create VMs using the wizard
