use crate::AppState;
use anyhow::{Context, Result};
use nexus_types::CreateVmReq;
use std::time::Instant;
use uuid::Uuid;

/// Create a dedicated MicroVM for running a Docker container with warm boot support
///
/// This function attempts to use runtime snapshots for fast boot (warm path).
/// If no snapshot is available or warm boot fails, it falls back to cold boot.
///
/// Warm path (~5-10s):
/// - Restore from runtime snapshot (Docker already running)
/// - Docker auto-starts via init system
/// - DHCP assigns new IP
/// - Container ready
///
/// Cold path (~60-120s):
/// - Boot fresh VM with container runtime
/// - Wait for Docker daemon to start
/// - Container ready
pub async fn create_container_vm(
    st: &AppState,
    container_id: Uuid,
    container_name: &str,
    vcpu: u8,
    memory_mb: u32,
) -> Result<(Uuid, String)> {
    // Try warm boot first
    let start_time = Instant::now();

    match try_warm_boot_container_vm(st, container_id, container_name, vcpu, memory_mb).await {
        Ok((vm_id, boot_method, snapshot_id)) => {
            let elapsed = start_time.elapsed();
            tracing::info!(
                "Container {} VM created via {} in {:.2}s (snapshot: {})",
                container_id,
                boot_method,
                elapsed.as_secs_f64(),
                snapshot_id.map(|id| id.to_string()).unwrap_or_else(|| "none".to_string())
            );
            return Ok((vm_id, boot_method));
        }
        Err((e, snapshot_id_opt)) => {
            tracing::warn!(
                "Warm boot failed for container {}, falling back to cold boot: {}",
                container_id,
                e
            );

            // Increment failure count if we attempted to use a snapshot
            if let Some(snapshot_id) = snapshot_id_opt {
                let snapshot_repo =
                    crate::features::runtime_snapshots::repo::RuntimeSnapshotRepository::new(
                        st.db.clone(),
                    );
                if let Err(e) = snapshot_repo.increment_failure(snapshot_id).await {
                    tracing::warn!("Failed to increment snapshot failure count: {}", e);
                }

                // Check if we should mark snapshot as unhealthy
                if let Ok(snapshot) = snapshot_repo.get(snapshot_id).await {
                    if snapshot.failure_count >= 2 {
                        // After 3 consecutive failures, mark as unhealthy
                        tracing::warn!(
                            "Snapshot {} has {} failures, marking as unhealthy",
                            snapshot_id,
                            snapshot.failure_count + 1
                        );
                        if let Err(e) = snapshot_repo.mark_unhealthy(snapshot_id).await {
                            tracing::error!("Failed to mark snapshot as unhealthy: {}", e);
                        }
                    }
                }
            }
        }
    }

    // Fallback to cold boot
    cold_boot_container_vm(st, container_id, container_name, vcpu, memory_mb).await
}

/// Cleanup a restored VM on the agent when database operations fail
async fn cleanup_restored_vm(agent_addr: &str, vm_id: Uuid) -> Result<()> {
    let client = reqwest::Client::new();
    let kill_url = format!("{}/agent/v1/vms/{}", agent_addr, vm_id);

    client
        .delete(&kill_url)
        .send()
        .await
        .context("Failed to send kill request to agent")?
        .error_for_status()
        .context("Agent kill request failed")?;

    tracing::info!("Cleaned up restored VM {} on agent {}", vm_id, agent_addr);
    Ok(())
}

/// Attempt to create a container VM using runtime snapshot (warm boot)
///
/// Returns (vm_id, boot_method, snapshot_id)
/// On error, returns (error, snapshot_id) where snapshot_id is Some if we found a snapshot
async fn try_warm_boot_container_vm(
    st: &AppState,
    container_id: Uuid,
    container_name: &str,
    vcpu: u8,
    memory_mb: u32,
) -> std::result::Result<(Uuid, String, Option<Uuid>), (anyhow::Error, Option<Uuid>)> {
    // Get container runtime image ID
    let (_, base_rootfs_path) = get_container_runtime_image_paths()
        .map_err(|e| (e, None))?;

    // Find the runtime image in the database
    let image_root = std::env::var("MANAGER_IMAGE_ROOT").unwrap_or_else(|_| "/srv/images".to_string());
    let image_repo = crate::features::images::repo::ImageRepository::new(st.db.clone(), image_root);

    // Find image by path (this is a workaround - ideally we'd track runtime image ID)
    let runtime_image_id = find_runtime_image_by_path(&image_repo, &base_rootfs_path)
        .await
        .context("Failed to find runtime image")
        .map_err(|e| (e, None))?;

    // Check if a runtime snapshot exists
    let snapshot_repo = crate::features::runtime_snapshots::repo::RuntimeSnapshotRepository::new(st.db.clone());
    let snapshot_service = crate::features::runtime_snapshots::service::RuntimeSnapshotService::new(snapshot_repo);

    let mut snapshot = snapshot_service
        .find_by_runtime_image(runtime_image_id)
        .await
        .map_err(|e| (anyhow::Error::from(e), None))?
        .ok_or_else(|| (anyhow::anyhow!("No runtime snapshot available"), None))?;

    let snapshot_id = snapshot.id;

    // If snapshot is creating, wait up to 60 seconds for it to become ready
    if snapshot.state == "creating" {
        tracing::info!(
            "Snapshot {} is being created, waiting up to 60s...",
            snapshot_id
        );

        let wait_start = Instant::now();
        let wait_timeout = std::time::Duration::from_secs(60);

        loop {
            if wait_start.elapsed() > wait_timeout {
                return Err((
                    anyhow::anyhow!(
                        "Timeout waiting for snapshot {} to be ready (still in creating state)",
                        snapshot_id
                    ),
                    Some(snapshot_id),
                ));
            }

            tokio::time::sleep(std::time::Duration::from_secs(2)).await;

            // Re-fetch snapshot to check current state
            snapshot = snapshot_service
                .get(snapshot_id)
                .await
                .context("Failed to fetch snapshot status")
                .map_err(|e| (e, Some(snapshot_id)))?;

            if snapshot.state == "ready" {
                tracing::info!(
                    "Snapshot {} became ready after {:.1}s",
                    snapshot_id,
                    wait_start.elapsed().as_secs_f64()
                );
                break;
            } else if snapshot.state == "unhealthy" {
                return Err((
                    anyhow::anyhow!("Snapshot {} is unhealthy", snapshot_id),
                    Some(snapshot_id),
                ));
            }
        }
    } else if snapshot.state != "ready" {
        return Err((
            anyhow::anyhow!("Runtime snapshot is not ready (state: {})", snapshot.state),
            Some(snapshot_id),
        ));
    }

    tracing::info!(
        "Using warm boot for container {} with snapshot {}",
        container_id,
        snapshot.id
    );

    // Get snapshot paths
    let snapshot_base = snapshot.snapshot_path.clone();
    let mem_path = format!("{}/snapshot.mem", snapshot_base);
    let state_path = format!("{}/snapshot.state", snapshot_base);
    let rootfs_path = format!("{}/rootfs.ext4", snapshot_base);

    // Verify snapshot files exist
    if !tokio::fs::metadata(&mem_path).await.is_ok()
        || !tokio::fs::metadata(&state_path).await.is_ok()
        || !tokio::fs::metadata(&rootfs_path).await.is_ok()
    {
        return Err((
            anyhow::anyhow!("Snapshot files missing at {}", snapshot_base),
            Some(snapshot_id),
        ));
    }

    // Select a healthy host
    let host = st
        .hosts
        .first_healthy()
        .await
        .context("No healthy hosts available")
        .map_err(|e| (e, Some(snapshot_id)))?;

    // Generate VM ID and TAP device name
    let vm_id = Uuid::new_v4();

    // Check if VM ID already exists (shouldn't happen with UUID but be safe)
    if crate::features::vms::repo::get(&st.db, vm_id).await.is_ok() {
        tracing::warn!("Generated VM ID {} already exists, regenerating", vm_id);
        return Err((
            anyhow::anyhow!("Generated VM ID collision (extremely rare)"),
            Some(snapshot_id),
        ));
    }

    // For runtime snapshot restores, we need to use the SAME tap device name that was
    // used during snapshot creation. The snapshot builder uses snapshot_id as the VM ID,
    // which generates tap name: tap-{snapshot-id[..8]}
    let tap_name = format!("tap-{}", &snapshot.id.to_string()[..8]);

    // Generate VM name
    let vm_name = format!(
        "container-{}-{}",
        container_name,
        &container_id.to_string()[..8]
    );

    // Create VM paths for logging and socket
    let run_dir = std::env::var("MANAGER_STORAGE_ROOT").unwrap_or_else(|_| "/srv/fc".to_string());
    let vm_dir = format!("{}/vms/{}", run_dir, vm_id);
    tokio::fs::create_dir_all(&vm_dir)
        .await
        .context("Failed to create VM directory")
        .map_err(|e| (e, Some(snapshot_id)))?;

    let api_sock = format!("{}/fc.sock", vm_dir);
    let log_path = format!("{}/fc.log", vm_dir);
    let fc_unit = format!("fc-{}.scope", vm_id);

    // Call agent to restore snapshot
    let client = reqwest::Client::new();
    let restore_url = format!("{}/agent/v1/vms/{}/snapshots/restore", host.addr, vm_id);

    let restore_req = serde_json::json!({
        "snapshot_path": state_path,
        "mem_path": mem_path,
        "enable_diff_snapshots": false,
        "vcpu": vcpu,
        "mem_mib": memory_mb,
        "fc_version": snapshot.fc_version,
        "tap_device": tap_name,
        "guest_mac": null, // Let Firecracker auto-generate
    });

    tracing::info!(
        "Calling agent to restore snapshot for VM {} on host {}",
        vm_id,
        host.addr
    );

    let restore_resp = client
        .post(&restore_url)
        .json(&restore_req)
        .send()
        .await
        .context("Failed to send restore request to agent")
        .map_err(|e| (e, Some(snapshot_id)))?
        .error_for_status()
        .context("Agent restore request failed")
        .map_err(|e| (anyhow::Error::from(e), Some(snapshot_id)))?;

    let restore_result: serde_json::Value = restore_resp
        .json()
        .await
        .context("Failed to parse restore response")
        .map_err(|e| (anyhow::Error::from(e), Some(snapshot_id)))?;

    if !restore_result
        .get("success")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        return Err((
            anyhow::anyhow!(
                "Agent restore failed: {}",
                restore_result
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown error")
            ),
            Some(snapshot_id),
        ));
    }

    tracing::info!("VM {} restored from snapshot successfully", vm_id);

    // Note: The agent updates the guest agent config BEFORE loading the snapshot
    // This ensures the guest agent starts with the correct VM ID
    // We don't need to update it here after the VM is already running
    tracing::info!("Guest agent config updated by agent before snapshot load");

    // Create VM record in database
    // Note: We use tags to track warm boot and runtime snapshot ID
    // source_snapshot_id is for VM snapshots (snapshots table), not runtime snapshots
    let tags = vec![
        "type:container".to_string(),
        "boot:warm".to_string(),
        format!("runtime_snapshot_id:{}", snapshot.id),
    ];
    let insert_result = crate::features::vms::repo::insert(
        &st.db,
        &crate::features::vms::repo::VmRow {
            id: vm_id,
            name: vm_name.clone(),
            state: "running".into(),
            host_id: host.id,
            template_id: None,
            host_addr: host.addr.clone(),
            api_sock: api_sock.clone(),
            tap: tap_name.clone(),
            log_path: log_path.clone(),
            http_port: 0,
            fc_unit: fc_unit.clone(),
            vcpu: vcpu as i32,
            mem_mib: memory_mb as i32,
            kernel_path: "".to_string(), // Not applicable for snapshot restore
            rootfs_path: rootfs_path.clone(),
            source_snapshot_id: None, // Runtime snapshots are different from VM snapshots
            guest_ip: None, // Will be set when guest agent reports
            tags,
            created_by_user_id: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        },
    )
    .await;

    if let Err(e) = insert_result {
        // Database insert failed, clean up the restored VM
        let err_msg = format!("Database error: {}", e);
        tracing::error!(
            "Failed to insert VM record for {}: {}. Cleaning up restored VM.",
            vm_id,
            err_msg
        );

        // Try to kill the restored VM on the agent
        if let Err(cleanup_err) = cleanup_restored_vm(&host.addr, vm_id).await {
            tracing::warn!(
                "Failed to cleanup restored VM {} after database insert failure: {}",
                vm_id,
                cleanup_err
            );
        }

        return Err((
            anyhow::Error::from(e).context("Failed to insert VM record"),
            Some(snapshot_id),
        ));
    }

    tracing::info!(
        "Container {} warm boot completed: VM {} created from snapshot {}",
        container_id,
        vm_id,
        snapshot_id
    );

    // Update snapshot success count
    let snapshot_repo =
        crate::features::runtime_snapshots::repo::RuntimeSnapshotRepository::new(st.db.clone());
    if let Err(e) = snapshot_repo.increment_success(snapshot_id).await {
        tracing::warn!("Failed to increment snapshot success count: {}", e);
    }

    Ok((vm_id, "warm".to_string(), Some(snapshot_id)))
}

/// Create a container VM using traditional cold boot
async fn cold_boot_container_vm(
    st: &AppState,
    container_id: Uuid,
    container_name: &str,
    vcpu: u8,
    memory_mb: u32,
) -> Result<(Uuid, String)> {
    use tokio::process::Command;

    // Get container runtime image paths
    let (kernel_path, base_rootfs_path) = get_container_runtime_image_paths()?;

    // Create a per-container copy of the runtime image
    // This is necessary because:
    // 1. Firecracker requires exclusive write access to rootfs
    // 2. Each container gets its own isolated filesystem
    // 3. Multiple VMs cannot share the same writable rootfs file
    let vm_id = Uuid::new_v4();
    let container_rootfs_path = format!("/srv/images/containers/{}.ext4", vm_id);

    eprintln!(
        "[Container {}] Creating VM {} with dedicated runtime image copy",
        container_id, vm_id
    );
    eprintln!(
        "[Container {}] Copying {} to {}",
        container_id, base_rootfs_path, container_rootfs_path
    );

    // Ensure directory exists
    tokio::fs::create_dir_all("/srv/images/containers")
        .await
        .context("Failed to create containers image directory")?;

    // Copy the base runtime image to a container-specific image
    let copy_status = Command::new("cp")
        .args([&base_rootfs_path, &container_rootfs_path])
        .status()
        .await
        .context("Failed to execute cp command")?;

    if !copy_status.success() {
        anyhow::bail!(
            "Failed to copy runtime image from {} to {}",
            base_rootfs_path,
            container_rootfs_path
        );
    }

    eprintln!(
        "[Container {}] Runtime image copied successfully",
        container_id
    );

    // Create VM request using container-specific rootfs copy
    let vm_name = format!(
        "container-{}-{}",
        container_name,
        &container_id.to_string()[..8]
    );
    let vm_req = CreateVmReq {
        name: vm_name,
        vcpu,
        mem_mib: memory_mb,
        kernel_image_id: None,
        rootfs_image_id: None,
        kernel_path: Some(kernel_path),
        rootfs_path: Some(container_rootfs_path),
        source_snapshot_id: None,
        username: Some("root".to_string()),
        password: Some("container".to_string()),
        tags: vec!["type:container".to_string()],
    };

    // Create and start VM
    crate::features::vms::service::create_and_start(st, vm_id, vm_req, None).await?;

    eprintln!(
        "[Container {}] VM {} created and starting",
        container_id, vm_id
    );

    Ok((vm_id, "cold".to_string()))
}

/// Helper to find runtime image by path
async fn find_runtime_image_by_path(
    image_repo: &crate::features::images::repo::ImageRepository,
    path: &str,
) -> Result<Uuid> {
    // Query database to find image with matching path
    let filter = nexus_types::ImageFilter {
        kind: None,
        project: None,
        name: None,
    };
    let images = image_repo.list(&filter).await?;

    for image in images {
        if image.host_path == path {
            return Ok(image.id);
        }
    }

    anyhow::bail!("No runtime image found with path: {}", path)
}

/// Get kernel and rootfs paths for container runtime
fn get_container_runtime_image_paths() -> Result<(String, String)> {
    // TODO: These paths should be configurable via environment variables
    // or stored in a database/config file
    //
    // The container runtime image should have:
    // - Alpine Linux or Ubuntu minimal base
    // - Docker daemon installed
    // - Docker configured to listen on TCP port 2375 (or configurable)
    // - Systemd/OpenRC service to auto-start Docker daemon on boot
    // - iptables, containerd, runc, and other container runtime dependencies
    //
    // Example Dockerfile to build the image:
    // ```dockerfile
    // FROM alpine:3.18
    // RUN apk add --no-cache docker openrc iptables ip6tables
    // RUN rc-update add docker default
    // RUN mkdir -p /etc/docker && echo '{"hosts": ["tcp://0.0.0.0:2375", "unix:///var/run/docker.sock"]}' > /etc/docker/daemon.json
    // ```
    //
    // Then convert to ext4 rootfs for Firecracker

    let kernel = std::env::var("CONTAINER_RUNTIME_KERNEL")
        .unwrap_or_else(|_| "/srv/images/vmlinux-5.10.fc.bin".to_string());

    let rootfs = std::env::var("CONTAINER_RUNTIME_ROOTFS")
        .unwrap_or_else(|_| "/srv/images/container-runtime.ext4".to_string());

    Ok((kernel, rootfs))
}

/// Wait for Docker daemon to be ready in the guest VM
///
/// Polls the Docker API inside the guest until it responds successfully
pub async fn wait_for_docker_ready(guest_ip: &str, timeout_secs: u64) -> Result<()> {
    use std::time::Duration;

    let docker_url = format!("http://{}:2375/_ping", guest_ip);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()?;

    let start = std::time::Instant::now();
    let timeout = Duration::from_secs(timeout_secs);

    loop {
        if start.elapsed() > timeout {
            anyhow::bail!("Timeout waiting for Docker daemon to be ready");
        }

        match client.get(&docker_url).send().await {
            Ok(resp) if resp.status().is_success() => {
                eprintln!("[Docker] Daemon ready at {}", guest_ip);
                return Ok(());
            }
            Ok(resp) => {
                eprintln!("[Docker] Ping returned status: {}", resp.status());
            }
            Err(e) => {
                eprintln!("[Docker] Ping failed: {}", e);
            }
        }

        tokio::time::sleep(Duration::from_secs(2)).await;
    }
}

/// Clean up container VM and associated resources
pub async fn cleanup_container_vm(st: &AppState, vm_id: Uuid) -> Result<()> {
    eprintln!("[Container VM] Cleaning up VM {}", vm_id);

    // Delete the VM (this will stop it if running)
    if let Err(e) = crate::features::vms::service::stop_and_delete(st, vm_id).await {
        eprintln!("[Container VM] Failed to delete VM {}: {}", vm_id, e);
        // Continue cleanup even if VM deletion fails
    }

    // Delete the container-specific rootfs image
    let container_rootfs_path = format!("/srv/images/containers/{}.ext4", vm_id);
    if tokio::fs::metadata(&container_rootfs_path).await.is_ok() {
        if let Err(e) = tokio::fs::remove_file(&container_rootfs_path).await {
            eprintln!(
                "[Container VM] Failed to delete rootfs {}: {}",
                container_rootfs_path, e
            );
        } else {
            eprintln!("[Container VM] Deleted rootfs {}", container_rootfs_path);
        }
    }

    Ok(())
}

/// Get Docker API base URL for a container's VM
#[allow(dead_code)]
pub fn get_docker_api_url(guest_ip: &str) -> String {
    format!("http://{}:2375", guest_ip)
}

/// Update guest agent config file in rootfs after snapshot restore
///
/// When a VM is restored from a runtime snapshot, it contains the guest agent config
/// from the temporary VM that was used to create the snapshot. This function updates
/// the `/etc/guest-agent.conf` file with the new VM's ID so the guest agent reports
/// to the correct VM.
async fn update_guest_agent_config(rootfs_path: &str, vm_id: Uuid, _st: &AppState) -> Result<()> {
    use tokio::process::Command;
    use tokio::fs;

    tracing::info!("Updating guest agent config in {} for VM {}", rootfs_path, vm_id);

    // Mount the rootfs
    let mount_point = format!("/tmp/vm-{}-rootfs", vm_id);
    fs::create_dir_all(&mount_point).await?;

    // Mount the rootfs image
    let mount_result = Command::new("sudo")
        .args(["mount", "-o", "loop", rootfs_path, &mount_point])
        .status()
        .await?;

    if !mount_result.success() {
        anyhow::bail!("Failed to mount rootfs at {}", mount_point);
    }

    // Update config file
    let result = async {
        // Get manager URL from environment or construct it
        let manager_url = std::env::var("MANAGER_BASE")
            .or_else(|_| std::env::var("MANAGER_URL"))
            .unwrap_or_else(|_| {
                let bind_addr = std::env::var("MANAGER_BIND")
                    .unwrap_or_else(|_| "127.0.0.1:18080".to_string());
                format!("http://{}", bind_addr)
            });

        let config_content = format!(
            r#"# Guest Agent Configuration
# Auto-generated during VM restore from snapshot
VM_ID={}
MANAGER_URL={}
"#,
            vm_id, manager_url
        );

        let config_temp = format!("/tmp/guest-agent-config-{}", vm_id);
        fs::write(&config_temp, config_content).await?;

        let config_dest = format!("{}/etc/guest-agent.conf", mount_point);
        Command::new("sudo")
            .args(["cp", &config_temp, &config_dest])
            .status()
            .await?;

        fs::remove_file(&config_temp).await?;
        tracing::info!("Updated guest agent config at {}", config_dest);

        Ok::<(), anyhow::Error>(())
    }
    .await;

    // Always unmount
    let unmount_result = Command::new("sudo")
        .args(["umount", &mount_point])
        .status()
        .await;

    if let Err(e) = unmount_result {
        tracing::error!("Failed to unmount {}: {}", mount_point, e);
    }

    let _ = fs::remove_dir(&mount_point).await;

    result
}

/*
 * NOTES ON BUILDING CONTAINER RUNTIME IMAGE
 *
 * To create the container-runtime.ext4 image, you need:
 *
 * 1. Start with a base Linux distribution (Alpine recommended for size)
 * 2. Install Docker and dependencies:
 *    - docker
 *    - containerd
 *    - runc
 *    - iptables
 *    - openrc or systemd
 *
 * 3. Configure Docker to listen on TCP:
 *    /etc/docker/daemon.json:
 *    {
 *      "hosts": ["tcp://0.0.0.0:2375", "unix:///var/run/docker.sock"],
 *      "storage-driver": "overlay2"
 *    }
 *
 * 4. Enable Docker to start on boot:
 *    Alpine: rc-update add docker default
 *    Ubuntu: systemctl enable docker
 *
 * 5. Convert to ext4 rootfs:
 *    dd if=/dev/zero of=container-runtime.ext4 bs=1M count=2048
 *    mkfs.ext4 container-runtime.ext4
 *    mount -o loop container-runtime.ext4 /mnt
 *    # Copy your configured system to /mnt
 *    umount /mnt
 *
 * 6. Place the image at /srv/images/container-runtime.ext4
 *
 * Example script: scripts/build-container-runtime-image.sh
 */
