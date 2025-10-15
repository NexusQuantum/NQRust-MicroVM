Implementation Plan: Proper Drive Management
Core Concept
Drives should be pure database records that are applied to Firecracker during VM start/restart, not live operations on running VMs.
1. Database Schema Changes
File: apps/manager/migrations/0009_vm_drive_size.sql (NEW) Changes needed:
-- Add size_bytes column to store original disk size for auto-provisioned drives
ALTER TABLE vm_drive ADD COLUMN size_bytes BIGINT;

-- This allows recreating sparse files on restart with correct size
Why: Currently vm_drive only stores path_on_host, but auto-provisioned drives need size_bytes to recreate the sparse file after stop.
2. Backend: Drive Creation Endpoint
File: apps/manager/src/features/vms/service.rs Function to modify: create_drive (around line 491-545) Current behavior:
Receives CreateDriveReq
Creates disk file via alloc_data_disk
Immediately calls Firecracker to add drive ❌
New behavior:
✅ Validate VM exists
✅ Create disk file if auto-provisioning (path_on_host is null)
✅ Insert record into vm_drive table with all metadata
❌ DO NOT call Firecracker - drives will be applied on next start
✅ Return success
Pseudocode:
pub async fn create_drive(st: &AppState, vm_id: Uuid, req: CreateDriveReq) -> Result<VmDrive> {
    let vm = repo::get(&st.db, vm_id).await?;
    
    // Determine path and size
    let (path, size) = if let Some(p) = req.path_on_host {
        (p, None) // User-provided path
    } else {
        let size = req.size_bytes.unwrap_or(10_737_418_240); // 10GB default
        let path = st.storage.alloc_data_disk(vm_id, size).await?;
        (path, Some(size))
    };
    
    // Insert into database ONLY
    let drive = repo::insert_drive(&st.db, DriveRow {
        id: Uuid::new_v4(),
        vm_id,
        drive_id: req.drive_id,
        path_on_host: path,
        size_bytes: size,
        is_root_device: req.is_root_device.unwrap_or(false),
        is_read_only: req.is_read_only.unwrap_or(false),
        cache_type: req.cache_type,
        io_engine: req.io_engine,
        rate_limiter: req.rate_limiter,
        // timestamps auto-generated
    }).await?;
    
    // NOTE: Drive will be applied to Firecracker on next VM start
    Ok(drive.into())
}
Key insight: This makes drive creation work regardless of VM state (stopped, running, never-started).
3. Backend: Drive Deletion Endpoint
File: apps/manager/src/features/vms/service.rs Function to modify: delete_drive (similar to create_drive) Current behavior:
Deletes drive from database
Calls Firecracker to remove drive ❌
New behavior:
✅ Delete record from vm_drive table
✅ Optionally delete the disk file from filesystem
❌ DO NOT call Firecracker
✅ Return success
Note: If VM is running, the drive will still be attached to Firecracker until restart. This is acceptable behavior.
4. Backend: VM Stop - Don't Delete Storage
File: apps/manager/src/features/vms/service.rs Function to modify: stop_only (line 212-230) Current behavior:
.json(&serde_json::json!({
    "tap": vm.tap,
    "sock": vm.api_sock,
    "fc_unit": vm.fc_unit,
    "storage_path": st.storage.vm_dir(vm.id).display().to_string(), // ❌ DELETES STORAGE
}))
New behavior:
.json(&serde_json::json!({
    "tap": vm.tap,
    "sock": vm.api_sock,
    "fc_unit": vm.fc_unit,
    // DO NOT pass storage_path - keep drive files for restart
}))
Why: Stopped VMs should preserve their drives for restart.
5. Backend: VM Delete - Clean Up Storage
File: apps/manager/src/features/vms/service.rs Function to modify: stop_and_delete (line 232-238) Current behavior:
pub async fn stop_and_delete(st: &AppState, id: Uuid) -> Result<()> {
    stop_only(st, id).await?; // Stops but doesn't delete storage anymore
    super::repo::delete_row(&st.db, id).await?;
    Ok(())
}
New behavior:
pub async fn stop_and_delete(st: &AppState, id: Uuid) -> Result<()> {
    let vm = super::repo::get(&st.db, id).await?;
    
    // Stop the VM (doesn't delete storage)
    if let Err(err) = stop_only(st, id).await {
        tracing::warn!(vm_id = %id, error = ?err, "failed to stop vm before deletion");
    }
    
    // NOW delete storage manually
    let storage_dir = st.storage.vm_dir(id);
    if let Err(e) = tokio::fs::remove_dir_all(&storage_dir).await {
        tracing::warn!(error = ?e, path = ?storage_dir, "failed to cleanup storage directory");
    }
    
    // Delete from database (CASCADE will delete vm_drive rows)
    super::repo::delete_row(&st.db, id).await?;
    Ok(())
}
6. Backend: VM Restart - Restore All Drives
File: apps/manager/src/features/vms/service.rs Function to modify: restart_vm (line 190-210) Current behavior:
Spawns Firecracker
Calls configure_vm (only attaches rootfs)
Starts VM
New behavior:
✅ Query all drives from vm_drive table
✅ Verify/recreate drive files if missing (using size_bytes)
✅ Pass drives to configure_vm
✅ configure_vm attaches ALL drives to Firecracker
Pseudocode:
pub async fn restart_vm(st: &AppState, vm: &super::repo::VmRow) -> Result<()> {
    let host = st.hosts.get(vm.host_id).await?;
    let paths = VmPaths::from_row(vm);
    ensure_allowed_path(st, &vm.kernel_path)?;
    ensure_allowed_path(st, &vm.rootfs_path)?;
    
    let spec = ResolvedVmSpec {
        name: vm.name.clone(),
        vcpu: vm.vcpu.try_into()?,
        mem_mib: vm.mem_mib.try_into()?,
        kernel_path: vm.kernel_path.clone(),
        rootfs_path: vm.rootfs_path.clone(),
    };

    let network = select_network(&host.capabilities_json)?;
    
    // NEW: Load all drives from database
    let drives = super::repo::list_drives(&st.db, vm.id).await?;
    
    // NEW: Verify/recreate drive files
    for drive in &drives {
        if !tokio::fs::try_exists(&drive.path_on_host).await? {
            if let Some(size) = drive.size_bytes {
                // Recreate auto-provisioned drive
                st.storage.create_sparse_file(&drive.path_on_host, size).await?;
            } else {
                bail!("Drive file missing and no size_bytes to recreate: {}", drive.path_on_host);
            }
        }
    }
    
    create_tap(&host.addr, vm.id, &network.bridge).await?;
    spawn_firecracker(&host.addr, vm.id, &paths).await?;
    
    // NEW: Pass drives to configure_vm
    configure_vm(&host.addr, vm.id, &spec, &paths, &drives).await?;
    
    start_vm(&host.addr, vm.id, &paths).await?;
    super::repo::update_state(&st.db, vm.id, "running").await?;
    Ok(())
}
7. Backend: Configure VM - Apply All Drives
File: apps/manager/src/features/vms/service.rs Function to modify: configure_vm (line 1312-1435) Current behavior:
Configures machine-config, boot-source, rootfs, network, logger, metrics
Only attaches rootfs drive
New behavior:
Same as above, but also attach all additional drives from database
Pseudocode addition (after rootfs drive config):
// Attach additional drives from database
for drive in additional_drives {
    if drive.is_root_device {
        continue; // Already attached as rootfs
    }
    
    info!(vm_id=%id, drive_id=%drive.drive_id, path=%drive.path_on_host, "attaching additional drive");
    
    let mut drive_config = json!({
        "drive_id": drive.drive_id,
        "path_on_host": drive.path_on_host,
        "is_root_device": drive.is_root_device,
        "is_read_only": drive.is_read_only,
    });
    
    if let Some(cache) = &drive.cache_type {
        drive_config["cache_type"] = json!(cache);
    }
    if let Some(io) = &drive.io_engine {
        drive_config["io_engine"] = json!(io);
    }
    if let Some(rl) = &drive.rate_limiter {
        drive_config["rate_limiter"] = rl.clone();
    }
    
    http.put(format!("{base}/drives/{}{qs}", drive.drive_id))
        .json(&drive_config)
        .send()
        .await
        .context("drive attach request failed")?
        .error_for_status()
        .context("drive attach returned error")?;
    
    info!(vm_id=%id, drive_id=%drive.drive_id, "drive attached");
}
8. Backend: Repo Methods
File: apps/manager/src/features/vms/repo.rs New functions needed:
// Insert drive into database
pub async fn insert_drive(db: &PgPool, row: DriveRow) -> Result<DriveRow> { ... }

// List all drives for a VM
pub async fn list_drives(db: &PgPool, vm_id: Uuid) -> Result<Vec<DriveRow>> { ... }

// Get single drive
pub async fn get_drive(db: &PgPool, vm_id: Uuid, drive_id: &str) -> Result<DriveRow> { ... }

// Delete drive
pub async fn delete_drive(db: &PgPool, vm_id: Uuid, drive_id: &str) -> Result<()> { ... }

// Update drive
pub async fn update_drive(db: &PgPool, vm_id: Uuid, drive_id: &str, updates: UpdateDriveReq) -> Result<DriveRow> { ... }
9. Frontend: Update Drive Management UX
File: apps/frontend/components/drive-list.tsx Changes:
Remove "VM must be stopped" warning
Update message to: "Drives are stored in the database and will be attached when the VM starts. Changes take effect on next VM start."
File: apps/frontend/components/drive-editor-dialog.tsx Changes:
Keep current auto-provision vs manual path UI
Add info message: "This drive will be created immediately but attached to Firecracker on next VM start."
10. Testing Checklist
Test 1: Create VM, Add Drive, Restart
Create VM (auto-starts)
Add drive via API (VM is running)
Stop VM
Verify drive file exists in /srv/fc/vms/{id}/storage/
Start VM
Verify drive is attached in Firecracker (check VM detail page)
Test 2: Create VM, Stop, Add Drive, Start
Create VM (auto-starts)
Stop VM
Add drive via API
Start VM
Verify drive is attached
Test 3: Delete VM Cleans Up Storage
Create VM with extra drives
Delete VM
Verify /srv/fc/vms/{id}/ directory is deleted
Test 4: Auto-provisioned Drive Recreation
Create VM with auto-provisioned drive
Stop VM
Manually delete drive file from /srv/fc/vms/{id}/storage/
Start VM
Verify drive file is recreated with correct size
Summary of Changes
Component	File	Change
Database	0009_vm_drive_size.sql	Add size_bytes column
Create Drive	vms/service.rs	Don't call Firecracker, just save to DB
Delete Drive	vms/service.rs	Don't call Firecracker, just delete from DB
Stop VM	vms/service.rs	Don't pass storage_path to agent
Delete VM	vms/service.rs	Manually delete storage directory
Restart VM	vms/service.rs	Load drives from DB, recreate files, pass to configure
Configure VM	vms/service.rs	Attach all drives from database to Firecracker
Repo	vms/repo.rs	Add CRUD methods for vm_drive table
Frontend	drive-list.tsx, drive-editor-dialog.tsx	Update UX messaging
Update Todos

Analyze current drive lifecycle and document findings

Create implementation plan for proper drive persistence
The plan is complete! The key insight is that drives should be database-backed state, not live Firecracker operations. This allows:
✅ Adding/removing drives regardless of VM state
✅ Drives persist across stop/start cycles
✅ Auto-provisioned drives can be recreated using stored size_bytes
✅ Clean separation between VM lifecycle and drive management