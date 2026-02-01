use std::path::{Path, PathBuf};

use axum::{extract::Path as AxumPath, http::StatusCode, routing::post, Extension, Json, Router};
use serde::{Deserialize, Serialize};
use tokio::fs;
use uuid::Uuid;

use crate::AppState;

pub fn router() -> Router {
    Router::new()
        .route("/:id/snapshots/prepare", post(prepare))
        .route("/:id/snapshots/restore", post(restore_from_snapshot))
}

#[derive(Deserialize)]
struct RestoreSnapshotRequest {
    snapshot_path: String,
    mem_path: String,
    enable_diff_snapshots: bool,
    vcpu: u8,
    mem_mib: u32,
    /// Optional: If provided, validate FC version matches
    fc_version: Option<String>,
    /// Network configuration
    tap_device: String,
    /// Optional: If not provided, Firecracker will auto-generate
    guest_mac: Option<String>,
}

#[derive(Serialize)]
struct RestoreSnapshotResponse {
    success: bool,
    message: String,
    /// The actual rootfs path that the VM is using (the embedded path from snapshot state)
    rootfs_path: String,
}

#[derive(Deserialize)]
struct PrepareSnapshotRequest {
    snapshot_id: Uuid,
    #[serde(default)]
    snapshot_type: Option<String>,
}

#[derive(Serialize)]
struct PrepareSnapshotResponse {
    snapshot_path: String,
    mem_path: Option<String>,
    diff_dir: Option<String>,
    snapshot_size_bytes: Option<u64>,
    mem_size_bytes: Option<u64>,
}

async fn prepare(
    Extension(st): Extension<AppState>,
    AxumPath(vm_id): AxumPath<Uuid>,
    Json(req): Json<PrepareSnapshotRequest>,
) -> Result<Json<PrepareSnapshotResponse>, (StatusCode, String)> {
    let run_dir = PathBuf::from(&st.run_dir);
    let base_dir = snapshot_base_dir(&run_dir, &vm_id, &req.snapshot_id);
    fs::create_dir_all(&base_dir)
        .await
        .map_err(internal_error)?;
    let base_dir = canonicalize_dir(&base_dir).await?;

    let snapshot_type = req.snapshot_type.as_deref().unwrap_or("Full").to_string();

    let snapshot_path = base_dir.join(match snapshot_type.as_str() {
        "Diff" => "diff.fc",
        _ => "snapshot.fc",
    });

    let mem_path = if snapshot_type == "Diff" {
        None
    } else {
        let mem_dir = base_dir.join("mem");
        fs::create_dir_all(&mem_dir).await.map_err(internal_error)?;
        let mem_dir = canonicalize_dir(&mem_dir).await?;
        Some(mem_dir.join("mem.fc"))
    };

    let diff_dir = if snapshot_type == "Diff" {
        let dir = base_dir.join("diff");
        fs::create_dir_all(&dir).await.map_err(internal_error)?;
        let dir = canonicalize_dir(&dir).await?;
        Some(dir)
    } else {
        None
    };

    let (_, snapshot_size_bytes) = file_status(&snapshot_path).await?;
    let mem_size_bytes = match &mem_path {
        Some(path) => file_status(path).await?.1,
        None => None,
    };

    Ok(Json(PrepareSnapshotResponse {
        snapshot_path: path_to_string(&snapshot_path)?,
        mem_path: mem_path.map(|p| path_to_string(&p)).transpose()?,
        diff_dir: diff_dir.map(|p| path_to_string(&p)).transpose()?,
        snapshot_size_bytes,
        mem_size_bytes,
    }))
}

fn snapshot_base_dir(run_dir: &Path, vm_id: &Uuid, snapshot_id: &Uuid) -> PathBuf {
    run_dir
        .join("vms")
        .join(vm_id.to_string())
        .join("snapshots")
        .join(snapshot_id.to_string())
}

async fn canonicalize_dir(path: &Path) -> Result<PathBuf, (StatusCode, String)> {
    fs::canonicalize(path).await.map_err(internal_error)
}

async fn file_status(path: &Path) -> Result<(bool, Option<u64>), (StatusCode, String)> {
    match fs::metadata(path).await {
        Ok(meta) => {
            if meta.is_file() {
                Ok((true, Some(meta.len())))
            } else {
                Ok((true, None))
            }
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok((false, None)),
        Err(err) => Err(internal_error(err)),
    }
}

fn path_to_string(path: &Path) -> Result<String, (StatusCode, String)> {
    path.to_str().map(|s| s.to_owned()).ok_or_else(|| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "path encoding error".into(),
        )
    })
}

fn internal_error<E: std::fmt::Display>(err: E) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}

async fn restore_from_snapshot(
    Extension(st): Extension<AppState>,
    AxumPath(vm_id): AxumPath<Uuid>,
    Json(req): Json<RestoreSnapshotRequest>,
) -> Result<Json<RestoreSnapshotResponse>, (StatusCode, String)> {
    tracing::info!(
        "Restore snapshot request received for VM {}: snapshot={}, mem={}, tap={}",
        vm_id,
        req.snapshot_path,
        req.mem_path,
        req.tap_device
    );

    // 1. Validate Firecracker version if provided
    if let Some(expected_version) = &req.fc_version {
        match detect_firecracker_version().await {
            Ok(actual_version) => {
                if &actual_version != expected_version {
                    return Err((
                        StatusCode::CONFLICT,
                        format!(
                            "Firecracker version mismatch: snapshot created with {}, but system has {}",
                            expected_version, actual_version
                        ),
                    ));
                }
            }
            Err(e) => {
                tracing::warn!("Failed to detect Firecracker version: {}", e);
                // Continue anyway - version detection is best-effort
            }
        }
    }

    // 2. Verify snapshot files exist
    if !PathBuf::from(&req.snapshot_path).exists() {
        return Err((
            StatusCode::NOT_FOUND,
            format!("Snapshot file not found: {}", req.snapshot_path),
        ));
    }
    if !PathBuf::from(&req.mem_path).exists() {
        return Err((
            StatusCode::NOT_FOUND,
            format!("Memory file not found: {}", req.mem_path),
        ));
    }

    // 3. Spawn Firecracker process
    let run_dir = PathBuf::from(&st.run_dir);
    let vm_dir = run_dir.join("vms").join(vm_id.to_string());
    fs::create_dir_all(&vm_dir)
        .await
        .map_err(internal_error)?;

    let sock_path = vm_dir.join("fc.sock");
    let log_path = vm_dir.join("fc.log");

    // Remove old socket if exists
    if sock_path.exists() {
        let _ = fs::remove_file(&sock_path).await;
    }

    // Spawn Firecracker via systemd
    let unit = format!("fc-{}.scope", vm_id);
    let sock_str = sock_path.to_str().ok_or_else(|| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Invalid socket path".to_string(),
        )
    })?;

    crate::core::systemd::spawn_fc_scope(&unit, sock_str)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to spawn Firecracker: {}", e),
            )
        })?;

    // Wait for socket to be ready
    tracing::info!("Waiting for Firecracker socket to be ready");
    let sock_ready = wait_for_socket(&sock_path).await;
    if !sock_ready {
        let err_msg = "Firecracker socket did not appear".to_string();
        tracing::error!("{}", err_msg);
        return Err((StatusCode::INTERNAL_SERVER_ERROR, err_msg));
    }
    tracing::info!("Firecracker socket is ready");

    // 4. Create TAP device on host
    // The snapshot contains network configuration, so we just need to ensure
    // the tap device exists on the host side
    tracing::info!("Creating TAP device: {}", req.tap_device);
    crate::core::net::create_tap(&req.tap_device, &st.bridge, None)
        .await
        .map_err(|e| {
            let err_msg = format!("Failed to create TAP device: {}", e);
            tracing::error!("{}", err_msg);
            (StatusCode::INTERNAL_SERVER_ERROR, err_msg)
        })?;
    tracing::info!("TAP device created successfully");

    // 5. Ensure rootfs exists at the path embedded in snapshot
    // The snapshot state file contains absolute paths to drives that must exist
    // We need to extract the rootfs path and ensure the file is available
    tracing::info!("Preparing rootfs for snapshot restore");
    let snapshot_dir = std::path::Path::new(&req.snapshot_path)
        .parent()
        .ok_or_else(|| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Invalid snapshot path".to_string(),
            )
        })?;
    let snapshot_rootfs = snapshot_dir.join("rootfs.ext4");

    if !snapshot_rootfs.exists() {
        let err_msg = format!("Snapshot rootfs not found at {}", snapshot_rootfs.display());
        tracing::error!("{}", err_msg);
        return Err((StatusCode::INTERNAL_SERVER_ERROR, err_msg));
    }

    // Use strings command to extract rootfs path from snapshot state file
    // This is more reliable than reading the binary file directly
    tracing::info!("Extracting embedded rootfs path from snapshot state");
    let strings_output = tokio::process::Command::new("strings")
        .arg(&req.snapshot_path)
        .output()
        .await
        .map_err(|e| {
            let err_msg = format!("Failed to run strings command: {}", e);
            tracing::error!("{}", err_msg);
            (StatusCode::INTERNAL_SERVER_ERROR, err_msg)
        })?;

    if !strings_output.status.success() {
        let err_msg = "strings command failed".to_string();
        tracing::error!("{}", err_msg);
        return Err((StatusCode::INTERNAL_SERVER_ERROR, err_msg));
    }

    let strings_text = String::from_utf8_lossy(&strings_output.stdout);
    let embedded_rootfs_path = strings_text
        .lines()
        .find(|line| line.contains("/srv/fc/vms/") && line.ends_with(".ext4"))
        .ok_or_else(|| {
            let err_msg = "Could not find rootfs path in snapshot state".to_string();
            tracing::error!("{}", err_msg);
            (StatusCode::INTERNAL_SERVER_ERROR, err_msg)
        })?;

    tracing::info!("Found embedded rootfs path in snapshot: {}", embedded_rootfs_path);

    // Create the directory structure for the embedded path
    if let Some(parent) = std::path::Path::new(embedded_rootfs_path).parent() {
        tracing::info!("Creating parent directory: {}", parent.display());
        fs::create_dir_all(parent)
            .await
            .map_err(|e| {
                let err_msg = format!("Failed to create rootfs parent directory: {}", e);
                tracing::error!("{}", err_msg);
                (StatusCode::INTERNAL_SERVER_ERROR, err_msg)
            })?;
    }

    // Copy snapshot rootfs to the embedded path if needed
    let embedded_path = PathBuf::from(embedded_rootfs_path);
    let need_copy = if embedded_path.exists() {
        // Check if sizes match using async metadata
        tracing::info!("Embedded path already exists, checking if update needed");
        match (
            fs::metadata(&snapshot_rootfs).await,
            fs::metadata(&embedded_path).await,
        ) {
            (Ok(src_meta), Ok(dst_meta)) => {
                let size_mismatch = src_meta.len() != dst_meta.len();
                tracing::info!("Size check: src={} dst={} need_copy={}", src_meta.len(), dst_meta.len(), size_mismatch);
                size_mismatch
            }
            _ => {
                tracing::info!("Could not get metadata, will copy");
                true
            }
        }
    } else {
        tracing::info!("Embedded path does not exist, will copy");
        true
    };

    if need_copy {
        tracing::info!("Copying rootfs from {} to {}", snapshot_rootfs.display(), embedded_path.display());
        fs::copy(&snapshot_rootfs, &embedded_path)
            .await
            .map_err(|e| {
                let err_msg = format!("Failed to copy rootfs: {}", e);
                tracing::error!("{}", err_msg);
                (StatusCode::INTERNAL_SERVER_ERROR, err_msg)
            })?;
        tracing::info!("Rootfs copied successfully");
    } else {
        tracing::info!("Rootfs already exists at correct path with matching size, skipping copy");
    }

    // Update guest agent config in the rootfs BEFORE loading snapshot
    // This ensures the guest agent starts with the correct VM ID
    tracing::info!("Updating guest agent config in rootfs before loading snapshot");
    if let Err(e) = update_guest_agent_config_in_rootfs(&embedded_path, vm_id).await {
        tracing::warn!("Failed to update guest agent config: {}. Guest IP reporting may not work.", e);
    } else {
        tracing::info!("Guest agent config updated successfully with VM ID {}", vm_id);
    }

    // 6. Load snapshot via Firecracker API
    // IMPORTANT: Do NOT configure network before loading snapshot!
    // Firecracker does not allow configuring boot-specific resources before snapshot load.
    // The snapshot already contains the network configuration from when it was created.
    tracing::info!("Loading snapshot from {}", req.snapshot_path);
    let load_payload = serde_json::json!({
        "snapshot_path": req.snapshot_path,
        "mem_file_path": req.mem_path,
        "enable_diff_snapshots": req.enable_diff_snapshots,
    });

    let load_result = crate::core::uds_proxy::forward(
        sock_str,
        "/snapshot/load",
        axum::http::Method::PUT,
        axum::http::HeaderMap::new(),
        serde_json::to_vec(&load_payload)
            .map_err(internal_error)?
            .into(),
    )
    .await;

    if let Err(e) = load_result {
        let err_msg = format!("Failed to load snapshot: {:?}", e);
        tracing::error!("{}", err_msg);
        // Clean up on failure
        let _ = crate::core::systemd::stop_unit(&unit).await;
        let _ = crate::core::net::delete_tap(&req.tap_device).await;
        return Err((StatusCode::INTERNAL_SERVER_ERROR, err_msg));
    }
    tracing::info!("Snapshot loaded successfully");

    // 7. VM is now running after loading the snapshot
    // Note: When loading a snapshot, Firecracker automatically resumes the VM
    // We don't need to (and can't) send an InstanceStart action
    tracing::info!("VM automatically resumed from snapshot");

    // 8. Network and guest agent are handled automatically:
    // The guest agent (running from snapshot memory) self-heals by:
    //   - Re-reading /etc/guest-agent.conf to pick up the new VM ID
    //   - Bringing up eth0 and running DHCP if the network is down
    //   - Reporting the new IP to the manager with the correct VM ID
    // No screen console commands needed (serial console is read-only after snapshot restore).
    tracing::info!("Guest agent in VM will self-heal network and report IP automatically");

    tracing::info!(
        "VM {} restored from snapshot {} successfully",
        vm_id,
        req.snapshot_path
    );

    Ok(Json(RestoreSnapshotResponse {
        success: true,
        message: format!("VM restored from snapshot successfully"),
        rootfs_path: embedded_path.to_string_lossy().to_string(),
    }))
}

/// Detect Firecracker version from binary
async fn detect_firecracker_version() -> Result<String, String> {
    use tokio::process::Command;

    let output = Command::new("firecracker")
        .arg("--version")
        .output()
        .await
        .map_err(|e| format!("Failed to execute firecracker: {}", e))?;

    if !output.status.success() {
        return Err("firecracker --version failed".to_string());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Output format: "Firecracker v1.9.0" or similar
    let version = stdout
        .split_whitespace()
        .nth(1)
        .ok_or_else(|| "Could not parse version".to_string())?
        .to_string();

    Ok(version)
}

/// Wait for Firecracker socket to appear
async fn wait_for_socket(sock_path: &Path) -> bool {
    use tokio::net::UnixStream;
    use std::time::Duration;

    // Wait up to 20 seconds for socket to appear and be connectable
    for _ in 0..400 {
        if sock_path.exists() {
            // Try to connect to verify it's not stale
            if UnixStream::connect(sock_path).await.is_ok() {
                return true;
            }
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    false
}

/// Update guest agent config in rootfs with new VM ID
/// This must be called BEFORE loading the snapshot so the guest agent starts with correct ID
async fn update_guest_agent_config_in_rootfs(rootfs_path: &Path, vm_id: Uuid) -> Result<(), String> {
    use tokio::process::Command;

    tracing::info!("Mounting rootfs to update guest agent config");
    let mount_point = format!("/tmp/vm-{}-rootfs-update", vm_id);

    // Create mount point
    tokio::fs::create_dir_all(&mount_point)
        .await
        .map_err(|e| format!("Failed to create mount point: {}", e))?;

    // Mount the rootfs
    let mount_status = Command::new("sudo")
        .args(["mount", "-o", "loop", rootfs_path.to_str().unwrap(), &mount_point])
        .status()
        .await
        .map_err(|e| format!("Failed to mount rootfs: {}", e))?;

    if !mount_status.success() {
        return Err("Failed to mount rootfs".to_string());
    }

    // Update the config file
    let config_path = format!("{}/etc/guest-agent.conf", mount_point);
    let manager_url = std::env::var("MANAGER_BASE")
        .unwrap_or_else(|_| "http://192.168.18.1:18080".to_string());

    let config_content = format!(
        "VM_ID={}\nMANAGER_URL={}\n",
        vm_id,
        manager_url
    );

    let write_result = tokio::fs::write(&config_path, config_content).await;

    // Always unmount, even if write failed
    let unmount_status = Command::new("sudo")
        .args(["umount", &mount_point])
        .status()
        .await
        .map_err(|e| format!("Failed to unmount rootfs: {}", e))?;

    // Clean up mount point
    let _ = tokio::fs::remove_dir(&mount_point).await;

    if !unmount_status.success() {
        return Err("Failed to unmount rootfs".to_string());
    }

    write_result.map_err(|e| format!("Failed to write config: {}", e))?;

    tracing::info!("Successfully updated guest agent config with VM ID {}", vm_id);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_dir_includes_vm_and_snapshot() {
        let vm_id = Uuid::new_v4();
        let snapshot_id = Uuid::new_v4();
        let base = snapshot_base_dir(Path::new("/srv/fc"), &vm_id, &snapshot_id);
        assert!(base.ends_with(format!("{snapshot_id}")));
        assert!(base.starts_with(Path::new("/srv/fc/vms")));
    }

    #[tokio::test]
    async fn file_status_reports_sizes() {
        let tmp = tempfile::tempdir().unwrap();
        let file_path = tmp.path().join("file.bin");
        assert_eq!(file_status(&file_path).await.unwrap(), (false, None));

        tokio::fs::write(&file_path, &[1u8; 8]).await.unwrap();
        assert_eq!(file_status(&file_path).await.unwrap(), (true, Some(8)));
    }
}
