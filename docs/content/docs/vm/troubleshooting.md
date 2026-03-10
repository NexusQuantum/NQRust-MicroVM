+++
title = "Troubleshooting"
description = "Diagnose and resolve common VM issues"
weight = 60
date = 2025-01-08
+++

A reference for diagnosing and resolving common issues across VM creation, management, access, and snapshots.

---

## VM Creation

### Kernel or Rootfs Dropdown is Empty

**Problem**: No options appear in the kernel or rootfs selection dropdowns during VM creation.

**Solution**:
1. Go to **Image Registry** in the sidebar
2. Upload the required kernel and rootfs images (see [Upload Images](../registry/upload-images/))
3. Return to VM creation — dropdowns will now be populated

---

### Not Enough Resources

**Problem**: Error message "Insufficient resources available" when creating a VM.

**Solution**:
- Reduce the vCPU or memory allocation
- Stop unused VMs to free host resources
- Go to **Hosts** page to check available capacity
- Contact your administrator to provision additional capacity

---

### VM Stuck in "Creating" State

**Problem**: VM shows "Creating" for more than 30 seconds.

**Solution**:
1. Refresh the browser page
2. Go to **Hosts** and verify the agent is **Online** (green indicator)
3. If still stuck, delete the VM and recreate it
4. Contact your administrator if the problem persists

---

## VM Management

### Can't Start VM

**Symptoms**: Start button doesn't respond, VM is stuck in a transitioning state, or an error notification appears.

**Solutions**:

1. **Check host resources** — go to **Hosts** and verify the host has available CPU and memory; stop other VMs if the host is overloaded
2. **Verify images exist** — go to **Registry → Images** and confirm the kernel and rootfs are present; re-upload if missing
3. **Check agent status** — on the **Hosts** page, confirm the agent is **Online**; if offline, contact your administrator
4. **Recreate the VM** — delete the problematic VM and create a new one with the same settings

---

### Delete Button is Disabled

**Symptoms**: Delete button is greyed out; tooltip reads "Cannot delete running VM".

![Disabled delete button with tooltip](/images/vm/vm-delete-disabled-running.png)

**Solution**: This is expected behavior — running VMs cannot be deleted.

1. Click **Stop** and wait for the state to change to "Stopped"
2. The **Delete** button will now be enabled

This restriction prevents accidental deletion of active services and protects data integrity.

---

### VM Won't Stop

**Symptoms**: Clicked Stop but the VM still shows "Running" after 30 seconds.

**Solutions**:

1. Refresh the page (Ctrl+R / F5) and check if the state has updated
2. Wait 60 seconds and click Stop again
3. Gracefully shut down from inside the VM first:
   ```bash
   shutdown now
   ```
   Then click Stop after ~10 seconds
4. Open the VM detail page and check for displayed errors

---

### Paused VM Can't Resume

**Symptoms**: Resume button has no effect, or an error message appears.

**Solutions**:

1. Refresh the browser and try Resume again
2. Stop the paused VM, wait for "Stopped" state, then Start it
3. Open browser DevTools (F12 → Console) and look for error messages to share with your administrator
4. If a snapshot exists, delete the VM and restore from snapshot

---

## Console & SSH Access

### Can't Connect to Console

**Problem**: Console shows "Connection failed" or a blank screen.

**Solutions**:
1. Verify the VM is in **Running** state
2. Refresh the browser page
3. Check the browser console (F12) for JavaScript errors
4. Try a different browser (Chrome, Firefox, Edge)
5. Confirm WebSocket connections are not blocked by a firewall or proxy
6. Disable browser extensions temporarily

---

### SSH Connection Refused

**Problem**: `ssh: connect to host <ip> port 22: Connection refused`

**Solutions**:
1. Verify the VM is running and has an IP address
2. Ping the IP: `ping <vm-ip>`
3. Check that the SSH service is running inside the VM (via the console):
   ```bash
   # Alpine
   rc-service sshd status

   # Ubuntu/Debian
   systemctl status sshd
   ```
4. Check firewall rules inside the VM

---

### SSH Permission Denied

**Problem**: `Permission denied (publickey)`

**Solutions**:
1. Confirm the SSH key was configured during VM creation
2. Check you are using the correct key:
   ```bash
   ssh -v root@<vm-ip>
   ```
3. Try password authentication (if enabled):
   ```bash
   ssh -o PreferredAuthentications=password root@<vm-ip>
   ```
4. Recreate the VM with the correct SSH key

---

### Console is Slow or Laggy

**Problem**: Noticeable input delay in the web console.

**Solutions**:
- Use SSH instead of the web console for better performance
- Check your network latency to the server
- Close other browser tabs to free resources
- Try the console in private/incognito mode

---

### Can't Paste in Console

**Problem**: Ctrl+V does not work in the web console.

**Solution**: Use `Ctrl+Shift+V`, or right-click and select **Paste**. Some browsers also support middle-click to paste.

---

## Snapshots

### Snapshot Creation Fails

**Problem**: An error appears when attempting to create a snapshot.

**Solutions**:
1. Check available disk space on the host
2. Ensure the VM is in a stable state (not mid-boot or transitioning)
3. Try stopping the VM before taking the snapshot
4. Contact your administrator if disk space is exhausted

---

### Restore Takes Too Long

**Problem**: Restoration appears stuck or is very slow.

**Solutions**:
1. Wait — large VMs can take several minutes to restore
2. Check your network connection to the server
3. Refresh the browser after 5 minutes and check VM status
4. Contact your administrator if the operation exceeds 10 minutes

---

### Can't Delete Snapshot

**Problem**: Delete button is greyed out.

**Solutions**:
1. Check if the snapshot is currently in use
2. Stop any VMs dependent on the snapshot
3. Wait for other operations to complete, then refresh the page
4. Verify you have the required permissions

---

### Snapshot Missing

**Problem**: An expected snapshot is not visible in the list.

**Solutions**:
1. Refresh the browser page
2. Confirm you are viewing the correct VM
3. Check the **All Snapshots** page
4. Verify the snapshot was not auto-deleted or removed by a team member

---

## Getting Help

If none of the solutions above resolve your issue:

1. Check the **Hosts** page to confirm all agents are online
2. Review browser DevTools (F12) for errors
3. Contact your platform administrator with the VM name, error message, and browser console output
