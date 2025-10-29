/// Golden snapshot management for ultra-fast function provisioning
///
/// This module creates and manages "golden snapshots" - pre-booted VMs with
/// runtime servers already running. Restoring from a snapshot (2-3s) is 10x
/// faster than cold booting (20-30s).
use anyhow::{Context, Result};
use std::path::Path;
use uuid::Uuid;

use crate::AppState;

/// Path where golden snapshots are stored
const SNAPSHOT_DIR: &str = "/srv/snapshots";

/// Inject a script into the golden rootfs that restarts networking on boot
/// This ensures restored VMs request a fresh DHCP lease instead of using stale IPs
async fn inject_network_restart_script(rootfs_path: &str) -> Result<()> {
    eprintln!("[Snapshot] Mounting rootfs to inject network restart script...");

    // Mount the rootfs
    let mount_point = format!("/tmp/golden-snapshot-{}", Uuid::new_v4());
    tokio::fs::create_dir_all(&mount_point).await?;

    // Mount with sudo
    let mount_status = tokio::process::Command::new("sudo")
        .args(["-n", "mount", "-o", "loop", rootfs_path, &mount_point])
        .status()
        .await?;

    if !mount_status.success() {
        anyhow::bail!("Failed to mount rootfs");
    }

    // Create a local.d script that runs on boot to restart networking
    // This runs after all services are started, ensuring we get a fresh DHCP lease
    let script_path = format!("{}/etc/local.d/99-refresh-network.start", mount_point);
    let script_content = r#"#!/bin/sh
# Auto-generated script to refresh network on snapshot restore
# This ensures each restored VM gets a unique IP via DHCP

# Kill any existing DHCP client
pkill udhcpc 2>/dev/null || true

# Restart networking to get fresh DHCP lease
rc-service networking restart
"#;

    tokio::fs::write(&script_path, script_content).await?;

    // Make it executable
    tokio::process::Command::new("chmod")
        .args(["+x", &script_path])
        .output()
        .await?;

    // Enable local service if not already enabled
    let _ = tokio::process::Command::new("sudo")
        .args(["chroot", &mount_point, "rc-update", "add", "local", "default"])
        .output()
        .await;

    eprintln!("[Snapshot] ✅ Network restart script injected");

    // Unmount
    let _ = tokio::process::Command::new("sudo")
        .args(["-n", "umount", &mount_point])
        .status()
        .await?;

    tokio::fs::remove_dir(&mount_point).await?;

    Ok(())
}

/// Golden snapshot metadata
#[derive(Debug, Clone)]
pub struct GoldenSnapshot {
    pub runtime: String,
    pub vm_snapshot_path: String,
    pub mem_snapshot_path: String,
    pub rootfs_path: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Check if a golden snapshot exists for a runtime
pub async fn snapshot_exists(runtime: &str) -> bool {
    let vm_path = format!("{}/{}-golden-vm.snap", SNAPSHOT_DIR, runtime);
    let mem_path = format!("{}/{}-golden-mem.snap", SNAPSHOT_DIR, runtime);

    Path::new(&vm_path).exists() && Path::new(&mem_path).exists()
}

/// Get golden snapshot paths for a runtime
pub fn get_snapshot_paths(runtime: &str) -> (String, String, String) {
    let vm_path = format!("{}/{}-golden-vm.snap", SNAPSHOT_DIR, runtime);
    let mem_path = format!("{}/{}-golden-mem.snap", SNAPSHOT_DIR, runtime);
    let rootfs_path = format!("/srv/images/{}-runtime.ext4", runtime);

    (vm_path, mem_path, rootfs_path)
}

/// Create a golden snapshot for a runtime
///
/// This boots a VM with the runtime, waits for it to be ready, then takes a
/// Firecracker snapshot. Future VMs can restore from this snapshot instantly.
pub async fn create_golden_snapshot(st: &AppState, runtime: &str) -> Result<()> {
    eprintln!("========================================");
    eprintln!("Creating Golden Snapshot for {}", runtime);
    eprintln!("========================================");

    // Ensure snapshot directory exists
    tokio::fs::create_dir_all(SNAPSHOT_DIR)
        .await
        .context("Failed to create snapshot directory")?;

    let snapshot_id = Uuid::new_v4();
    let vm_name = format!("{}-golden-template", runtime);

    eprintln!("[Snapshot] Step 1: Creating template VM...");

    // Create a temporary VM with the runtime
    let vm_id = Uuid::new_v4();
    let (kernel_path, base_rootfs_path) = super::vm::get_runtime_image_paths(runtime)?;

    // IMPORTANT: Create the golden snapshot rootfs with its FINAL name from the start
    // Firecracker bakes the drive path into the snapshot during creation.
    // If we rename after snapshot creation, restore will fail because Firecracker
    // expects the drive at the original path.
    // Use /srv/images/functions/ which should be writable by the manager process
    let temp_rootfs_path = format!("/srv/images/functions/{}-golden.ext4", runtime);

    eprintln!("[Snapshot] Copying base runtime image for golden template...");
    eprintln!("[Snapshot]   From: {}", base_rootfs_path);
    eprintln!("[Snapshot]   To: {}", temp_rootfs_path);

    // Ensure the functions directory exists
    tokio::fs::create_dir_all("/srv/images/functions")
        .await
        .context("Failed to create /srv/images/functions directory")?;

    crate::features::vms::fast_provisioning::reflink_copy(&base_rootfs_path, &temp_rootfs_path)
        .await
        .context("Failed to copy base runtime for golden snapshot")?;

    eprintln!("[Snapshot] ✅ Runtime image copied successfully");

    // Note: Network restart script will be injected during guest agent installation
    // (the rootfs is already mounted during that phase)

    // Fix any filesystem corruption in the copied image
    // Use -y to automatically fix all errors without prompting
    eprintln!("[Snapshot] Running fsck to fix filesystem corruption...");
    let fsck_output = tokio::process::Command::new("sudo")
        .args(["-n", "fsck.ext4", "-y", "-f", &temp_rootfs_path])
        .output()
        .await
        .context("Failed to run fsck")?;

    let fsck_stdout = String::from_utf8_lossy(&fsck_output.stdout);
    let fsck_stderr = String::from_utf8_lossy(&fsck_output.stderr);

    if !fsck_stdout.is_empty() {
        eprintln!("[Snapshot] fsck output:\n{}", fsck_stdout);
    }
    if !fsck_stderr.is_empty() {
        eprintln!("[Snapshot] fsck errors:\n{}", fsck_stderr);
    }

    // fsck exit codes: 0=no errors, 1=errors corrected, 2=system should reboot, 4=errors left uncorrected
    match fsck_output.status.code() {
        Some(0) => eprintln!("[Snapshot] ✅ Filesystem is clean"),
        Some(1) => eprintln!("[Snapshot] ✅ Filesystem errors corrected"),
        Some(2) => eprintln!("[Snapshot] ⚠️  Filesystem errors corrected, system should reboot (continuing anyway)"),
        Some(4) => {
            eprintln!("[Snapshot] ❌ Filesystem has uncorrectable errors!");
            anyhow::bail!("Base runtime image has serious filesystem corruption. Please rebuild the runtime image.");
        }
        Some(code) => eprintln!("[Snapshot] ⚠️  fsck returned code {}, continuing...", code),
        None => eprintln!("[Snapshot] ⚠️  fsck terminated by signal, continuing..."),
    }

    let vm_req = nexus_types::CreateVmReq {
        name: vm_name.clone(),
        vcpu: 1,
        mem_mib: 512,
        kernel_image_id: None,
        rootfs_image_id: None,
        kernel_path: Some(kernel_path),
        rootfs_path: Some(temp_rootfs_path.clone()),  // Use the copy
        source_snapshot_id: None,
        username: Some("root".to_string()),
        password: Some("golden".to_string()),
    };

    // Create and start the VM
    crate::features::vms::service::create_and_start(st, vm_id, vm_req, None).await?;

    eprintln!("[Snapshot] Step 2: Waiting for VM to boot and runtime to be ready...");

    // Wait a bit for the VM to boot
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;

    // Try to get guest IP from database first (normal guest agent reporting)
    // Golden snapshot creation needs longer timeout because:
    // - Guest agent detects it's a template (no VM ID in manager) and restarts networking
    // - Runtime server needs time to start on first boot
    // - Total can take 40-60 seconds on first boot
    let mut guest_ip: Option<String> = None;
    for attempt in 1..=60 {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        if let Ok(vm) = crate::features::vms::repo::get(&st.db, vm_id).await {
            if let Some(ip) = vm.guest_ip {
                eprintln!("[Snapshot] Got guest IP from database: {}", ip);
                guest_ip = Some(ip);
                break;
            }
        }

        if attempt % 10 == 0 {
            eprintln!("[Snapshot] Still waiting for guest IP from database... ({}s)", attempt);
        }
    }

    // If database reporting failed, try neighbor table detection
    if guest_ip.is_none() {
        eprintln!("[Snapshot] Database reporting failed, trying neighbor table detection...");
        
        if let Ok(vm) = crate::features::vms::repo::get(&st.db, vm_id).await {
            // Use the existing detect_ip_from_neighbor_table function
            use crate::features::vms::fast_provisioning::detect_ip_from_neighbor_table;
            
            match detect_ip_from_neighbor_table("fcbr0", &vm.tap, 30).await {
                Ok(Some(ip)) => {
                    eprintln!("[Snapshot] Detected IP from neighbor table: {}", ip);
                    guest_ip = Some(ip.clone());
                    
                    // Update the database with the detected IP
                    let _ = sqlx::query!(
                        "UPDATE vm SET guest_ip = $1 WHERE id = $2",
                        ip,
                        vm_id
                    )
                    .execute(&st.db)
                    .await;
                }
                Ok(None) => {
                    eprintln!("[Snapshot] Neighbor table detection found no IP");
                }
                Err(e) => {
                    eprintln!("[Snapshot] Neighbor table detection failed: {}", e);
                }
            }
        }
    }

    let guest_ip = guest_ip.context("Timeout waiting for guest IP")?;
    eprintln!("[Snapshot] Got guest IP: {}", guest_ip);

    // Wait for runtime to be ready (longer timeout for golden snapshot creation)
    eprintln!("[Snapshot] Step 3: Waiting for runtime server to be ready...");

    let url = format!("http://{}:3000/health", guest_ip);
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()?;

    let mut ready = false;
    for attempt in 1..=120 {
        match client.get(&url).send().await {
            Ok(resp) if resp.status().is_success() => {
                eprintln!("[Snapshot] Runtime ready after {} seconds!", attempt);
                ready = true;
                break;
            }
            _ => {
                if attempt % 10 == 0 {
                    eprintln!("[Snapshot] Still waiting for runtime... ({}s)", attempt);
                }
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        }
    }

    if !ready {
        anyhow::bail!("Runtime server did not become ready");
    }

    // Give runtime a bit more time to stabilize
    eprintln!("[Snapshot] Runtime ready! Waiting 5 seconds for stabilization...");
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;

    // Clean network state BEFORE pausing to ensure restored VMs get fresh IPs
    // Try guest agent endpoint first, fall back to direct rootfs manipulation
    eprintln!("[Snapshot] Step 3.5: Cleaning network state before snapshot...");
    let clean_url = format!("http://{}:9000/clean-network", guest_ip);
    let mut network_cleaned = false;

    match client.post(&clean_url).timeout(std::time::Duration::from_secs(3)).send().await {
        Ok(resp) if resp.status().is_success() => {
            eprintln!("[Snapshot] ✅ Network state cleaned via guest agent");
            if let Ok(body) = resp.json::<serde_json::Value>().await {
                eprintln!("[Snapshot]    Operations: {:?}", body.get("operations"));
            }
            network_cleaned = true;
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }
        _ => {
            eprintln!("[Snapshot] ⚠️  Guest agent endpoint not available, will clean network via rootfs after snapshot");
        }
    }

    eprintln!("[Snapshot] Step 4: Pausing VM before snapshot...");

    // Get VM info
    let vm = crate::features::vms::repo::get(&st.db, vm_id).await?;

    // Pause the VM via Firecracker API
    let pause_url = format!(
        "{}/agent/v1/vms/{}/proxy/vm?sock={}",
        vm.host_addr,
        vm_id,
        urlencoding::encode(&vm.api_sock)
    );

    let pause_req = serde_json::json!({
        "state": "Paused"
    });

    eprintln!("[Snapshot] Sending pause request to: {}", pause_url);

    let pause_resp = client
        .patch(&pause_url)
        .json(&pause_req)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .with_context(|| format!("Failed to call pause API at {}", pause_url))?;

    if !pause_resp.status().is_success() {
        anyhow::bail!(
            "Failed to pause VM: HTTP {} - {}",
            pause_resp.status(),
            pause_resp.text().await.unwrap_or_default()
        );
    }

    eprintln!("[Snapshot] ✅ VM paused successfully");

    eprintln!("[Snapshot] Step 5: Creating snapshot files...");

    let snap_vm_path = format!("{}/{}-golden-vm.snap", SNAPSHOT_DIR, runtime);
    let snap_mem_path = format!("{}/{}-golden-mem.snap", SNAPSHOT_DIR, runtime);

    // Call Firecracker snapshot API via agent
    // Use "Full" snapshot type since VM is paused
    let snapshot_req = serde_json::json!({
        "snapshot_path": snap_vm_path,
        "mem_file_path": snap_mem_path,
        "snapshot_type": "Full",
    });

    let snapshot_url = format!(
        "{}/agent/v1/vms/{}/proxy/snapshot/create?sock={}",
        vm.host_addr,
        vm_id,
        urlencoding::encode(&vm.api_sock)
    );

    eprintln!("[Snapshot] Calling snapshot API at: {}", snapshot_url);
    eprintln!("[Snapshot] Request body: {}", serde_json::to_string_pretty(&snapshot_req).unwrap());

    let snapshot_resp = client
        .put(&snapshot_url)
        .json(&snapshot_req)
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await
        .with_context(|| format!("Failed to call snapshot API at {}", snapshot_url))?;

    if !snapshot_resp.status().is_success() {
        anyhow::bail!(
            "Snapshot creation failed: HTTP {} - {}",
            snapshot_resp.status(),
            snapshot_resp.text().await.unwrap_or_default()
        );
    }

    eprintln!("[Snapshot] ✅ Snapshot created successfully!");
    eprintln!("[Snapshot]    VM state: {}", snap_vm_path);
    eprintln!("[Snapshot]    Memory: {}", snap_mem_path);

    // Keep the VM paused for a bit to ensure snapshot is complete
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    // Stop the template VM
    eprintln!("[Snapshot] Step 6: Stopping template VM...");
    let _ = crate::features::vms::service::stop_and_delete(st, vm_id).await;

    // If network wasn't cleaned via guest agent, clean it now by manipulating the rootfs
    if !network_cleaned {
        eprintln!("[Snapshot] Step 7: Cleaning network state from rootfs...");
        if let Err(e) = clean_network_from_rootfs(&temp_rootfs_path).await {
            eprintln!("[Snapshot] ⚠️  Failed to clean network from rootfs: {}", e);
            eprintln!("[Snapshot]    Restored VMs may reuse the same IP");
        } else {
            eprintln!("[Snapshot] ✅ Network state cleaned from rootfs");
        }
    }

    // No need to rename - we used the final golden path from the beginning
    // The snapshot was created with the correct drive path baked in
    let golden_rootfs_path = temp_rootfs_path.clone();

    eprintln!("========================================");
    eprintln!("✅ Golden snapshot created for {}", runtime);
    eprintln!("========================================");
    eprintln!();
    eprintln!("Snapshot files:");
    eprintln!("  VM state: {}", snap_vm_path);
    eprintln!("  Memory: {}", snap_mem_path);
    eprintln!("  Rootfs: {}", golden_rootfs_path);
    eprintln!();
    eprintln!("Future {} functions will restore from this snapshot", runtime);
    eprintln!("Expected provisioning time: 5-15 seconds (down from 120+ seconds)");
    eprintln!();

    Ok(())
}

/// List all available golden snapshots
pub async fn list_golden_snapshots() -> Result<Vec<String>> {
    let mut runtimes = Vec::new();

    if !Path::new(SNAPSHOT_DIR).exists() {
        return Ok(runtimes);
    }

    let mut entries = tokio::fs::read_dir(SNAPSHOT_DIR).await?;

    while let Some(entry) = entries.next_entry().await? {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        // Look for -golden-vm.snap files
        if name_str.ends_with("-golden-vm.snap") {
            let runtime = name_str
                .strip_suffix("-golden-vm.snap")
                .unwrap_or("")
                .to_string();

            // Verify memory snapshot also exists
            let mem_path = format!("{}/{}-golden-mem.snap", SNAPSHOT_DIR, runtime);
            if Path::new(&mem_path).exists() {
                runtimes.push(runtime);
            }
        }
    }

    Ok(runtimes)
}

/// Delete a golden snapshot
pub async fn delete_golden_snapshot(runtime: &str) -> Result<()> {
    let (vm_path, mem_path, _) = get_snapshot_paths(runtime);

    if Path::new(&vm_path).exists() {
        tokio::fs::remove_file(&vm_path).await?;
    }

    if Path::new(&mem_path).exists() {
        tokio::fs::remove_file(&mem_path).await?;
    }

    eprintln!("✅ Deleted golden snapshot for {}", runtime);

    Ok(())
}

/// Clean network state from rootfs by removing DHCP leases and network config
/// This ensures VMs restored from snapshot request fresh DHCP leases
async fn clean_network_from_rootfs(rootfs_path: &str) -> Result<()> {
    use tokio::process::Command;

    let mount_point = format!("/tmp/clean-network-{}", uuid::Uuid::new_v4());
    tokio::fs::create_dir_all(&mount_point).await?;

    // Mount the rootfs
    let mount_status = Command::new("sudo")
        .args(["-n", "mount", "-o", "loop", rootfs_path, &mount_point])
        .status()
        .await?;

    if !mount_status.success() {
        anyhow::bail!("Failed to mount rootfs for network cleanup");
    }

    // Clean DHCP lease files
    let lease_paths = vec![
        format!("{}/var/lib/dhcp", mount_point),
        format!("{}/var/lib/dhcpc", mount_point),
        format!("{}/var/run/udhcpc.eth0.pid", mount_point),
    ];

    for path in lease_paths {
        let _ = Command::new("sudo")
            .args(["rm", "-rf", &path])
            .status()
            .await;
    }

    // Remove any cached network state
    let _ = Command::new("sudo")
        .args(["rm", "-f", &format!("{}/etc/udhcpc/eth0.lease", mount_point)])
        .status()
        .await;

    // Unmount
    let unmount_status = Command::new("sudo")
        .args(["-n", "umount", &mount_point])
        .status()
        .await?;

    tokio::fs::remove_dir(&mount_point).await?;

    if !unmount_status.success() {
        anyhow::bail!("Failed to unmount rootfs after network cleanup");
    }

    Ok(())
}
