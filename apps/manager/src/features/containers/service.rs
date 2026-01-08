use anyhow::{anyhow, Result};
use nexus_types::{
    ContainerLogsResp, ContainerStatsResp, CreateContainerReq, CreateContainerResp, ExecCommandReq,
    ExecCommandResp, GetContainerResp, ListContainersResp, OkResponse, UpdateContainerReq,
};
use sqlx::PgPool;
use std::path::PathBuf;
use uuid::Uuid;

use super::docker::DockerClient;
use super::repo::{ContainerRepository, ContainerStatsData};
use crate::AppState;

/// Create a new container (spawns dedicated microVM with Docker inside)
///
/// This follows the same pattern as functions:
/// 1. Create container record in DB (state: creating)
/// 2. Spawn background task to:
///    a. Create dedicated Firecracker microVM with Docker runtime
///    b. Wait for VM to boot and get guest IP
///    c. Wait for Docker daemon to be ready inside VM
///    d. Pull container image inside VM
///    e. Create and start Docker container inside VM
///    f. Update container state to running
pub async fn create_container(
    st: &AppState,
    req: CreateContainerReq,
) -> Result<CreateContainerResp> {
    let repo = ContainerRepository::new(st.db.clone());

    // Validate request
    if req.name.is_empty() {
        return Err(anyhow!("Container name cannot be empty"));
    }
    if req.image.is_empty() {
        return Err(anyhow!("Container image cannot be empty"));
    }

    // Check port availability BEFORE creating the container
    if !req.port_mappings.is_empty() {
        let host_ports: Vec<u16> = req.port_mappings.iter().map(|p| p.host as u16).collect();

        let unavailable = super::port_forward::check_ports_available(&host_ports).await?;

        if !unavailable.is_empty() {
            let port_list = unavailable
                .iter()
                .map(|p| p.to_string())
                .collect::<Vec<_>>()
                .join(", ");
            return Err(anyhow!(
                "Port mapping failed: port(s) {} are already in use",
                port_list
            ));
        }

        // Reserve ports immediately to prevent race conditions
        for port in &host_ports {
            super::port_forward::reserve_port(*port);
        }
    }

    // Determine resource allocations (use defaults if not specified)
    let vcpu = req.cpu_limit.map(|c| c.ceil() as u8).unwrap_or(1);
    let memory_mb = req.memory_limit_mb.unwrap_or(512) as u32;

    // Create container record in database (state: creating)
    let container_id = repo.create(req.clone(), None).await?;

    // Spawn dedicated MicroVM for this container in the background
    let st_clone = st.clone();
    let container_name = req.name.clone();

    tokio::spawn(async move {
        if let Err(e) = provision_container_vm(
            &st_clone,
            container_id,
            &container_name,
            &req,
            vcpu,
            memory_mb,
        )
        .await
        {
            eprintln!("[Container {}] Failed to provision: {}", container_id, e);
            let error_msg = format!("Failed to provision container VM: {}", e);
            // Release reserved ports on failure
            for mapping in &req.port_mappings {
                super::port_forward::release_port(mapping.host as u16);
            }
            let _ = ContainerRepository::new(st_clone.db.clone())
                .update_state(container_id, "error", Some(error_msg))
                .await;
        }
    });

    Ok(CreateContainerResp { id: container_id })
}

/// Find a local Docker image tarball that was pre-downloaded via the registry feature
///
/// The registry feature saves Docker images as tarballs in {image_root}/docker/
/// with the image name sanitized (e.g., "postgres:latest" -> "postgres_latest.tar")
fn find_local_image_tarball(image: &str, image_root: &std::path::Path) -> Option<PathBuf> {
    // Sanitize image name the same way the registry download does
    let safe_name = image.replace(['/', ':', '.'], "_");
    let tarball_path = image_root.join("docker").join(format!("{}.tar", safe_name));

    if tarball_path.exists() {
        tracing::info!(
            image = %image,
            path = ?tarball_path,
            "Found local Docker image tarball"
        );
        Some(tarball_path)
    } else {
        tracing::debug!(
            image = %image,
            path = ?tarball_path,
            "No local Docker image tarball found"
        );
        None
    }
}

/// Background task to provision container VM and start Docker container
async fn provision_container_vm(
    st: &AppState,
    container_id: Uuid,
    container_name: &str,
    req: &CreateContainerReq,
    vcpu: u8,
    memory_mb: u32,
) -> Result<()> {
    let repo = ContainerRepository::new(st.db.clone());

    eprintln!("[Container {}] Starting VM provisioning", container_id);

    // Create dedicated microVM with Docker runtime
    let vm_id =
        super::vm::create_container_vm(st, container_id, container_name, vcpu, memory_mb).await?;

    eprintln!("[Container {}] VM created: {}", container_id, vm_id);

    // Update container with VM ID
    repo.update_runtime_id(container_id, format!("vm-{}", vm_id))
        .await?;
    repo.update_state(container_id, "booting", None).await?;

    // Wait for guest IP to be available (up to 60 seconds)
    eprintln!("[Container {}] Waiting for VM guest IP...", container_id);
    let mut guest_ip: Option<String> = None;
    for attempt in 1..=60 {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        // Check VM for guest IP
        if let Ok(vm) = crate::features::vms::repo::get(&st.db, vm_id).await {
            if let Some(ip) = vm.guest_ip {
                guest_ip = Some(ip);
                break;
            }
        }

        if attempt % 10 == 0 {
            eprintln!(
                "[Container {}] Still waiting for guest IP... ({}s)",
                container_id, attempt
            );
        }
    }

    let guest_ip = match guest_ip {
        Some(ip) => ip,
        None => {
            let error_msg = "Timeout waiting for VM guest IP";
            repo.update_state(container_id, "error", Some(error_msg.to_string()))
                .await?;
            anyhow::bail!(error_msg);
        }
    };

    eprintln!("[Container {}] Got guest IP: {}", container_id, guest_ip);

    // Wait for Docker daemon to be ready inside VM
    repo.update_state(container_id, "initializing", None)
        .await?;

    if let Err(e) = super::vm::wait_for_docker_ready(&guest_ip, 120).await {
        let error_msg = format!("Docker daemon not ready: {}", e);
        repo.update_state(container_id, "error", Some(error_msg.clone()))
            .await?;
        anyhow::bail!(error_msg);
    }

    eprintln!("[Container {}] Docker daemon ready", container_id);

    // Connect to Docker inside the VM
    let docker = DockerClient::new(&guest_ip)?;

    // Try to load image from local tarball first, otherwise pull from registry
    if !docker.image_exists(&req.image).await.unwrap_or(false) {
        // Check if we have a pre-downloaded tarball on the host
        if let Some(tarball_path) = find_local_image_tarball(&req.image, st.images.root()) {
            eprintln!(
                "[Container {}] Loading image from local tarball: {:?}",
                container_id, tarball_path
            );
            if let Err(e) = docker.load_image_from_tarball(&tarball_path).await {
                eprintln!(
                    "[Container {}] Failed to load from tarball, falling back to pull: {}",
                    container_id, e
                );
                // Fall back to pulling from registry
                if let Err(e) = docker
                    .pull_image(&req.image, req.registry_auth.as_ref())
                    .await
                {
                    let error_msg = format!("Failed to pull image: {}", e);
                    repo.update_state(container_id, "error", Some(error_msg.clone()))
                        .await?;
                    anyhow::bail!(error_msg);
                }
            } else {
                eprintln!(
                    "[Container {}] Successfully loaded image from local tarball",
                    container_id
                );
            }
        } else {
            // No local tarball, pull from registry
            eprintln!("[Container {}] Pulling image: {}", container_id, req.image);
            if let Err(e) = docker
                .pull_image(&req.image, req.registry_auth.as_ref())
                .await
            {
                let error_msg = format!("Failed to pull image: {}", e);
                repo.update_state(container_id, "error", Some(error_msg.clone()))
                    .await?;
                anyhow::bail!(error_msg);
            }
        }
    }

    // Create Docker container inside the VM
    let docker_container_id = match docker.create_container(req).await {
        Ok(id) => id,
        Err(e) => {
            let error_msg = format!("Failed to create Docker container: {}", e);
            repo.update_state(container_id, "error", Some(error_msg.clone()))
                .await?;
            anyhow::bail!(error_msg);
        }
    };

    eprintln!(
        "[Container {}] Docker container created: {}",
        container_id, docker_container_id
    );

    // Start the Docker container
    if let Err(e) = docker.start_container(&docker_container_id).await {
        let error_msg = format!("Failed to start Docker container: {}", e);
        repo.update_state(container_id, "error", Some(error_msg.clone()))
            .await?;
        anyhow::bail!(error_msg);
    }

    // Update container state to running
    repo.set_started(container_id).await?;

    // Set up port forwarding from host to container VM
    // The Docker container inside the VM has already been started with port mappings
    // Now we need to forward from host -> VM
    for mapping in &req.port_mappings {
        // The Docker container inside VM exposes on mapping.host (which Docker maps to container port)
        // We forward host:mapping.host -> vm_ip:mapping.host
        if let Err(e) = super::port_forward::setup_port_forward(
            mapping.host as u16,
            &guest_ip,
            mapping.host as u16, // Docker inside VM maps to the same port
            &mapping.protocol,
        )
        .await
        {
            eprintln!(
                "[Container {}] Warning: Failed to setup port forward for {}: {}",
                container_id, mapping.host, e
            );
        }
    }

    // Register container volumes in the volume registry so they appear on the Volumes page
    if !req.volumes.is_empty() {
        // Get host_id from the VM
        let vm = crate::features::vms::repo::get(&st.db, vm_id).await?;
        let host_id = vm.host_id;

        if let Err(e) = register_container_volumes(
            st,
            container_id,
            container_name,
            vm_id,
            &req.volumes,
            host_id,
        )
        .await
        {
            eprintln!(
                "[Container {}] Warning: Failed to register volumes: {}",
                container_id, e
            );
        } else {
            eprintln!(
                "[Container {}] Registered {} volume(s) in volume registry",
                container_id,
                req.volumes.len()
            );
        }
    }

    eprintln!(
        "[Container {}] Container running successfully with {} port mappings and {} volumes",
        container_id,
        req.port_mappings.len(),
        req.volumes.len()
    );

    Ok(())
}

/// List all containers
pub async fn list_containers(
    db: &PgPool,
    state_filter: Option<String>,
    host_filter: Option<Uuid>,
) -> Result<ListContainersResp> {
    let repo = ContainerRepository::new(db.clone());
    let containers = repo.list(state_filter, host_filter).await?;

    Ok(ListContainersResp { items: containers })
}

/// Get a single container
pub async fn get_container(db: &PgPool, id: Uuid) -> Result<GetContainerResp> {
    let repo = ContainerRepository::new(db.clone());
    let mut container = repo.get(id).await?;

    // Try to get latest stats if container is running
    if container.state == "running" {
        if let Ok(stats) = get_latest_stats(db, id).await {
            if let Some(latest) = stats.items.first() {
                container.cpu_percent = latest.cpu_percent;
                container.memory_used_mb = latest.memory_used_mb;
            }
        }
    }

    Ok(GetContainerResp { item: container })
}

/// Update a container
pub async fn update_container(
    st: &AppState,
    id: Uuid,
    req: UpdateContainerReq,
) -> Result<GetContainerResp> {
    let repo = ContainerRepository::new(st.db.clone());

    // Check if container exists
    let container = repo.get(id).await?;

    // Don't allow updates to running containers
    if container.state == "running" {
        return Err(anyhow!("Cannot update a running container. Stop it first."));
    }

    // Perform update
    repo.update(id, req).await?;

    // Return updated container
    get_container(&st.db, id).await
}

/// Delete a container (stops VM and removes all resources)
pub async fn delete_container(st: &AppState, id: Uuid) -> Result<OkResponse> {
    let repo = ContainerRepository::new(st.db.clone());

    let container = repo.get(id).await?;

    // Clean up port forwarding rules first
    if !container.port_mappings.is_empty() {
        if let Some(guest_ip) = &container.guest_ip {
            if let Err(e) =
                super::port_forward::cleanup_port_forwards(&container.port_mappings, guest_ip).await
            {
                eprintln!("[Container {}] Failed to cleanup port forwards: {}", id, e);
            }
        } else {
            // Just release the ports from our registry
            for mapping in &container.port_mappings {
                super::port_forward::release_port(mapping.host as u16);
            }
        }
    }

    // Extract VM ID from runtime_id (format: "vm-<uuid>")
    if let Some(runtime_id) = &container.container_runtime_id {
        if let Some(vm_id_str) = runtime_id.strip_prefix("vm-") {
            if let Ok(vm_id) = Uuid::parse_str(vm_id_str) {
                // Clean up the VM and associated resources
                if let Err(e) = super::vm::cleanup_container_vm(st, vm_id).await {
                    eprintln!("[Container {}] Failed to cleanup VM: {}", id, e);
                    // Continue with database deletion even if VM cleanup fails
                }
            }
        }
    }

    // Delete from database
    repo.delete(id).await?;

    Ok(OkResponse::default())
}

/// Start a container (VM should already be running, just start Docker container)
pub async fn start_container(st: &AppState, id: Uuid) -> Result<OkResponse> {
    let repo = ContainerRepository::new(st.db.clone());

    let container = repo.get(id).await?;

    if container.state == "running" {
        return Err(anyhow!("Container is already running"));
    }

    // Extract VM ID from runtime_id
    let runtime_id = container
        .container_runtime_id
        .as_ref()
        .ok_or_else(|| anyhow!("Container has no runtime ID"))?;

    let vm_id_str = runtime_id
        .strip_prefix("vm-")
        .ok_or_else(|| anyhow!("Invalid runtime ID format"))?;

    let vm_id = Uuid::parse_str(vm_id_str)?;

    // Get VM
    let vm = crate::features::vms::repo::get(&st.db, vm_id).await?;

    // Check if VM needs to be started
    if vm.state != "running" || vm.guest_ip.is_none() {
        tracing::info!(
            container_id = %id,
            vm_id = %vm_id,
            vm_state = %vm.state,
            has_guest_ip = vm.guest_ip.is_some(),
            "VM not ready, starting VM first"
        );

        // Start the VM if it's not running
        crate::features::vms::service::start_vm_by_id(st, vm_id).await?;

        // Wait for guest IP to be available (up to 60 seconds)
        let mut guest_ip: Option<String> = None;
        for attempt in 1..=60 {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;

            if let Ok(vm) = crate::features::vms::repo::get(&st.db, vm_id).await {
                if let Some(ip) = vm.guest_ip {
                    guest_ip = Some(ip);
                    break;
                }
            }

            if attempt % 10 == 0 {
                tracing::info!(
                    container_id = %id,
                    vm_id = %vm_id,
                    attempt = attempt,
                    "Still waiting for VM guest IP..."
                );
            }
        }

        if guest_ip.is_none() {
            return Err(anyhow!(
                "Timed out waiting for VM guest IP. VM may not have started properly."
            ));
        }
    }

    // Get guest IP from VM
    let guest_ip = get_guest_ip_from_container(&st.db, &container).await?;

    // Connect to Docker inside VM
    let docker = DockerClient::new(&guest_ip)?;

    // Extract Docker container ID from runtime_id
    let docker_container_id = extract_docker_container_id(&container)?;

    docker.start_container(&docker_container_id).await?;
    repo.set_started(id).await?;

    tracing::info!(container_id = %id, "Container started");

    Ok(OkResponse::default())
}

/// Stop a container
pub async fn stop_container(st: &AppState, id: Uuid) -> Result<OkResponse> {
    let repo = ContainerRepository::new(st.db.clone());

    let container = repo.get(id).await?;

    if container.state != "running" {
        return Err(anyhow!("Container is not running"));
    }

    // Try to get guest IP, but handle the case where VM might be stopped
    let guest_ip_result = get_guest_ip_from_container(&st.db, &container).await;

    match guest_ip_result {
        Ok(guest_ip) => {
            // VM is running, try to stop container gracefully via Docker
            let docker = DockerClient::new(&guest_ip)?;
            let docker_container_id = extract_docker_container_id(&container)?;

            match docker.stop_container(&docker_container_id, Some(10)).await {
                Ok(_) => {
                    tracing::info!(container_id = %id, "Container stopped via Docker");
                }
                Err(e) => {
                    tracing::warn!(
                        container_id = %id,
                        error = %e,
                        "Failed to stop container via Docker, will mark as stopped anyway"
                    );
                }
            }
        }
        Err(e) => {
            tracing::warn!(
                container_id = %id,
                error = %e,
                "VM guest IP not available, marking container as stopped"
            );
            // VM is not reachable, but we can still update the database state
        }
    }

    repo.set_stopped(id).await?;
    tracing::info!(container_id = %id, "Container marked as stopped");

    Ok(OkResponse::default())
}

/// Restart a container
pub async fn restart_container(st: &AppState, id: Uuid) -> Result<OkResponse> {
    let repo = ContainerRepository::new(st.db.clone());

    let container = repo.get(id).await?;

    let guest_ip = get_guest_ip_from_container(&st.db, &container).await?;
    let docker = DockerClient::new(&guest_ip)?;
    let docker_container_id = extract_docker_container_id(&container)?;

    docker
        .restart_container(&docker_container_id, Some(10))
        .await?;
    repo.set_started(id).await?;

    tracing::info!(container_id = %id, "Container restarted");

    Ok(OkResponse::default())
}

/// Pause a container
pub async fn pause_container(st: &AppState, id: Uuid) -> Result<OkResponse> {
    let repo = ContainerRepository::new(st.db.clone());

    let container = repo.get(id).await?;

    if container.state != "running" {
        return Err(anyhow!("Container is not running"));
    }

    let guest_ip = get_guest_ip_from_container(&st.db, &container).await?;
    let docker = DockerClient::new(&guest_ip)?;
    let docker_container_id = extract_docker_container_id(&container)?;

    docker.pause_container(&docker_container_id).await?;
    repo.update_state(id, "paused", None).await?;

    tracing::info!(container_id = %id, "Container paused");

    Ok(OkResponse::default())
}

/// Resume (unpause) a container
pub async fn resume_container(st: &AppState, id: Uuid) -> Result<OkResponse> {
    let repo = ContainerRepository::new(st.db.clone());

    let container = repo.get(id).await?;

    if container.state != "paused" {
        return Err(anyhow!("Container is not paused"));
    }

    let guest_ip = get_guest_ip_from_container(&st.db, &container).await?;
    let docker = DockerClient::new(&guest_ip)?;
    let docker_container_id = extract_docker_container_id(&container)?;

    docker.unpause_container(&docker_container_id).await?;
    repo.update_state(id, "running", None).await?;

    tracing::info!(container_id = %id, "Container resumed");

    Ok(OkResponse::default())
}

/// Get container logs
pub async fn get_container_logs(
    db: &PgPool,
    id: Uuid,
    tail: Option<i64>,
) -> Result<ContainerLogsResp> {
    let repo = ContainerRepository::new(db.clone());

    // Verify container exists
    let _ = repo.get(id).await?;

    let logs = repo.get_logs(id, tail).await?;

    Ok(ContainerLogsResp { items: logs })
}

/// Get container stats
pub async fn get_container_stats(st: &AppState, id: Uuid) -> Result<ContainerStatsResp> {
    let repo = ContainerRepository::new(st.db.clone());

    let container = repo.get(id).await?;

    // If container is running, fetch live stats from Docker
    if container.state == "running" {
        if let Ok(guest_ip) = get_guest_ip_from_container(&st.db, &container).await {
            if let Ok(docker_container_id) = extract_docker_container_id(&container) {
                let docker = DockerClient::new(&guest_ip)?;

                match docker.get_stats(&docker_container_id).await {
                    Ok(docker_stats) => {
                        // Record stats in database
                        let stats_data = ContainerStatsData {
                            cpu_percent: docker_stats.cpu_percent,
                            memory_used_mb: docker_stats.memory_used_mb,
                            memory_limit_mb: docker_stats.memory_limit_mb,
                            network_rx_bytes: docker_stats.network_rx_bytes,
                            network_tx_bytes: docker_stats.network_tx_bytes,
                            block_read_bytes: docker_stats.block_read_bytes,
                            block_write_bytes: docker_stats.block_write_bytes,
                            pids: docker_stats.pids,
                        };

                        let _ = repo.record_stats(id, &stats_data).await;
                    }
                    Err(e) => {
                        tracing::warn!(error = ?e, "Failed to fetch live stats");
                    }
                }
            }
        }
    }

    // Return latest stats from database
    get_latest_stats(&st.db, id).await
}

/// Get latest stats from database
async fn get_latest_stats(db: &PgPool, id: Uuid) -> Result<ContainerStatsResp> {
    let repo = ContainerRepository::new(db.clone());
    let stats = repo.get_latest_stats(id, 10).await?;

    Ok(ContainerStatsResp { items: stats })
}

/// Execute a command in a container
pub async fn exec_command(st: &AppState, id: Uuid, req: ExecCommandReq) -> Result<ExecCommandResp> {
    let repo = ContainerRepository::new(st.db.clone());

    let container = repo.get(id).await?;

    if container.state != "running" {
        return Err(anyhow!("Container must be running to exec commands"));
    }

    let guest_ip = get_guest_ip_from_container(&st.db, &container).await?;
    let docker = DockerClient::new(&guest_ip)?;
    let docker_container_id = extract_docker_container_id(&container)?;

    let result = docker
        .exec_command(
            &docker_container_id,
            req.command,
            req.attach_stdout,
            req.attach_stderr,
        )
        .await?;

    Ok(ExecCommandResp {
        exec_id: result.exec_id,
        output: result.output,
        exit_code: result.exit_code,
    })
}

// Helper functions

async fn get_guest_ip_from_container(
    db: &PgPool,
    container: &nexus_types::Container,
) -> Result<String> {
    // Extract VM ID from runtime_id
    let runtime_id = container
        .container_runtime_id
        .as_ref()
        .ok_or_else(|| anyhow!("Container has no runtime ID"))?;

    let vm_id_str = runtime_id
        .strip_prefix("vm-")
        .ok_or_else(|| anyhow!("Invalid runtime ID format"))?;

    let vm_id = Uuid::parse_str(vm_id_str)?;

    // Get VM and extract guest IP
    let vm = crate::features::vms::repo::get(db, vm_id).await?;

    vm.guest_ip.ok_or_else(|| anyhow!("VM has no guest IP"))
}

fn extract_docker_container_id(container: &nexus_types::Container) -> Result<String> {
    // For now, the Docker container ID is stored separately
    // We'll use the container's runtime_id which includes VM info
    // In a full implementation, we'd store the Docker container ID separately

    // Placeholder: use container name as Docker container name
    Ok(container.name.clone())
}

/// Register container volumes in the volume registry
/// This creates volume records so they appear on the Volumes page and can be tracked
async fn register_container_volumes(
    st: &AppState,
    container_id: Uuid,
    container_name: &str,
    vm_id: Uuid,
    volumes: &[nexus_types::VolumeMount],
    host_id: Uuid,
) -> Result<()> {
    use crate::features::volumes::repo::VolumeRepository;
    use tracing::info;

    if volumes.is_empty() {
        return Ok(());
    }

    let volume_repo = VolumeRepository::new(st.db.clone());

    for (idx, volume_mount) in volumes.iter().enumerate() {
        // Check if volume with this path already exists
        let existing = volume_repo.list_by_host(host_id).await?;
        let already_exists = existing.iter().any(|v| v.path == volume_mount.host);

        if already_exists {
            info!(
                container_id = %container_id,
                path = %volume_mount.host,
                "Volume already registered, skipping creation"
            );

            // Find and attach the existing volume
            if let Some(existing_vol) = existing.iter().find(|v| v.path == volume_mount.host) {
                let drive_id = format!("container-vol-{}", idx);
                if let Err(e) = volume_repo.attach(existing_vol.id, vm_id, &drive_id).await {
                    tracing::warn!(
                        container_id = %container_id,
                        volume_id = %existing_vol.id,
                        error = %e,
                        "Failed to attach existing volume"
                    );
                }
            }
            continue;
        }

        // Create new volume record
        let volume_name = format!("{} - Volume {}", container_name, idx + 1);
        let description = format!(
            "Container volume mounted at {} (container: {})",
            volume_mount.container, container_name
        );

        // Get size from filesystem if possible, otherwise use 0
        let size_bytes = std::fs::metadata(&volume_mount.host)
            .ok()
            .map(|m| m.len() as i64)
            .unwrap_or(0);

        let volume_type = "container-bind";

        match volume_repo
            .create(
                &volume_name,
                Some(&description),
                &volume_mount.host,
                size_bytes,
                volume_type,
                host_id,
            )
            .await
        {
            Ok(volume) => {
                info!(
                    container_id = %container_id,
                    volume_id = %volume.id,
                    name = %volume_name,
                    path = %volume_mount.host,
                    "Container volume registered"
                );

                // Attach volume to the container's VM
                let drive_id = format!("container-vol-{}", idx);
                if let Err(e) = volume_repo.attach(volume.id, vm_id, &drive_id).await {
                    tracing::warn!(
                        container_id = %container_id,
                        volume_id = %volume.id,
                        error = %e,
                        "Failed to attach volume to VM"
                    );
                }
            }
            Err(e) => {
                tracing::warn!(
                    container_id = %container_id,
                    path = %volume_mount.host,
                    error = %e,
                    "Failed to register container volume"
                );
            }
        }
    }

    Ok(())
}
