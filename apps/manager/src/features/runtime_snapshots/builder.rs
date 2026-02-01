use crate::AppState;
use anyhow::{Context, Result};
use nexus_types::CreateVmReq;
use serde_json::json;
use std::time::Duration;
use uuid::Uuid;

/// Build a runtime snapshot for container warm boot
///
/// Process:
/// 1. Create temporary VM with container runtime image
/// 2. Wait for Docker daemon to be ready
/// 3. Flush network configuration (remove IP, reset interface)
/// 4. Stop guest agent to prevent premature reporting
/// 5. Take Firecracker snapshot (memory + disk)
/// 6. Store snapshot artifacts and metadata
/// 7. Cleanup temporary VM
pub struct RuntimeSnapshotBuilder {
    state: AppState,
}

impl RuntimeSnapshotBuilder {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }

    /// Build a runtime snapshot for the given container runtime image
    pub async fn build_snapshot(
        &self,
        snapshot_id: Uuid,
        runtime_image_id: Uuid,
        snapshot_path: &str,
    ) -> Result<SnapshotMetadata> {
        tracing::info!(
            "Building runtime snapshot {} for image {}",
            snapshot_id,
            runtime_image_id
        );

        // Create temporary VM
        let vm_id = self
            .create_temp_vm(snapshot_id, runtime_image_id)
            .await
            .context("Failed to create temporary VM")?;

        // Ensure cleanup on error
        let cleanup_result = self.build_snapshot_inner(vm_id, snapshot_path).await;

        // Always cleanup temp VM
        if let Err(e) = self.cleanup_temp_vm(vm_id).await {
            tracing::warn!("Failed to cleanup temp VM {}: {}", vm_id, e);
        }

        cleanup_result
    }

    async fn build_snapshot_inner(
        &self,
        vm_id: Uuid,
        snapshot_path: &str,
    ) -> Result<SnapshotMetadata> {
        // Wait for VM to get IP address
        let guest_ip = self
            .wait_for_guest_ip(vm_id, 60)
            .await
            .context("Failed to get guest IP")?;

        tracing::info!("VM {} got IP: {}", vm_id, guest_ip);

        // Wait for Docker daemon to be ready
        self.wait_for_docker_ready(&guest_ip, 120)
            .await
            .context("Docker daemon did not become ready in time")?;

        tracing::info!("Docker daemon ready on VM {}", vm_id);

        // Flush network configuration
        self.flush_network_config(vm_id, &guest_ip)
            .await
            .context("Failed to flush network config")?;

        tracing::info!("Network config flushed on VM {}", vm_id);

        // Stop guest agent to prevent reporting during snapshot
        self.stop_guest_agent(vm_id, &guest_ip)
            .await
            .context("Failed to stop guest agent")?;

        tracing::info!("Guest agent stopped on VM {}", vm_id);

        // Pause the VM before taking snapshot
        self.pause_vm(vm_id)
            .await
            .context("Failed to pause VM")?;

        tracing::info!("VM {} paused", vm_id);

        // Take the snapshot
        let metadata = self
            .take_snapshot(vm_id, snapshot_path)
            .await
            .context("Failed to take snapshot")?;

        tracing::info!(
            "Snapshot created at {} (size: {} bytes)",
            snapshot_path,
            metadata.total_size_bytes
        );

        Ok(metadata)
    }

    async fn create_temp_vm(&self, snapshot_id: Uuid, runtime_image_id: Uuid) -> Result<Uuid> {
        // Get container runtime image paths from the image registry
        let image_root =
            std::env::var("MANAGER_IMAGE_ROOT").unwrap_or_else(|_| "/srv/images".to_string());
        let image_repo =
            crate::features::images::repo::ImageRepository::new(self.state.db.clone(), image_root);

        let runtime_image = image_repo
            .get(runtime_image_id)
            .await
            .context("Runtime image not found")?;

        // Determine kernel path - use default if runtime image is rootfs-only
        let kernel_path = if runtime_image.kind == "kernel" {
            runtime_image.host_path.clone()
        } else {
            // Runtime image is rootfs, need to find kernel or use default
            std::env::var("CONTAINER_RUNTIME_KERNEL")
                .unwrap_or_else(|_| "/srv/images/vmlinux-5.10.fc.bin".to_string())
        };

        let rootfs_path = if runtime_image.kind == "rootfs" || runtime_image.kind == "container-runtime" {
            runtime_image.host_path.clone()
        } else {
            anyhow::bail!("Runtime image must be of kind 'rootfs' or 'container-runtime'");
        };

        // IMPORTANT: Use snapshot ID as VM ID so tap device name is predictable
        // This ensures the tap name will be: tap-{snapshot-id[..8]}-0
        // which can be recreated during snapshot restore
        let vm_id = snapshot_id;
        let vm_name = format!("runtime-snapshot-builder-{}", &snapshot_id.to_string()[..8]);

        let vm_req = CreateVmReq {
            name: vm_name,
            vcpu: 2,
            mem_mib: 512,
            kernel_image_id: None,
            rootfs_image_id: None,
            kernel_path: Some(kernel_path),
            rootfs_path: Some(rootfs_path),
            source_snapshot_id: None,
            username: Some("root".to_string()),
            password: Some("snapshot".to_string()),
            tags: vec!["type:runtime-snapshot-builder".to_string()],
        };

        crate::features::vms::service::create_and_start(&self.state, vm_id, vm_req, None).await?;

        Ok(vm_id)
    }

    async fn wait_for_guest_ip(&self, vm_id: Uuid, timeout_secs: u64) -> Result<String> {
        let start = std::time::Instant::now();
        let timeout = Duration::from_secs(timeout_secs);

        loop {
            if start.elapsed() > timeout {
                anyhow::bail!("Timeout waiting for guest IP");
            }

            let vm = crate::features::vms::repo::get(&self.state.db, vm_id).await?;

            if let Some(ip) = vm.guest_ip {
                return Ok(ip);
            }

            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    }

    async fn wait_for_docker_ready(&self, guest_ip: &str, timeout_secs: u64) -> Result<()> {
        let docker_url = format!("http://{}:2375/_ping", guest_ip);
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()?;

        let start = std::time::Instant::now();
        let timeout = Duration::from_secs(timeout_secs);

        loop {
            if start.elapsed() > timeout {
                anyhow::bail!("Timeout waiting for Docker daemon");
            }

            match client.get(&docker_url).send().await {
                Ok(resp) if resp.status().is_success() => {
                    tracing::info!("Docker daemon ready at {}", guest_ip);
                    return Ok(());
                }
                Ok(resp) => {
                    tracing::debug!("Docker ping returned status: {}", resp.status());
                }
                Err(e) => {
                    tracing::debug!("Docker ping failed: {}", e);
                }
            }

            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    }

    async fn flush_network_config(&self, vm_id: Uuid, guest_ip: &str) -> Result<()> {
        tracing::info!("Flushing network config on VM {} ({})", vm_id, guest_ip);

        // Use screen to send commands to the VM console
        let screen_session = format!("fc-{}", vm_id);
        // Only flush the IP address, keep eth0 UP.
        // After snapshot restore, eth0 stays UP so the guest agent
        // can immediately run DHCP without needing to bring it up first.
        let commands = vec![
            "ip addr flush dev eth0",
        ];

        for cmd in commands {
            // Send command via screen
            let status = tokio::process::Command::new("sudo")
                .args([
                    "screen", "-S", &screen_session, "-p", "0", "-X", "stuff",
                    &format!("{}\n", cmd),
                ])
                .status()
                .await?;

            if !status.success() {
                tracing::warn!("Failed to send network flush command to VM {}", vm_id);
            }

            tokio::time::sleep(Duration::from_millis(200)).await;
        }

        tracing::info!("Network config flushed on VM {}", vm_id);
        Ok(())
    }

    async fn stop_guest_agent(&self, vm_id: Uuid, _guest_ip: &str) -> Result<()> {
        // Keep ALL guest agent files (binary, config, service files) in the snapshot.
        // After snapshot restore:
        //   - The frozen guest agent process resumes and self-heals (network + config re-read)
        //   - If the process crashes (e.g., broken sockets from snapshot), OpenRC restarts it
        //   - The restarted process drops page caches, reads fresh config, and brings up network
        tracing::info!("Keeping guest agent intact in snapshot for VM {}", vm_id);
        Ok(())
    }

    async fn pause_vm(&self, vm_id: Uuid) -> Result<()> {
        let vm = crate::features::vms::repo::get(&self.state.db, vm_id).await?;

        let client = reqwest::Client::new();
        let base = format!("{}/agent/v1/vms/{}", vm.host_addr, vm.id);
        let qs = format!("?sock={}", urlencoding::encode(&vm.api_sock));
        let vm_url = format!("{}/proxy/vm{}", base, qs);

        client
            .patch(&vm_url)
            .json(&json!({"state": "Paused"}))
            .send()
            .await
            .context("Failed to pause VM")?
            .error_for_status()
            .context("Pause VM request failed")?;

        Ok(())
    }

    async fn take_snapshot(&self, vm_id: Uuid, snapshot_path: &str) -> Result<SnapshotMetadata> {
        let vm = crate::features::vms::repo::get(&self.state.db, vm_id).await?;

        // Create snapshot directory
        tokio::fs::create_dir_all(snapshot_path)
            .await
            .context("Failed to create snapshot directory")?;

        let mem_path = format!("{}/snapshot.mem", snapshot_path);
        let state_path = format!("{}/snapshot.state", snapshot_path);
        let rootfs_path = format!("{}/rootfs.ext4", snapshot_path);

        let client = reqwest::Client::new();
        let base = format!("{}/agent/v1/vms/{}", vm.host_addr, vm.id);
        let qs = format!("?sock={}", urlencoding::encode(&vm.api_sock));

        // Take Firecracker snapshot
        let snapshot_url = format!("{}/proxy/snapshot/create{}", base, qs);

        let snapshot_req = json!({
            "snapshot_type": "Full",
            "snapshot_path": state_path,
            "mem_file_path": mem_path,
        });

        client
            .put(&snapshot_url)
            .json(&snapshot_req)
            .send()
            .await
            .context("Failed to create snapshot")?
            .error_for_status()
            .context("Snapshot creation request failed")?;

        // Copy rootfs to snapshot directory
        let src_rootfs = &vm.rootfs_path;
        tokio::fs::copy(src_rootfs, &rootfs_path)
            .await
            .context("Failed to copy rootfs")?;

        // Calculate sizes
        let mem_size = tokio::fs::metadata(&mem_path)
            .await
            .context("Failed to get mem file size")?
            .len();

        let state_size = tokio::fs::metadata(&state_path)
            .await
            .context("Failed to get state file size")?
            .len();

        let rootfs_size = tokio::fs::metadata(&rootfs_path)
            .await
            .context("Failed to get rootfs file size")?
            .len();

        Ok(SnapshotMetadata {
            mem_size_bytes: mem_size,
            state_size_bytes: state_size,
            rootfs_size_bytes: rootfs_size,
            total_size_bytes: mem_size + state_size + rootfs_size,
            compressed: false, // TODO: Add compression support
        })
    }

    async fn cleanup_temp_vm(&self, vm_id: Uuid) -> Result<()> {
        tracing::info!("Cleaning up temporary VM {}", vm_id);

        // Stop and delete the VM
        if let Err(e) = crate::features::vms::service::stop_and_delete(&self.state, vm_id).await {
            tracing::warn!("Failed to delete temp VM {}: {}", vm_id, e);
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct SnapshotMetadata {
    pub mem_size_bytes: u64,
    pub state_size_bytes: u64,
    pub rootfs_size_bytes: u64,
    pub total_size_bytes: u64,
    pub compressed: bool,
}

/// Detect Firecracker version from the system
pub async fn detect_firecracker_version() -> Result<String> {
    use tokio::process::Command;

    // Try to detect firecracker version
    let output = Command::new("firecracker")
        .arg("--version")
        .output()
        .await;

    match output {
        Ok(out) if out.status.success() => {
            let version_str = String::from_utf8_lossy(&out.stdout);
            // Parse version from output like "Firecracker v1.9.0"
            let version = version_str
                .split_whitespace()
                .nth(1)
                .unwrap_or("unknown")
                .to_string();
            Ok(version)
        }
        _ => {
            // Fallback to a default version
            tracing::warn!("Could not detect Firecracker version, using default");
            Ok("v1.9.0".to_string())
        }
    }
}
