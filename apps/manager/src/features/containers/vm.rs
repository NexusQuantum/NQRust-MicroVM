use crate::AppState;
use anyhow::{Context, Result};
use nexus_types::{AuditAction, CreateVmReq};
use serde_json::json;
use uuid::Uuid;

use crate::features::users::audit;

/// Create a dedicated MicroVM for running a Docker container
///
/// This spawns a lightweight VM with:
/// - Container runtime rootfs (Alpine/Ubuntu with Docker pre-installed)
/// - Docker daemon auto-starting on boot and listening on TCP
/// - Minimal resources (configurable vCPU and memory)
/// - Network connectivity for Docker API access
pub async fn create_container_vm(
    st: &AppState,
    container_id: Uuid,
    container_name: &str,
    vcpu: u8,
    memory_mb: u32,
) -> Result<Uuid> {
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
        rootfs_size_mb: None,
    };

    // Create and start VM
    crate::features::vms::service::create_and_start(st, vm_id, vm_req, None, None, "system")
        .await?;

    eprintln!(
        "[Container {}] VM {} created and starting",
        container_id, vm_id
    );

    let _ = audit::log_action(
        &st.db,
        None,
        "system",
        AuditAction::SystemEvent,
        Some("container"),
        Some(container_id),
        Some(json!({"event": "container_vm_created", "vm_id": vm_id.to_string()})),
        None,
        true,
        None,
    )
    .await;

    Ok(vm_id)
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
