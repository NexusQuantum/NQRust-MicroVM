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

    eprintln!(
        "[Container {}] Container running successfully",
        container_id
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

    // Get guest IP
    let guest_ip = get_guest_ip_from_container(&st.db, &container).await?;

    // Connect to Docker
    let docker = DockerClient::new(&guest_ip)?;

    let docker_container_id = extract_docker_container_id(&container)?;

    docker
        .stop_container(&docker_container_id, Some(10))
        .await?;
    repo.set_stopped(id).await?;

    tracing::info!(container_id = %id, "Container stopped");

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
