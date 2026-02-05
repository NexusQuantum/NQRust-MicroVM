use crate::{features::snapshots::repo::SnapshotRow, AppState};
use anyhow::{anyhow, bail, Context, Result};
use nexus_types::{
    AuditAction, BalloonConfig, BalloonStatsConfig, CpuConfigReq, CreateDriveReq, CreateNicReq,
    CreateVmReq, EntropyConfigReq, LoggerUpdateReq, MachineConfigPatchReq, MmdsConfigReq,
    MmdsDataReq, SerialConfigReq, UpdateDriveReq, UpdateNicReq, VsockConfigReq,
};
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;
use serde_json::Value;
use sqlx::PgPool;
use std::path::Path;
use std::time::{Duration, Instant};
use tracing::{info, warn};
use uuid::Uuid;

use crate::features::users::audit;

struct NetworkSelection {
    bridge: String,
}

fn select_network(capabilities: &Value) -> Result<NetworkSelection> {
    if let Some(bridge) = capabilities.get("bridge").and_then(|v| v.as_str()) {
        return Ok(NetworkSelection {
            bridge: bridge.to_string(),
        });
    }
    Err(anyhow!("host capabilities missing bridge name"))
}

fn normalize_rate_limiter(raw: &Value) -> Value {
    match raw {
        Value::Object(obj) => {
            if obj.contains_key("bandwidth") || obj.contains_key("ops") {
                return raw.clone();
            }

            let mut normalized = serde_json::Map::new();
            let mut bandwidth = serde_json::Map::new();

            if let Some(value) = obj.get("size") {
                bandwidth.insert("size".to_string(), value.clone());
            }
            if let Some(value) = obj.get("one_time_burst") {
                bandwidth.insert("one_time_burst".to_string(), value.clone());
            }
            if let Some(value) = obj.get("refill_time") {
                bandwidth.insert("refill_time".to_string(), value.clone());
            }

            if !bandwidth.is_empty() {
                normalized.insert("bandwidth".to_string(), Value::Object(bandwidth));
            }

            if let Some(ops) = obj.get("ops") {
                normalized.insert("ops".to_string(), ops.clone());
            }

            if normalized.is_empty() {
                raw.clone()
            } else {
                Value::Object(normalized)
            }
        }
        _ => raw.clone(),
    }
}

pub async fn create_and_start(
    st: &AppState,
    id: Uuid,
    mut req: CreateVmReq,
    template_id: Option<Uuid>,
    user_id: Option<Uuid>,
    audit_username: &str,
) -> Result<()> {
    if let Some(snapshot_id) = req.source_snapshot_id.take() {
        let name = req.name.clone();
        let snapshot = st
            .snapshots
            .get(snapshot_id)
            .await
            .with_context(|| format!("failed to load snapshot {snapshot_id}"))?;
        return create_from_snapshot(st, id, name, template_id, snapshot, None).await;
    }

    let host = st
        .hosts
        .first_healthy()
        .await
        .context("no healthy hosts available")?;
    let network = select_network(&host.capabilities_json)?;
    let paths = VmPaths::new(id, &st.storage).await?;

    // Extract credentials and tags before moving req into resolve_vm_spec
    let username = req.username.clone().unwrap_or_else(|| "root".to_string());
    let password = req
        .password
        .clone()
        .unwrap_or_else(|| format!("vm-{}", &id.to_string()[..8]));
    let tags = req.tags.clone();

    let spec = resolve_vm_spec(st, req, id).await?;

    // Inject credentials into rootfs BEFORE VM starts (while rootfs is not in use)
    // This is the fallback for images without cloud-init
    if let Err(e) = inject_credentials_to_rootfs(id, &spec.rootfs_path, &username, &password).await
    {
        warn!(vm_id = %id, error = ?e, "rootfs credential injection failed (will try cloud-init)");
    }

    // Install guest agent into rootfs BEFORE VM starts (while rootfs is not in use)
    // Get manager URL from MANAGER_BIND (use bridge IP from network.bridge)
    let manager_bind =
        std::env::var("MANAGER_BIND").unwrap_or_else(|_| "127.0.0.1:18080".to_string());

    // Get bridge IP for manager URL (VMs connect via bridge network)
    let bridge_ip = std::process::Command::new("ip")
        .args(["addr", "show", &network.bridge])
        .output()
        .ok()
        .and_then(|output| {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if line.trim().starts_with("inet ") {
                    if let Some(ip_part) = line.split_whitespace().nth(1) {
                        if let Some(ip) = ip_part.split('/').next() {
                            return Some(ip.to_string());
                        }
                    }
                }
            }
            None
        })
        .unwrap_or_else(|| {
            manager_bind
                .split(':')
                .next()
                .unwrap_or("127.0.0.1")
                .to_string()
        });

    let manager_port = manager_bind.split(':').nth(1).unwrap_or("18080");
    let manager_url = format!("http://{}:{}", bridge_ip, manager_port);

    eprintln!("=== GUEST AGENT INSTALLATION STARTED for VM {} ===", id);
    eprintln!("Rootfs path: {}", &spec.rootfs_path);
    eprintln!("Manager bind: {}", manager_bind);
    eprintln!("Bridge: {}", network.bridge);
    eprintln!("Bridge IP: {}", bridge_ip);
    eprintln!("Manager port: {}", manager_port);
    eprintln!("Manager URL: {}", &manager_url);
    if let Err(e) = super::guest_agent::install_to_rootfs(&spec.rootfs_path, id, &manager_url).await
    {
        eprintln!("=== GUEST AGENT INSTALLATION FAILED for VM {} ===", id);
        eprintln!("Error: {:?}", e);
        warn!(vm_id = %id, error = ?e, "failed to install guest agent (continuing without it)");
        let _ = audit::log_action(
            &st.db,
            None,
            "system",
            AuditAction::SystemEvent,
            Some("vm"),
            Some(id),
            Some(json!({"event": "guest_agent_install_failed", "error": e.to_string()})),
            None,
            false,
            Some("guest agent installation failed"),
        )
        .await;
    } else {
        eprintln!("=== GUEST AGENT INSTALLATION SUCCESS for VM {} ===", id);
        let _ = audit::log_action(
            &st.db,
            None,
            "system",
            AuditAction::SystemEvent,
            Some("vm"),
            Some(id),
            Some(json!({"event": "guest_agent_installed"})),
            None,
            true,
            None,
        )
        .await;
    }

    create_tap(&host.addr, id, &network.bridge).await?;
    spawn_firecracker(st, &host.addr, id, &paths).await?;
    if std::env::var("MANAGER_TEST_MODE").is_ok() {
        eprintln!("MANAGER_TEST_MODE: Skipping VM configuration");
    } else {
        configure_vm(st, &host.addr, id, &spec, &paths).await?;
    }

    if std::env::var("MANAGER_TEST_MODE").is_ok() {
        eprintln!("MANAGER_TEST_MODE: Skipping VM start");
    } else {
        start_vm(&host.addr, id, &paths).await?;
    }

    super::repo::insert(
        &st.db,
        &super::repo::VmRow {
            id,
            name: spec.name.clone(),
            state: "running".into(),
            host_id: host.id,
            template_id,
            host_addr: host.addr.clone(),
            api_sock: paths.sock.clone(),
            tap: paths.tap.clone(),
            log_path: paths.log_path.clone(),
            http_port: 0,
            fc_unit: paths.fc_unit.clone(),
            vcpu: spec.vcpu as i32,
            mem_mib: spec.mem_mib as i32,
            kernel_path: spec.kernel_path.clone(),
            rootfs_path: spec.rootfs_path.clone(),
            source_snapshot_id: None,
            guest_ip: None, // Will be set when guest agent reports
            tags,
            created_by_user_id: None, // TODO: Set from authenticated user context
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        },
    )
    .await?;

    // Auto-register network if it doesn't exist
    info!(vm_id = %id, bridge = %network.bridge, host_id = %host.id, "attempting to auto-register network");
    let network_id_opt = match ensure_network_registered(st, &network.bridge, host.id).await {
        Ok(network_id) => {
            info!(vm_id = %id, bridge = %network.bridge, network_id = %network_id, "network auto-registration successful or already exists");
            Some(network_id)
        }
        Err(e) => {
            warn!(vm_id = %id, bridge = %network.bridge, error = ?e, "failed to auto-register network");
            None
        }
    };

    // Create default eth0 NIC record in database
    if let Some(network_id) = network_id_opt {
        info!(vm_id = %id, tap = %paths.tap, network_id = %network_id, "creating default eth0 NIC record");
        match super::repo::nics::insert(
            &st.db,
            id,
            "eth0",
            &paths.tap,
            None, // guest_mac auto-generated by Firecracker
            None, // rx_rate_limiter
            None, // tx_rate_limiter
            Some(network_id),
            None, // assigned_ip - eth0 uses DHCP from bridge network
        )
        .await
        {
            Ok(_) => info!(vm_id = %id, "default eth0 NIC record created successfully"),
            Err(e) => warn!(vm_id = %id, error = ?e, "failed to create default eth0 NIC record"),
        }
    }

    // Auto-register rootfs volume if it doesn't exist
    info!(vm_id = %id, rootfs = %spec.rootfs_path, host_id = %host.id, "attempting to auto-register rootfs volume");
    match ensure_volume_registered(st, id, &spec.rootfs_path, host.id).await {
        Ok(_) => {
            info!(vm_id = %id, rootfs = %spec.rootfs_path, "volume auto-registration successful or already exists")
        }
        Err(e) => {
            warn!(vm_id = %id, rootfs = %spec.rootfs_path, error = ?e, "failed to auto-register rootfs volume")
        }
    }

    // Store shell credentials for the VM (use the same credentials that were injected)
    if let Err(e) = st
        .shell_repo
        .upsert_credentials(id, &username, &password)
        .await
    {
        warn!(vm_id = %id, error = ?e, "failed to create shell credentials for VM");
    } else {
        info!(vm_id = %id, username = %username, "created shell credentials for VM");
    }

    // Configure cloud-init with credentials and network AFTER VM is inserted in DB
    // This enables DHCP networking for cloud-init enabled images
    if let Err(e) = configure_cloud_init_with_network(st, id, &username, &password).await {
        warn!(vm_id = %id, error = ?e, "cloud-init configuration failed (not critical if image lacks cloud-init)");
    }

    let _ = audit::log_action(
        &st.db,
        user_id,
        audit_username,
        AuditAction::CreateVm,
        Some("vm"),
        Some(id),
        None,
        None,
        true,
        None,
    )
    .await;
    Ok(())
}

pub async fn create_from_snapshot(
    st: &AppState,
    id: Uuid,
    name: String,
    template_id: Option<Uuid>,
    snapshot: SnapshotRow,
    source_vm: Option<super::repo::VmRow>,
) -> Result<()> {
    let SnapshotRow {
        id: source_snapshot_id,
        vm_id,
        ref snapshot_path,
        ref mem_path,
        ..
    } = snapshot;

    let source_vm = match source_vm {
        Some(vm) => vm,
        None => super::repo::get(&st.db, vm_id)
            .await
            .with_context(|| format!("failed to load source vm {vm_id}"))?,
    };
    ensure_allowed_path(st, &source_vm.kernel_path)?;
    ensure_allowed_path(st, &source_vm.rootfs_path)?;

    let host = st
        .hosts
        .get(source_vm.host_id)
        .await
        .with_context(|| format!("failed to load host {}", source_vm.host_id))?;
    let spec = ResolvedVmSpec {
        name: name.clone(),
        vcpu: source_vm
            .vcpu
            .try_into()
            .context("stored vcpu exceeds u8")?,
        mem_mib: source_vm
            .mem_mib
            .try_into()
            .context("stored mem_mib negative")?,
        kernel_path: source_vm.kernel_path.clone(),
        rootfs_path: source_vm.rootfs_path.clone(),
        rootfs_size_bytes: None,
    };

    let paths = VmPaths::new(id, &st.storage)
        .await?
        .with_snapshot(snapshot_path.clone(), mem_path.clone());

    let network = select_network(&host.capabilities_json)?;

    // Install guest agent into rootfs BEFORE VM starts (while rootfs is not in use)
    // Get manager URL from MANAGER_BIND (use bridge IP from network.bridge)
    let manager_bind =
        std::env::var("MANAGER_BIND").unwrap_or_else(|_| "127.0.0.1:18080".to_string());

    // Get bridge IP for manager URL (VMs connect via bridge network)
    let bridge_ip = std::process::Command::new("ip")
        .args(["addr", "show", &network.bridge])
        .output()
        .ok()
        .and_then(|output| {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if line.trim().starts_with("inet ") {
                    if let Some(ip_part) = line.split_whitespace().nth(1) {
                        if let Some(ip) = ip_part.split('/').next() {
                            return Some(ip.to_string());
                        }
                    }
                }
            }
            None
        })
        .unwrap_or_else(|| {
            manager_bind
                .split(':')
                .next()
                .unwrap_or("127.0.0.1")
                .to_string()
        });

    let manager_port = manager_bind.split(':').nth(1).unwrap_or("18080");
    let manager_url = format!("http://{}:{}", bridge_ip, manager_port);

    eprintln!(
        "=== GUEST AGENT INSTALLATION STARTED for VM {} (from snapshot) ===",
        id
    );
    eprintln!("Rootfs path: {}", &spec.rootfs_path);
    eprintln!("Manager bind: {}", manager_bind);
    eprintln!("Bridge: {}", network.bridge);
    eprintln!("Bridge IP: {}", bridge_ip);
    eprintln!("Manager port: {}", manager_port);
    eprintln!("Manager URL: {}", &manager_url);
    if let Err(e) = super::guest_agent::install_to_rootfs(&spec.rootfs_path, id, &manager_url).await
    {
        eprintln!(
            "=== GUEST AGENT INSTALLATION FAILED for VM {} (from snapshot) ===",
            id
        );
        eprintln!("Error: {:?}", e);
        warn!(vm_id = %id, error = ?e, "failed to install guest agent (continuing without it)");
        let _ = audit::log_action(&st.db, None, "system", AuditAction::SystemEvent, Some("vm"), Some(id), Some(json!({"event": "guest_agent_install_failed", "source": "snapshot", "error": e.to_string()})), None, false, Some("guest agent installation failed")).await;
    } else {
        eprintln!(
            "=== GUEST AGENT INSTALLATION SUCCESS for VM {} (from snapshot) ===",
            id
        );
        let _ = audit::log_action(
            &st.db,
            None,
            "system",
            AuditAction::SystemEvent,
            Some("vm"),
            Some(id),
            Some(json!({"event": "guest_agent_installed", "source": "snapshot"})),
            None,
            true,
            None,
        )
        .await;
    }

    create_tap(&host.addr, id, &network.bridge).await?;
    spawn_firecracker(st, &host.addr, id, &paths).await?;
    if std::env::var("MANAGER_TEST_MODE").is_ok() {
        eprintln!("MANAGER_TEST_MODE: Skipping VM configuration");
    } else {
        configure_vm(st, &host.addr, id, &spec, &paths).await?;
    }
    load_snapshot(st, id, &snapshot).await?;
    if std::env::var("MANAGER_TEST_MODE").is_ok() {
        eprintln!("MANAGER_TEST_MODE: Skipping VM start");
    } else {
        start_vm(&host.addr, id, &paths).await?;
    }

    super::repo::insert(
        &st.db,
        &super::repo::VmRow {
            id,
            name: name.clone(),
            state: "running".into(),
            host_id: host.id,
            template_id: template_id.or(source_vm.template_id),
            host_addr: host.addr.clone(),
            api_sock: paths.sock.clone(),
            tap: paths.tap.clone(),
            log_path: paths.log_path.clone(),
            http_port: 0,
            fc_unit: paths.fc_unit.clone(),
            vcpu: spec.vcpu as i32,
            mem_mib: spec.mem_mib as i32,
            kernel_path: spec.kernel_path.clone(),
            rootfs_path: spec.rootfs_path.clone(),
            source_snapshot_id: Some(source_snapshot_id),
            guest_ip: None,               // Will be set when guest agent reports
            tags: source_vm.tags.clone(), // Preserve tags from source VM
            created_by_user_id: source_vm.created_by_user_id, // Preserve ownership from source VM
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        },
    )
    .await?;

    // Auto-register network if it doesn't exist
    info!(vm_id = %id, bridge = %network.bridge, host_id = %host.id, "attempting to auto-register network");
    let network_id_opt = match ensure_network_registered(st, &network.bridge, host.id).await {
        Ok(network_id) => {
            info!(vm_id = %id, bridge = %network.bridge, network_id = %network_id, "network auto-registration successful or already exists");
            Some(network_id)
        }
        Err(e) => {
            warn!(vm_id = %id, bridge = %network.bridge, error = ?e, "failed to auto-register network");
            None
        }
    };

    // Create default eth0 NIC record in database
    if let Some(network_id) = network_id_opt {
        info!(vm_id = %id, tap = %paths.tap, network_id = %network_id, "creating default eth0 NIC record");
        match super::repo::nics::insert(
            &st.db,
            id,
            "eth0",
            &paths.tap,
            None, // guest_mac auto-generated by Firecracker
            None, // rx_rate_limiter
            None, // tx_rate_limiter
            Some(network_id),
            None, // assigned_ip - eth0 uses DHCP from bridge network
        )
        .await
        {
            Ok(_) => info!(vm_id = %id, "default eth0 NIC record created successfully"),
            Err(e) => warn!(vm_id = %id, error = ?e, "failed to create default eth0 NIC record"),
        }
    }

    // Auto-generate shell credentials for the VM
    let username = "root";
    let password = format!("vm-{}", &id.to_string()[..8]);
    if let Err(e) = st
        .shell_repo
        .upsert_credentials(id, username, &password)
        .await
    {
        warn!(vm_id = %id, error = ?e, "failed to create shell credentials for VM");
    } else {
        info!(vm_id = %id, username = %username, "created shell credentials for VM");
    }

    Ok(())
}

pub async fn restart_vm(st: &AppState, vm: &super::repo::VmRow) -> Result<()> {
    let host = st.hosts.get(vm.host_id).await?;
    let paths = VmPaths::from_row(vm);
    ensure_allowed_path(st, &vm.kernel_path)?;
    ensure_allowed_path(st, &vm.rootfs_path)?;
    let spec = ResolvedVmSpec {
        name: vm.name.clone(),
        vcpu: vm.vcpu.try_into().context("stored vcpu exceeds u8")?,
        mem_mib: vm.mem_mib.try_into().context("stored mem_mib negative")?,
        kernel_path: vm.kernel_path.clone(),
        rootfs_path: vm.rootfs_path.clone(),
        rootfs_size_bytes: None,
    };

    let network = select_network(&host.capabilities_json)?;

    // Create TAP devices for all NICs (including eth0 and additional NICs)
    create_all_tap_devices(st, &host.addr, vm.id, &network.bridge).await?;

    spawn_firecracker(st, &host.addr, vm.id, &paths).await?;
    configure_vm(st, &host.addr, vm.id, &spec, &paths).await?;
    start_vm(&host.addr, vm.id, &paths).await?;
    super::repo::update_state(&st.db, vm.id, "running").await?;

    // Spawn background task to configure secondary network interfaces via guest agent
    // This runs asynchronously so restart completes immediately
    let st_clone = st.clone();
    let vm_id = vm.id;
    tokio::spawn(async move {
        // Wait for guest agent to report IP (retry for up to 60 seconds)
        for attempt in 1..=12 {
            tokio::time::sleep(Duration::from_secs(5)).await;

            match super::repo::get(&st_clone.db, vm_id).await {
                Ok(vm) if vm.guest_ip.as_ref().is_some_and(|ip| !ip.is_empty()) => {
                    let guest_ip = vm.guest_ip.as_deref().unwrap_or("unknown");
                    info!(vm_id=%vm_id, attempt=%attempt, guest_ip=%guest_ip,
                          "guest IP detected, waiting for it to stabilize...");
                    let _ = audit::log_action(
                        &st_clone.db,
                        None,
                        "system",
                        AuditAction::SystemEvent,
                        Some("vm"),
                        Some(vm_id),
                        Some(json!({"event": "guest_ip_assigned", "ip": guest_ip})),
                        None,
                        true,
                        None,
                    )
                    .await;

                    // Wait a bit for IP to stabilize (DHCP might reassign)
                    tokio::time::sleep(Duration::from_secs(3)).await;

                    info!(vm_id=%vm_id, "configuring secondary NICs via guest agent");
                    if let Err(e) = configure_secondary_nics_via_guest_agent(&st_clone, vm_id).await
                    {
                        warn!(vm_id=%vm_id, error=?e, "failed to configure secondary NICs via guest agent");
                    }

                    // Apply port forwards now that guest IP is known
                    if let Err(e) = super::port_forwards::service::apply_forwards(&st_clone, vm_id).await {
                        warn!(vm_id=%vm_id, error=?e, "failed to apply port forwards");
                    }

                    return;
                }
                _ => {
                    if attempt % 3 == 0 {
                        info!(vm_id=%vm_id, attempt=%attempt, "waiting for guest agent to report IP...");
                    }
                }
            }
        }

        warn!(vm_id=%vm_id, "timeout waiting for guest IP, skipping secondary NIC configuration");
    });

    Ok(())
}

pub async fn stop_only(
    st: &AppState,
    id: Uuid,
    user_id: Option<Uuid>,
    username: &str,
) -> Result<()> {
    let vm = super::repo::get(&st.db, id).await?;
    super::repo::update_state(&st.db, id, "stopping").await?;

    // Clean up port forwards before stopping
    if let Err(e) = super::port_forwards::service::cleanup_forwards(st, id).await {
        tracing::warn!(vm_id=%id, error=?e, "failed to cleanup port forwards");
    }

    let response = reqwest::Client::new()
        .post(format!("{}/agent/v1/vms/{}/stop", vm.host_addr, vm.id))
        .json(&serde_json::json!({
            "tap": vm.tap,
            "sock": vm.api_sock,
            "fc_unit": vm.fc_unit,
            // Do NOT send storage_path - drives are persisted for restart
        }))
        .send()
        .await?;

    response.error_for_status()?;
    super::repo::update_state(&st.db, id, "stopped").await?;
    let _ = audit::log_action(
        &st.db,
        user_id,
        username,
        AuditAction::StopVm,
        Some("vm"),
        Some(id),
        None,
        None,
        true,
        None,
    )
    .await;
    Ok(())
}

pub async fn stop_and_delete(st: &AppState, id: Uuid) -> Result<()> {
    stop_and_delete_with_user(st, id, None, "system").await
}

pub async fn stop_and_delete_with_user(
    st: &AppState,
    id: Uuid,
    user_id: Option<Uuid>,
    username: &str,
) -> Result<()> {
    if let Err(err) = stop_only(st, id, None, "system").await {
        tracing::warn!(vm_id = %id, error = ?err, "failed to stop vm before deletion");
    }

    // Manually clean up storage directory (drives, logs, etc.)
    let storage_path = st.storage.vm_dir(id);
    if let Err(e) = tokio::fs::remove_dir_all(&storage_path).await {
        tracing::warn!(vm_id = %id, path = ?storage_path, error = ?e,
                      "failed to cleanup storage directory during deletion");
    } else {
        info!(vm_id = %id, path = ?storage_path, "cleaned up VM storage directory");
    }

    // Delete from database (this cascades to vm_drive and vm_network_interface)
    super::repo::delete_row(&st.db, id).await?;
    let _ = audit::log_action(
        &st.db,
        user_id,
        username,
        AuditAction::DeleteVm,
        Some("vm"),
        Some(id),
        None,
        None,
        true,
        None,
    )
    .await;
    Ok(())
}

pub async fn start_vm_by_id(st: &AppState, id: Uuid) -> Result<()> {
    start_vm_by_id_with_user(st, id, None, "system").await
}

pub async fn start_vm_by_id_with_user(
    st: &AppState,
    id: Uuid,
    user_id: Option<Uuid>,
    username: &str,
) -> Result<()> {
    let vm = super::repo::get(&st.db, id).await?;

    if vm.state == "running" {
        return Ok(()); // Already running
    }

    restart_vm(st, &vm).await?;
    let _ = audit::log_action(
        &st.db,
        user_id,
        username,
        AuditAction::StartVm,
        Some("vm"),
        Some(id),
        None,
        None,
        true,
        None,
    )
    .await;
    Ok(())
}

pub async fn pause_vm(
    st: &AppState,
    id: Uuid,
    user_id: Option<Uuid>,
    username: &str,
) -> Result<()> {
    let vm = super::repo::get(&st.db, id).await?;

    if vm.state != "running" {
        bail!("VM must be running to pause");
    }

    super::repo::update_state(&st.db, id, "pausing").await?;

    let base = format!("{}/agent/v1/vms/{}/proxy", vm.host_addr, vm.id);
    let qs = format!("?sock={}", urlencoding::encode(&vm.api_sock));

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .context("failed to build reqwest client (pause_vm)")?;

    let response = client
        .patch(format!("{base}/vm{qs}"))
        .json(&serde_json::json!({
            "state": "Paused"
        }))
        .send()
        .await?;

    response.error_for_status()?;
    super::repo::update_state(&st.db, id, "paused").await?;
    let _ = audit::log_action(
        &st.db,
        user_id,
        username,
        AuditAction::PauseVm,
        Some("vm"),
        Some(id),
        None,
        None,
        true,
        None,
    )
    .await;
    Ok(())
}

pub async fn resume_vm(
    st: &AppState,
    id: Uuid,
    user_id: Option<Uuid>,
    username: &str,
) -> Result<()> {
    let vm = super::repo::get(&st.db, id).await?;

    if vm.state != "paused" {
        bail!("VM must be paused to resume");
    }

    super::repo::update_state(&st.db, id, "resuming").await?;

    let base = format!("{}/agent/v1/vms/{}/proxy", vm.host_addr, vm.id);
    let qs = format!("?sock={}", urlencoding::encode(&vm.api_sock));

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .context("failed to build reqwest client (resume_vm)")?;

    let response = client
        .patch(format!("{base}/vm{qs}"))
        .json(&serde_json::json!({
            "state": "Resumed"
        }))
        .send()
        .await?;

    response.error_for_status()?;
    super::repo::update_state(&st.db, id, "running").await?;
    let _ = audit::log_action(
        &st.db,
        user_id,
        username,
        AuditAction::ResumeVm,
        Some("vm"),
        Some(id),
        None,
        None,
        true,
        None,
    )
    .await;
    Ok(())
}

pub async fn flush_vm_metrics(st: &AppState, id: Uuid) -> Result<()> {
    let vm = super::repo::get(&st.db, id).await?;
    let base = format!("{}/agent/v1/vms/{}/proxy", vm.host_addr, vm.id);
    let qs = format!("?sock={}", urlencoding::encode(&vm.api_sock));

    let response = reqwest::Client::new()
        .put(format!("{base}/actions{qs}"))
        .json(&serde_json::json!({
            "action_type": "FlushMetrics"
        }))
        .send()
        .await?;

    response.error_for_status()?;

    Ok(())
}

#[derive(serde::Deserialize)]
pub struct ProcessStats {
    #[allow(dead_code)]
    pub pid: u32,
    pub cpu_percent: f64,
    #[allow(dead_code)]
    pub memory_rss_kb: u64,
    pub memory_percent: f64,
}

#[derive(serde::Deserialize)]
struct GuestMetrics {
    cpu_usage_percent: f64,
    memory_usage_percent: f64,
    memory_used_kb: u64,
    #[allow(dead_code)]
    memory_total_kb: u64,
}

pub async fn get_process_stats(st: &AppState, id: Uuid) -> Result<ProcessStats> {
    let vm = super::repo::get(&st.db, id).await?;

    // Try to get metrics from guest agent first (if guest_ip is set)
    if let Some(guest_ip) = &vm.guest_ip {
        if let Ok(guest_metrics) = get_guest_metrics(guest_ip).await {
            // Convert guest metrics to ProcessStats format
            return Ok(ProcessStats {
                pid: 0, // Not applicable for guest metrics
                cpu_percent: guest_metrics.cpu_usage_percent,
                memory_rss_kb: guest_metrics.memory_used_kb,
                memory_percent: guest_metrics.memory_usage_percent,
            });
        }
        // If guest agent fails, fall through to host-side metrics
        tracing::debug!(vm_id = %id, guest_ip = %guest_ip, "Guest agent unavailable, falling back to host-side metrics");
    }

    // Fallback: Get host-side process stats via agent
    let url = format!(
        "{}/agent/v1/vms/{}/metrics/process-stats",
        vm.host_addr, vm.id
    );

    let response = reqwest::Client::new()
        .post(&url)
        .json(&serde_json::json!({
            "sock_path": vm.api_sock
        }))
        .send()
        .await?;

    let stats = response.error_for_status()?.json::<ProcessStats>().await?;

    Ok(stats)
}

async fn get_guest_metrics(guest_ip: &str) -> Result<GuestMetrics> {
    let url = format!("http://{}:9000/metrics", guest_ip);
    let response = reqwest::Client::new()
        .get(&url)
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await?;

    let metrics = response.error_for_status()?.json::<GuestMetrics>().await?;
    Ok(metrics)
}

pub async fn send_ctrl_alt_del(st: &AppState, id: Uuid) -> Result<()> {
    let vm = super::repo::get(&st.db, id).await?;

    if vm.state != "running" {
        bail!("VM must be running to send Ctrl-Alt-Del");
    }

    let base = format!("{}/agent/v1/vms/{}/proxy", vm.host_addr, vm.id);
    let qs = format!("?sock={}", urlencoding::encode(&vm.api_sock));

    let response = reqwest::Client::new()
        .put(format!("{base}/actions{qs}"))
        .json(&serde_json::json!({
            "action_type": "SendCtrlAltDel"
        }))
        .send()
        .await?;

    response.error_for_status()?;

    Ok(())
}

#[cfg_attr(test, allow(dead_code))]
struct VmPaths {
    sock: String,
    log_path: String,
    metrics_path: String,
    tap: String,
    fc_unit: String,
    snapshot_path: Option<String>,
    mem_path: Option<String>,
}

impl VmPaths {
    async fn new(id: Uuid, storage: &crate::features::storage::LocalStorage) -> Result<Self> {
        storage.ensure_vm_dirs(id).await?;
        Ok(Self {
            sock: storage.sock_path(id),
            log_path: storage.log_path(id),
            metrics_path: storage.metrics_path(id),
            tap: format!("tap-{}", &id.to_string()[..8]),
            fc_unit: format!("fc-{id}.scope"),
            snapshot_path: None,
            mem_path: None,
        })
    }

    fn from_row(vm: &super::repo::VmRow) -> Self {
        Self {
            sock: vm.api_sock.clone(),
            log_path: vm.log_path.clone(),
            metrics_path: format!("/srv/fc/vms/{}/logs/metrics.json", vm.id),
            tap: vm.tap.clone(),
            fc_unit: vm.fc_unit.clone(),
            snapshot_path: None,
            mem_path: None,
        }
    }

    fn with_snapshot(mut self, snapshot_path: String, mem_path: String) -> Self {
        self.snapshot_path = Some(snapshot_path);
        self.mem_path = Some(mem_path);
        self
    }
}

#[derive(Clone)]
struct ResolvedVmSpec {
    name: String,
    vcpu: u8,
    mem_mib: u32,
    kernel_path: String,
    rootfs_path: String,
    #[allow(dead_code)]
    rootfs_size_bytes: Option<u64>,
}

async fn resolve_vm_spec(st: &AppState, req: CreateVmReq, vm_id: Uuid) -> Result<ResolvedVmSpec> {
    let kernel_path =
        resolve_image_path(st, req.kernel_image_id, req.kernel_path, "kernel").await?;
    let (rootfs_path, rootfs_size_bytes) = provision_rootfs(
        st,
        req.rootfs_image_id,
        req.rootfs_path,
        vm_id,
        req.rootfs_size_mb,
    )
    .await?;

    Ok(ResolvedVmSpec {
        name: req.name,
        vcpu: req.vcpu,
        mem_mib: req.mem_mib,
        kernel_path,
        rootfs_path,
        rootfs_size_bytes,
    })
}

async fn resolve_image_path(
    st: &AppState,
    image_id: Option<Uuid>,
    direct_path: Option<String>,
    field: &str,
) -> Result<String> {
    if let Some(id) = image_id {
        let image = st
            .images
            .get(id)
            .await
            .with_context(|| format!("failed to load {field} image {id}"))?;
        ensure_allowed_path(st, &image.host_path)?;
        return Ok(image.host_path);
    }

    if let Some(path) = direct_path {
        if !st.allow_direct_image_paths {
            bail!("{field} path not permitted in production mode");
        }
        ensure_allowed_path(st, &path)?;
        return Ok(path);
    }

    Err(anyhow!("{field} requires an image id or host path"))
}

async fn provision_rootfs(
    st: &AppState,
    image_id: Option<Uuid>,
    direct_path: Option<String>,
    vm_id: Uuid,
    rootfs_size_mb: Option<u32>,
) -> Result<(String, Option<u64>)> {
    // Determine source path (from registry or direct)
    let source_path = if let Some(id) = image_id {
        let image = st
            .images
            .get(id)
            .await
            .with_context(|| format!("failed to load rootfs image {id}"))?;
        ensure_allowed_path(st, &image.host_path)?;
        image.host_path
    } else if let Some(path) = direct_path {
        if !st.allow_direct_image_paths {
            bail!("rootfs path not permitted in production mode");
        }
        ensure_allowed_path(st, &path)?;
        path
    } else {
        bail!("rootfs requires an image id or host path")
    };

    // Check if this is already a per-VM copy (from containers/functions feature)
    // These paths indicate the rootfs was already copied and should NOT be copied again:
    // - /srv/images/containers/{vm-id}.ext4
    // - /srv/images/functions/{vm-id}.ext4
    let is_already_vm_copy =
        source_path.contains("/containers/") || source_path.contains("/functions/");

    if is_already_vm_copy {
        // Already a per-VM copy from container/function feature, use it directly
        info!(vm_id = %vm_id, source = %source_path, "using pre-copied rootfs from container/function feature");
        return Ok((source_path, None));
    }

    // For regular VMs: ALWAYS copy the rootfs to VM-specific storage
    // This prevents VMs from sharing the same rootfs file and corrupting each other
    let (vm_root, size_bytes) = st
        .storage
        .alloc_rootfs(vm_id, Path::new(&source_path), rootfs_size_mb)
        .await
        .context("failed to provision rootfs")?;

    Ok((vm_root, Some(size_bytes)))
}

fn ensure_allowed_path(st: &AppState, path: &str) -> Result<()> {
    let candidate = Path::new(path);

    // Allow paths within the image root
    if st.images.is_path_allowed(candidate) {
        return Ok(());
    }

    // Also allow paths within the storage root (for auto-provisioned drives, rootfs, snapshots)
    let storage_base = std::env::var("MANAGER_STORAGE_ROOT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("/srv/fc/vms"));

    if candidate.starts_with(&storage_base) {
        return Ok(());
    }

    bail!("path {path} is not within the configured image root or storage root");
}

pub async fn list_drives(st: &AppState, vm_id: Uuid) -> Result<Vec<nexus_types::VmDrive>> {
    let rows = super::repo::drives::list(&st.db, vm_id).await?;
    Ok(rows.into_iter().map(Into::into).collect())
}

pub async fn create_drive(
    st: &AppState,
    vm_id: Uuid,
    req: CreateDriveReq,
) -> Result<nexus_types::VmDrive> {
    // Verify VM exists
    let _vm = super::repo::get(&st.db, vm_id).await?;

    // Determine path and size
    let (host_path, size_bytes) = if let Some(path) = req.path_on_host.as_ref() {
        // User-provided path
        ensure_allowed_path(st, path)?;
        (path.clone(), None)
    } else {
        // Auto-provision: create sparse disk file
        let size = req.size_bytes.unwrap_or(10_737_418_240); // Default 10GB
        let path = st.storage.alloc_data_disk(vm_id, size).await?;
        (path, Some(size as i64))
    };

    // Check for duplicate drive_id
    if super::repo::drives::list(&st.db, vm_id)
        .await?
        .iter()
        .any(|d| d.drive_id == req.drive_id)
    {
        bail!("drive_id already exists for this VM");
    }

    // Insert into database ONLY - drive will be applied on next VM start
    let drive = super::repo::drives::insert(
        &st.db,
        vm_id,
        &req.drive_id,
        &host_path,
        size_bytes,
        req.is_root_device,
        req.is_read_only,
        req.cache_type.as_deref(),
        req.io_engine.as_deref(),
        req.rate_limiter.as_ref(),
    )
    .await?;

    info!(vm_id = %vm_id, drive_id = %req.drive_id, path = %host_path,
          "Drive created in database, will be attached on next VM start");

    // Auto-register drive as a volume in the volume registry
    let vm = super::repo::get(&st.db, vm_id).await?;
    if let Err(e) =
        ensure_data_drive_registered(st, vm_id, &host_path, &req.drive_id, vm.host_id).await
    {
        warn!(vm_id = %vm_id, drive_id = %req.drive_id, error = ?e, "failed to auto-register data drive as volume");
    }

    Ok(drive.into())
}

pub async fn update_drive(
    st: &AppState,
    vm_id: Uuid,
    drive_id: Uuid,
    req: UpdateDriveReq,
) -> Result<nexus_types::VmDrive> {
    let drive = super::repo::drives::get(&st.db, drive_id).await?;
    if drive.vm_id != vm_id {
        bail!("drive does not belong to VM");
    }

    let new_path = req
        .path_on_host
        .unwrap_or_else(|| drive.path_on_host.clone());
    ensure_allowed_path(st, &new_path)?;

    let updated =
        super::repo::drives::update(&st.db, drive_id, &new_path, req.rate_limiter.as_ref()).await?;

    let vm = super::repo::get(&st.db, vm_id).await?;
    let base = format!("{}/agent/v1/vms/{}/proxy", vm.host_addr, vm.id);
    let qs = format!("?sock={}", urlencoding::encode(&vm.api_sock));

    reqwest::Client::new()
        .patch(format!("{base}/drives/{}{}", drive.drive_id, qs))
        .json(&serde_json::json!({
            "drive_id": drive.drive_id,
            "path_on_host": new_path,
            "rate_limiter": req.rate_limiter,
        }))
        .send()
        .await?
        .error_for_status()?;

    Ok(updated.into())
}

pub async fn delete_drive(st: &AppState, vm_id: Uuid, drive_id: Uuid) -> Result<()> {
    let drive = super::repo::drives::get(&st.db, drive_id).await?;
    if drive.vm_id != vm_id {
        bail!("drive does not belong to VM");
    }

    // Detach volume from volume registry if it exists
    use crate::features::volumes::repo::VolumeRepository;
    let volume_repo = VolumeRepository::new(st.db.clone());
    let vm = super::repo::get(&st.db, vm_id).await?;
    if let Ok(volumes) = volume_repo.list_by_host(vm.host_id).await {
        for volume in volumes {
            if volume.path == drive.path_on_host {
                // Detach the volume
                if let Err(e) = volume_repo.detach(volume.id, vm_id).await {
                    warn!(volume_id = %volume.id, error = ?e, "failed to detach volume during drive deletion");
                }
                info!(volume_id = %volume.id, "volume detached during drive deletion");
                break;
            }
        }
    }

    // Delete from database ONLY - drive removal will apply on next VM start
    super::repo::drives::delete(&st.db, drive_id).await?;

    // Optionally delete the disk file from filesystem if it's auto-provisioned
    if let Some(_size) = drive.size_bytes {
        if let Err(e) = tokio::fs::remove_file(&drive.path_on_host).await {
            warn!(path = %drive.path_on_host, error = ?e, "failed to delete drive file");
        } else {
            info!(path = %drive.path_on_host, "deleted drive file");
        }
    }

    info!(vm_id = %vm_id, drive_id = %drive.drive_id,
          "Drive deleted from database, will be removed from Firecracker on next VM start");

    Ok(())
}

pub async fn list_nics(st: &AppState, vm_id: Uuid) -> Result<Vec<nexus_types::VmNic>> {
    let rows = super::repo::nics::list(&st.db, vm_id).await?;
    Ok(rows.into_iter().map(Into::into).collect())
}

pub async fn create_nic(
    st: &AppState,
    vm_id: Uuid,
    req: CreateNicReq,
) -> Result<nexus_types::VmNic> {
    // Validate VM exists
    let _vm = super::repo::get(&st.db, vm_id).await?;

    // Get existing NICs to determine next interface ID
    let existing = super::repo::nics::list(&st.db, vm_id).await?;

    // Determine interface ID - either use provided one or auto-assign next sequential
    let iface_id = if let Some(provided_id) = req.iface_id {
        // Validate provided interface ID
        let iface_id = provided_id.trim().to_ascii_lowercase();
        if !iface_id.starts_with("eth") {
            bail!("interface id must start with eth");
        }
        if iface_id == "eth0" {
            bail!("eth0 is reserved for the primary interface");
        }
        if iface_id.len() <= 3 {
            bail!("interface id must include an index, e.g. eth1");
        }
        if !iface_id[3..].chars().all(|c| c.is_ascii_digit()) {
            bail!("interface id must be in the form eth<index>");
        }

        // Check for duplicate
        if existing
            .iter()
            .any(|nic| nic.iface_id.eq_ignore_ascii_case(&iface_id))
        {
            bail!("interface id already exists for this VM");
        }

        iface_id
    } else {
        // Auto-assign next sequential interface ID (eth1, eth2, eth3, ...)
        // Find the highest existing interface number
        let max_index = existing
            .iter()
            .filter_map(|nic| {
                // Parse interface ID like "eth1" -> 1, "eth2" -> 2, etc.
                nic.iface_id
                    .strip_prefix("eth")
                    .and_then(|num_str| num_str.parse::<u32>().ok())
            })
            .max()
            .unwrap_or(0); // If no NICs exist yet, start from 0 (next will be eth1)

        let next_index = max_index + 1;
        let auto_iface_id = format!("eth{}", next_index);

        info!(vm_id=%vm_id, iface_id=%auto_iface_id, "auto-assigned sequential interface ID");
        auto_iface_id
    };

    // Fetch network to get bridge_name and vlan_id
    use crate::features::networks::repo::NetworkRepository;
    let network_repo = NetworkRepository::new(st.db.clone());
    let network = network_repo
        .get(req.network_id)
        .await
        .map_err(|_| anyhow::anyhow!("Network not found"))?;

    // Auto-generate TAP device name: tap-{vm-4chars}-{num}
    // Linux interface names must be ≤15 chars, so we use shortened format
    // Examples: tap-ce77-1, tap-ce77-10, tap-ce77-222
    let vm_short_id = &vm_id.to_string()[..4]; // Use first 4 chars instead of 8

    // Extract numeric part from iface_id (e.g., "eth1" -> "1", "eth222" -> "222")
    let iface_num = iface_id.trim_start_matches("eth");
    let host_dev_name = format!("tap-{}-{}", vm_short_id, iface_num);

    // Check for duplicate TAP device names (shouldn't happen with our naming scheme, but be safe)
    if existing
        .iter()
        .any(|nic| nic.host_dev_name.eq_ignore_ascii_case(&host_dev_name))
    {
        bail!("host device already in use by another interface");
    }

    let guest_mac = req
        .guest_mac
        .as_ref()
        .map(|mac| mac.trim())
        .filter(|mac| !mac.is_empty());

    let rx_rate_limiter = req.rx_rate_limiter.as_ref().map(normalize_rate_limiter);
    let tx_rate_limiter = req.tx_rate_limiter.as_ref().map(normalize_rate_limiter);

    // Allocate static IP if network has CIDR configured
    let assigned_ip = if let Some(cidr) = &network.cidr {
        Some(allocate_ip_from_cidr(&st.db, req.network_id, cidr).await?)
    } else {
        None
    };

    // Insert network interface into database with network_id and assigned_ip
    // Interface will be attached to Firecracker on next VM start/restart
    let nic = super::repo::nics::insert(
        &st.db,
        vm_id,
        &iface_id,
        &host_dev_name,
        guest_mac,
        rx_rate_limiter.as_ref(),
        tx_rate_limiter.as_ref(),
        Some(req.network_id),
        assigned_ip.as_deref(),
    )
    .await?;

    info!(vm_id = %vm_id, iface_id = %iface_id, host_dev = %host_dev_name,
          network_id = %req.network_id, bridge = %network.bridge_name,
          "Network interface created in database, will be attached on next VM start");

    Ok(nic.into())
}

pub async fn update_nic(
    st: &AppState,
    vm_id: Uuid,
    nic_id: Uuid,
    req: UpdateNicReq,
) -> Result<nexus_types::VmNic> {
    let nic = super::repo::nics::get(&st.db, nic_id).await?;
    if nic.vm_id != vm_id {
        bail!("network interface does not belong to VM");
    }

    // Update database only - changes will apply on next VM start/restart
    let rx_rate_limiter = req.rx_rate_limiter.as_ref().map(normalize_rate_limiter);
    let tx_rate_limiter = req.tx_rate_limiter.as_ref().map(normalize_rate_limiter);

    let updated = super::repo::nics::update_rate_limiters(
        &st.db,
        nic_id,
        rx_rate_limiter.as_ref(),
        tx_rate_limiter.as_ref(),
    )
    .await?;

    info!(vm_id = %vm_id, iface_id = %nic.iface_id,
          "Network interface updated in database, will apply on next VM restart");

    Ok(updated.into())
}

pub async fn delete_nic(st: &AppState, vm_id: Uuid, nic_id: Uuid) -> Result<()> {
    let nic = super::repo::nics::get(&st.db, nic_id).await?;
    if nic.vm_id != vm_id {
        bail!("network interface does not belong to VM");
    }

    // Delete from database only - interface removal will apply on next VM start/restart
    super::repo::nics::delete(&st.db, nic_id).await?;

    info!(vm_id = %vm_id, iface_id = %nic.iface_id,
          "Network interface deleted from database, will be removed from Firecracker on next VM restart");

    Ok(())
}

pub async fn patch_machine_config(
    st: &AppState,
    vm_id: Uuid,
    req: MachineConfigPatchReq,
) -> Result<()> {
    let vm = super::repo::get(&st.db, vm_id).await?;
    let base = format!("{}/agent/v1/vms/{}/proxy", vm.host_addr, vm.id);
    let qs = format!("?sock={}", urlencoding::encode(&vm.api_sock));

    reqwest::Client::new()
        .patch(format!("{base}/machine-config{qs}"))
        .json(&req)
        .send()
        .await?
        .error_for_status()?;

    super::repo::update_state(&st.db, vm.id, &vm.state).await?;
    Ok(())
}

pub async fn put_cpu_config(st: &AppState, vm_id: Uuid, req: CpuConfigReq) -> Result<()> {
    let vm = super::repo::get(&st.db, vm_id).await?;
    let base = format!("{}/agent/v1/vms/{}/proxy", vm.host_addr, vm.id);
    let qs = format!("?sock={}", urlencoding::encode(&vm.api_sock));

    reqwest::Client::new()
        .put(format!("{base}/cpu-config{qs}"))
        .json(&req)
        .send()
        .await?
        .error_for_status()?;
    Ok(())
}

pub async fn put_vsock(st: &AppState, vm_id: Uuid, req: VsockConfigReq) -> Result<()> {
    let vm = super::repo::get(&st.db, vm_id).await?;
    let base = format!("{}/agent/v1/vms/{}/proxy", vm.host_addr, vm.id);
    let qs = format!("?sock={}", urlencoding::encode(&vm.api_sock));

    reqwest::Client::new()
        .put(format!("{base}/vsock{qs}"))
        .json(&req)
        .send()
        .await?
        .error_for_status()?;
    Ok(())
}

pub async fn put_mmds(st: &AppState, vm_id: Uuid, req: MmdsDataReq) -> Result<()> {
    let vm = super::repo::get(&st.db, vm_id).await?;
    let base = format!("{}/agent/v1/vms/{}/proxy", vm.host_addr, vm.id);
    let qs = format!("?sock={}", urlencoding::encode(&vm.api_sock));

    reqwest::Client::new()
        .put(format!("{base}/mmds{qs}"))
        .json(&req.data)
        .send()
        .await?
        .error_for_status()?;
    Ok(())
}

pub async fn put_mmds_config(st: &AppState, vm_id: Uuid, req: MmdsConfigReq) -> Result<()> {
    let vm = super::repo::get(&st.db, vm_id).await?;
    let base = format!("{}/agent/v1/vms/{}/proxy", vm.host_addr, vm.id);
    let qs = format!("?sock={}", urlencoding::encode(&vm.api_sock));

    reqwest::Client::new()
        .put(format!("{base}/mmds/config{qs}"))
        .json(&req)
        .send()
        .await?
        .error_for_status()?;
    Ok(())
}

pub async fn put_entropy(st: &AppState, vm_id: Uuid, req: EntropyConfigReq) -> Result<()> {
    let vm = super::repo::get(&st.db, vm_id).await?;
    let base = format!("{}/agent/v1/vms/{}/proxy", vm.host_addr, vm.id);
    let qs = format!("?sock={}", urlencoding::encode(&vm.api_sock));

    reqwest::Client::new()
        .put(format!("{base}/entropy{qs}"))
        .json(&req)
        .send()
        .await?
        .error_for_status()?;
    Ok(())
}

pub async fn put_serial(st: &AppState, vm_id: Uuid, req: SerialConfigReq) -> Result<()> {
    let vm = super::repo::get(&st.db, vm_id).await?;
    let base = format!("{}/agent/v1/vms/{}/proxy", vm.host_addr, vm.id);
    let qs = format!("?sock={}", urlencoding::encode(&vm.api_sock));

    reqwest::Client::new()
        .put(format!("{base}/serial{qs}"))
        .json(&req)
        .send()
        .await?
        .error_for_status()?;
    Ok(())
}

pub async fn patch_logger(st: &AppState, vm_id: Uuid, req: LoggerUpdateReq) -> Result<()> {
    let vm = super::repo::get(&st.db, vm_id).await?;
    let base = format!("{}/agent/v1/vms/{}/proxy", vm.host_addr, vm.id);
    let qs = format!("?sock={}", urlencoding::encode(&vm.api_sock));

    reqwest::Client::new()
        .put(format!("{base}/logger{qs}"))
        .json(&req)
        .send()
        .await?
        .error_for_status()?;
    Ok(())
}

pub async fn put_balloon(st: &AppState, vm_id: Uuid, req: BalloonConfig) -> Result<()> {
    let vm = super::repo::get(&st.db, vm_id).await?;
    let base = format!("{}/agent/v1/vms/{}/proxy", vm.host_addr, vm.id);
    let qs = format!("?sock={}", urlencoding::encode(&vm.api_sock));

    reqwest::Client::new()
        .put(format!("{base}/balloon{qs}"))
        .json(&req)
        .send()
        .await?
        .error_for_status()?;
    Ok(())
}

pub async fn patch_balloon(st: &AppState, vm_id: Uuid, req: BalloonConfig) -> Result<()> {
    let vm = super::repo::get(&st.db, vm_id).await?;
    let base = format!("{}/agent/v1/vms/{}/proxy", vm.host_addr, vm.id);
    let qs = format!("?sock={}", urlencoding::encode(&vm.api_sock));

    reqwest::Client::new()
        .patch(format!("{base}/balloon{qs}"))
        .json(&req)
        .send()
        .await?
        .error_for_status()?;
    Ok(())
}

pub async fn patch_balloon_stats(
    st: &AppState,
    vm_id: Uuid,
    req: BalloonStatsConfig,
) -> Result<()> {
    let vm = super::repo::get(&st.db, vm_id).await?;
    let base = format!("{}/agent/v1/vms/{}/proxy", vm.host_addr, vm.id);
    let qs = format!("?sock={}", urlencoding::encode(&vm.api_sock));

    reqwest::Client::new()
        .patch(format!("{base}/balloon/statistics{qs}"))
        .json(&req)
        .send()
        .await?
        .error_for_status()?;
    Ok(())
}

pub async fn load_snapshot(
    st: &AppState,
    vm_id: Uuid,
    snapshot: &crate::features::snapshots::repo::SnapshotRow,
) -> Result<()> {
    let vm = super::repo::get(&st.db, vm_id).await?;

    let client = reqwest::Client::new();
    let base = format!("{}/agent/v1/vms/{}", vm.host_addr, vm.id);
    let qs = format!("?sock={}", urlencoding::encode(&vm.api_sock));

    let is_diff = snapshot.snapshot_type == "Diff";
    let mem_value = if is_diff || snapshot.mem_path.is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::Value::String(snapshot.mem_path.clone())
    };

    let load_payload = serde_json::json!({
        "snapshot_path": snapshot.snapshot_path.clone(),
        "mem_file_path": mem_value,
        "enable_diff_snapshots": snapshot.track_dirty_pages,
    });

    let load_resp = client
        .put(format!("{base}/proxy/snapshot/load{qs}"))
        .json(&load_payload)
        .send()
        .await?;
    load_resp.error_for_status()?;

    if let Some(parent_id) = snapshot.parent_id {
        tracing::info!(vm_id = %vm.id, parent_id = %parent_id, "diff snapshot load uses parent");
    }

    Ok(())
}

/// Configure cloud-init credentials and network via MMDS after VM is configured
/// This injects cloud-init user-data with username/password AND network-config with DHCP
#[cfg(not(test))]
async fn configure_cloud_init_with_network(
    st: &AppState,
    vm_id: Uuid,
    username: &str,
    password: &str,
) -> Result<()> {
    use base64::{engine::general_purpose, Engine as _};

    // Generate cloud-init YAML with user credentials
    let cloud_init_yaml = format!(
        r#"#cloud-config
users:
  - name: {username}
    plain_text_passwd: {password}
    lock_passwd: false
    sudo: ALL=(ALL) NOPASSWD:ALL
chpasswd:
  expire: false
"#,
        username = username,
        password = password
    );

    // Fetch all NICs for this VM to generate network config for all interfaces
    let all_nics = super::repo::nics::list(&st.db, vm_id).await?;

    // Generate network-config YAML with DHCP for all interfaces
    let mut ethernets_config = String::new();
    for nic in &all_nics {
        ethernets_config.push_str(&format!(
            "  {}:\n    dhcp4: true\n    dhcp6: false\n",
            nic.iface_id
        ));
    }

    let network_config_yaml = format!("version: 2\nethernets:\n{}", ethernets_config);

    // Base64 encode both configs (cloud-init standard)
    let user_data_b64 = general_purpose::STANDARD.encode(cloud_init_yaml.as_bytes());
    let network_config_b64 = general_purpose::STANDARD.encode(network_config_yaml.as_bytes());

    info!(vm_id = %vm_id, username = %username, "configuring cloud-init with credentials and DHCP network");

    // Step 1: Configure MMDS for eth0 interface (required before injecting data)
    put_mmds_config(
        st,
        vm_id,
        MmdsConfigReq {
            version: Some("V2".to_string()),
            network_interfaces: Some(vec!["eth0".to_string()]),
            ipv4_address: None,
            imds_compat: None,
        },
    )
    .await
    .context("failed to configure MMDS")?;

    // Step 2: Inject cloud-init user-data and network-config into MMDS
    put_mmds(
        st,
        vm_id,
        MmdsDataReq {
            data: json!({
                "latest": {
                    "user-data": user_data_b64,
                    "network-config": network_config_b64
                }
            }),
        },
    )
    .await
    .context("failed to inject cloud-init data")?;

    info!(vm_id = %vm_id, "cloud-init configured with credentials and DHCP networking");
    Ok(())
}

#[cfg(test)]
async fn configure_cloud_init_with_network(_: &AppState, _: Uuid, _: &str, _: &str) -> Result<()> {
    Ok(())
}

/// Detect Linux distribution from mounted rootfs
/// Returns: alpine, ubuntu, debian, fedora, rhel, centos, arch, or unknown
#[cfg(not(test))]
#[allow(dead_code)] // Will be used in Phase 2 for distribution-aware network configuration
async fn detect_distro(mount_point: &str) -> Result<String> {
    use tokio::fs;

    let os_release_path = format!("{}/etc/os-release", mount_point);

    // Try reading /etc/os-release (systemd standard)
    if let Ok(contents) = fs::read_to_string(&os_release_path).await {
        // Parse ID= line to get distro name
        for line in contents.lines() {
            if let Some(id) = line.strip_prefix("ID=") {
                let distro = id.trim().trim_matches('"').to_lowercase();
                info!(mount_point = %mount_point, distro = %distro, "detected Linux distribution");
                return Ok(distro);
            }
        }
    }

    // Fallback: Check for distro-specific files
    if fs::metadata(format!("{}/etc/alpine-release", mount_point))
        .await
        .is_ok()
    {
        info!(mount_point = %mount_point, "detected Alpine Linux (via /etc/alpine-release)");
        return Ok("alpine".to_string());
    }

    if fs::metadata(format!("{}/etc/debian_version", mount_point))
        .await
        .is_ok()
    {
        info!(mount_point = %mount_point, "detected Debian/Ubuntu (via /etc/debian_version)");
        return Ok("debian".to_string());
    }

    if fs::metadata(format!("{}/etc/fedora-release", mount_point))
        .await
        .is_ok()
    {
        info!(mount_point = %mount_point, "detected Fedora (via /etc/fedora-release)");
        return Ok("fedora".to_string());
    }

    if fs::metadata(format!("{}/etc/redhat-release", mount_point))
        .await
        .is_ok()
    {
        info!(mount_point = %mount_point, "detected RHEL/CentOS (via /etc/redhat-release)");
        return Ok("rhel".to_string());
    }

    if fs::metadata(format!("{}/etc/arch-release", mount_point))
        .await
        .is_ok()
    {
        info!(mount_point = %mount_point, "detected Arch Linux (via /etc/arch-release)");
        return Ok("arch".to_string());
    }

    warn!(mount_point = %mount_point, "could not detect distribution, assuming unknown");
    Ok("unknown".to_string())
}

/// Fallback: Inject credentials directly into rootfs by mounting and modifying /etc/shadow
/// This is used when cloud-init is not available in the guest OS
#[cfg(not(test))]
async fn inject_credentials_to_rootfs(
    vm_id: Uuid,
    rootfs_path: &str,
    username: &str,
    password: &str,
) -> Result<()> {
    use std::path::PathBuf;
    use tokio::process::Command;

    info!(vm_id = %vm_id, rootfs = %rootfs_path, username = %username,
          "attempting rootfs credential injection (cloud-init fallback)");

    // Create temporary mount directory
    let mount_dir = format!("/tmp/nexus-mount-{}", vm_id);
    let mount_path = PathBuf::from(&mount_dir);

    // Cleanup function to ensure unmount even on error
    let cleanup = |mount_dir: String| async move {
        let _ = Command::new("sudo")
            .args(["umount", &mount_dir])
            .status()
            .await;
        let _ = tokio::fs::remove_dir(&mount_dir).await;
    };

    // Create mount directory
    tokio::fs::create_dir_all(&mount_path)
        .await
        .context("failed to create mount directory")?;

    // Mount the rootfs
    let mount_status = Command::new("sudo")
        .args(["mount", "-o", "loop", rootfs_path, &mount_dir])
        .status()
        .await
        .context("failed to execute mount command")?;

    if !mount_status.success() {
        cleanup(mount_dir.clone()).await;
        bail!("failed to mount rootfs at {}", rootfs_path);
    }

    // Generate password hash using openssl (SHA-512)
    // Use -stdin to avoid interactive prompts
    let mut child = Command::new("openssl")
        .args(["passwd", "-6", "-stdin"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .context("failed to spawn openssl")?;

    // Write password to stdin
    if let Some(mut stdin) = child.stdin.take() {
        use tokio::io::AsyncWriteExt;
        stdin
            .write_all(password.as_bytes())
            .await
            .context("failed to write password to openssl stdin")?;
        stdin
            .write_all(b"\n")
            .await
            .context("failed to write newline to openssl stdin")?;
        drop(stdin); // Close stdin to signal EOF
    }

    let hash_output = child
        .wait_with_output()
        .await
        .context("failed to wait for openssl")?;

    if !hash_output.status.success() {
        cleanup(mount_dir.clone()).await;
        bail!("openssl passwd failed");
    }

    let password_hash = String::from_utf8_lossy(&hash_output.stdout)
        .trim()
        .to_string();

    // Read current /etc/shadow using sudo (requires elevated permissions)
    let shadow_path = mount_path.join("etc/shadow");
    let shadow_read = Command::new("sudo")
        .arg("cat")
        .arg(&shadow_path)
        .output()
        .await
        .context("failed to read /etc/shadow from rootfs")?;

    if !shadow_read.status.success() {
        cleanup(mount_dir.clone()).await;
        bail!("failed to read /etc/shadow with sudo");
    }

    let shadow_contents = String::from_utf8_lossy(&shadow_read.stdout).to_string();

    // Update or add the user line in shadow file
    let mut new_shadow = String::new();
    let mut user_found = false;

    for line in shadow_contents.lines() {
        if line.starts_with(&format!("{}:", username)) {
            // Replace existing user's password
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 9 {
                new_shadow.push_str(&format!(
                    "{}:{}:{}:{}:{}:{}:{}:{}:{}\n",
                    username,
                    password_hash,
                    parts[2],
                    parts[3],
                    parts[4],
                    parts[5],
                    parts[6],
                    parts[7],
                    parts[8]
                ));
                user_found = true;
            } else {
                new_shadow.push_str(line);
                new_shadow.push('\n');
            }
        } else {
            new_shadow.push_str(line);
            new_shadow.push('\n');
        }
    }

    // If user not found, add new entry (for root: no expiry, no aging)
    if !user_found {
        new_shadow.push_str(&format!(
            "{}:{}:19000:0:99999:7:::\n",
            username, password_hash
        ));

        // Also create the user in /etc/passwd if it doesn't exist
        info!(vm_id = %vm_id, username = %username, "creating new user in /etc/passwd");

        let passwd_path = mount_path.join("etc/passwd");
        let passwd_read = Command::new("sudo")
            .arg("cat")
            .arg(&passwd_path)
            .output()
            .await
            .context("failed to read /etc/passwd")?;

        if passwd_read.status.success() {
            let passwd_contents = String::from_utf8_lossy(&passwd_read.stdout).to_string();

            // Check if user already exists in passwd
            let user_exists_in_passwd = passwd_contents
                .lines()
                .any(|line| line.starts_with(&format!("{}:", username)));

            if !user_exists_in_passwd {
                // Determine UID/GID (1000 for regular user, 0 for root)
                let (uid, gid) = if username == "root" {
                    (0, 0)
                } else {
                    (1000, 1000)
                };

                let home_dir = if username == "root" {
                    "/root"
                } else {
                    &format!("/home/{}", username)
                };

                // Add user to /etc/passwd
                let new_passwd_entry =
                    format!("{}:x:{}:{}::/{}:/bin/sh\n", username, uid, gid, home_dir);

                let mut passwd_write = Command::new("sudo")
                    .arg("tee")
                    .arg("-a")
                    .arg(&passwd_path)
                    .stdin(std::process::Stdio::piped())
                    .stdout(std::process::Stdio::null())
                    .spawn()
                    .context("failed to spawn tee for passwd")?;

                if let Some(mut stdin) = passwd_write.stdin.take() {
                    use tokio::io::AsyncWriteExt;
                    stdin
                        .write_all(new_passwd_entry.as_bytes())
                        .await
                        .context("failed to write passwd entry")?;
                    drop(stdin);
                }

                passwd_write
                    .wait()
                    .await
                    .context("failed to wait for passwd write")?;

                info!(vm_id = %vm_id, username = %username, uid = uid, "added user to /etc/passwd");

                // Create group entry if not root
                if username != "root" {
                    let group_path = mount_path.join("etc/group");
                    let group_entry = format!("{}:x:{}:\n", username, gid);

                    let mut group_write = Command::new("sudo")
                        .arg("tee")
                        .arg("-a")
                        .arg(&group_path)
                        .stdin(std::process::Stdio::piped())
                        .stdout(std::process::Stdio::null())
                        .spawn()
                        .context("failed to spawn tee for group")?;

                    if let Some(mut stdin) = group_write.stdin.take() {
                        use tokio::io::AsyncWriteExt;
                        stdin
                            .write_all(group_entry.as_bytes())
                            .await
                            .context("failed to write group entry")?;
                        drop(stdin);
                    }

                    group_write
                        .wait()
                        .await
                        .context("failed to wait for group write")?;

                    info!(vm_id = %vm_id, username = %username, gid = gid, "added group to /etc/group");
                }

                // Create home directory if not root
                if username != "root" {
                    let home_path = mount_path.join(format!("home/{}", username));
                    Command::new("sudo")
                        .arg("mkdir")
                        .arg("-p")
                        .arg(&home_path)
                        .status()
                        .await
                        .context("failed to create home directory")?;

                    Command::new("sudo")
                        .arg("chown")
                        .arg(format!("{}:{}", uid, gid))
                        .arg(&home_path)
                        .status()
                        .await
                        .context("failed to chown home directory")?;

                    info!(vm_id = %vm_id, username = %username, home = %home_path.display(), "created home directory");
                }
            }
        }
    }

    // Write updated shadow file using sudo via tee
    let mut write_child = Command::new("sudo")
        .arg("tee")
        .arg(&shadow_path)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .context("failed to spawn sudo tee")?;

    if let Some(mut stdin) = write_child.stdin.take() {
        use tokio::io::AsyncWriteExt;
        stdin
            .write_all(new_shadow.as_bytes())
            .await
            .context("failed to write to tee stdin")?;
        drop(stdin);
    }

    let write_output = write_child
        .wait_with_output()
        .await
        .context("failed to wait for tee")?;

    if !write_output.status.success() {
        cleanup(mount_dir.clone()).await;
        bail!("failed to write /etc/shadow with sudo tee");
    }

    // Set proper permissions on shadow file (0640)
    let chmod_status = Command::new("sudo")
        .arg("chmod")
        .arg("640")
        .arg(&shadow_path)
        .status()
        .await
        .context("failed to chmod /etc/shadow")?;

    if !chmod_status.success() {
        cleanup(mount_dir.clone()).await;
        bail!("failed to set permissions on /etc/shadow");
    }

    // Also inject network configuration for minimal images (Alpine, etc.)
    // This enables DHCP even if cloud-init is not available
    info!(vm_id = %vm_id, "injecting network DHCP configuration for minimal images");

    // Create /etc/network/interfaces for Alpine/BusyBox
    let interfaces_path = mount_path.join("etc/network/interfaces");
    let network_config = r#"auto lo
iface lo inet loopback

auto eth0
iface eth0 inet dhcp
    udhcpc_opts -b -s /etc/udhcpc/default.script
    hostname localhost
"#;

    let mut network_write = Command::new("sudo")
        .arg("tee")
        .arg(&interfaces_path)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .context("failed to spawn sudo tee for network config")?;

    if let Some(mut stdin) = network_write.stdin.take() {
        use tokio::io::AsyncWriteExt;
        stdin.write_all(network_config.as_bytes()).await?;
        drop(stdin);
    }

    let network_output = network_write.wait_with_output().await?;
    if !network_output.status.success() {
        warn!(vm_id = %vm_id, "failed to inject /etc/network/interfaces (may not be Alpine)");
    } else {
        info!(vm_id = %vm_id, "injected /etc/network/interfaces with DHCP config");
    }

    // Remove broken Firecracker tap script that interferes with DHCP
    let firecracker_tap_script = mount_path.join("etc/network/if-up.d/firecracker-tap");
    if firecracker_tap_script.exists() {
        let rm_status = Command::new("sudo")
            .arg("rm")
            .arg("-f")
            .arg(&firecracker_tap_script)
            .status()
            .await;

        match rm_status {
            Ok(status) if status.success() => {
                info!(vm_id = %vm_id, "removed broken firecracker-tap script");
            }
            _ => {
                warn!(vm_id = %vm_id, "could not remove firecracker-tap script (may not exist)");
            }
        }
    }

    // Inject a proper udhcpc default script for Alpine
    // The default script is broken/missing, causing DHCP to fail to configure the interface
    // First create the directory with sudo
    let udhcpc_dir = mount_path.join("etc/udhcpc");
    let mkdir_status = Command::new("sudo")
        .arg("mkdir")
        .arg("-p")
        .arg(&udhcpc_dir)
        .status()
        .await
        .context("failed to create /etc/udhcpc directory")?;

    if !mkdir_status.success() {
        warn!(vm_id = %vm_id, "failed to create /etc/udhcpc directory");
    }

    let udhcpc_script_path = mount_path.join("etc/udhcpc/default.script");
    let udhcpc_script = r#"#!/bin/sh
# udhcpc script for Alpine Linux (BusyBox udhcpc)

[ -z "$1" ] && echo "Error: should be called from udhcpc" && exit 1

case "$1" in
    deconfig)
        ip addr flush dev $interface
        ;;

    renew|bound)
        # BusyBox udhcpc provides: $ip, $subnet, $router, $dns
        # Configure IP address with subnet mask
        # Note: $subnet is in dotted-decimal notation (e.g., 255.255.255.0)
        # We need to convert it to CIDR or use ip addr with broadcast

        # Simple approach: just use /24 for most common networks
        # Better: Convert subnet to CIDR (for production, use proper conversion)
        CIDR=24  # Default to /24
        case "$subnet" in
            255.255.255.0) CIDR=24 ;;
            255.255.255.128) CIDR=25 ;;
            255.255.255.192) CIDR=26 ;;
            255.255.255.224) CIDR=27 ;;
            255.255.255.240) CIDR=28 ;;
            255.255.255.248) CIDR=29 ;;
            255.255.255.252) CIDR=30 ;;
            255.255.0.0) CIDR=16 ;;
            255.0.0.0) CIDR=8 ;;
        esac

        ip addr add $ip/$CIDR dev $interface

        # Configure default route if provided
        if [ -n "$router" ]; then
            ip route add default via $router dev $interface 2>/dev/null || true
        fi

        # Configure DNS if provided
        if [ -n "$dns" ]; then
            echo -n > /etc/resolv.conf
            for i in $dns; do
                echo "nameserver $i" >> /etc/resolv.conf
            done
        fi
        ;;
esac

exit 0
"#;

    let mut udhcpc_write = Command::new("sudo")
        .arg("tee")
        .arg(&udhcpc_script_path)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .context("failed to spawn sudo tee for udhcpc script")?;

    if let Some(mut stdin) = udhcpc_write.stdin.take() {
        use tokio::io::AsyncWriteExt;
        stdin.write_all(udhcpc_script.as_bytes()).await?;
        drop(stdin);
    }

    let udhcpc_output = udhcpc_write.wait_with_output().await?;
    if !udhcpc_output.status.success() {
        warn!(vm_id = %vm_id, "failed to inject udhcpc script");
    } else {
        // Make the script executable
        let chmod_udhcpc_status = Command::new("sudo")
            .arg("chmod")
            .arg("+x")
            .arg(&udhcpc_script_path)
            .status()
            .await
            .context("failed to chmod udhcpc script")?;

        if chmod_udhcpc_status.success() {
            info!(vm_id = %vm_id, "injected udhcpc default script for DHCP configuration");
        } else {
            warn!(vm_id = %vm_id, "failed to make udhcpc script executable");
        }
    }

    // Enable Alpine's built-in networking service by creating a symlink
    let runlevels_dir = mount_path.join("etc/runlevels");
    let networking_service = mount_path.join("etc/init.d/networking");

    if runlevels_dir.exists() && networking_service.exists() {
        info!(vm_id = %vm_id, "detected OpenRC (Alpine) - enabling built-in networking service");

        // Create symlink in default runlevel to enable the service at boot
        let default_runlevel = mount_path.join("etc/runlevels/default");
        if !default_runlevel.exists() {
            if let Err(e) = tokio::fs::create_dir_all(&default_runlevel).await {
                warn!(vm_id = %vm_id, "failed to create default runlevel: {}", e);
            }
        }

        let symlink_path = default_runlevel.join("networking");
        let ln_status = Command::new("sudo")
            .arg("ln")
            .arg("-sf")
            .arg("/etc/init.d/networking")
            .arg(&symlink_path)
            .status()
            .await
            .context("failed to create networking service symlink")?;

        if ln_status.success() {
            info!(vm_id = %vm_id, "enabled Alpine's built-in networking service for boot");
        } else {
            warn!(vm_id = %vm_id, "failed to enable networking service");
        }
    } else if runlevels_dir.exists() {
        warn!(vm_id = %vm_id, "OpenRC detected but /etc/init.d/networking not found - networking may not start automatically");
    }

    // Unmount and cleanup
    cleanup(mount_dir).await;

    info!(vm_id = %vm_id, "successfully injected credentials and network config into rootfs");
    Ok(())
}

#[cfg(test)]
async fn inject_credentials_to_rootfs(_: Uuid, _: &str, _: &str, _: &str) -> Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::hosts::repo::HostRepository;
    use crate::features::snapshots::repo::SnapshotRow;
    use crate::features::vms::repo;
    use nexus_types::CreateImageReq;
    use serde_json::json;
    #[derive(Clone)]
    pub struct TestSnapshotLoad {
        vm_id: Uuid,
        snapshot_path: String,
        mem_path: String,
    }

    static SNAPSHOT_LOAD_CALLS: std::sync::OnceLock<std::sync::Mutex<Vec<TestSnapshotLoad>>> =
        std::sync::OnceLock::new();

    fn snapshot_load_store() -> &'static std::sync::Mutex<Vec<TestSnapshotLoad>> {
        SNAPSHOT_LOAD_CALLS.get_or_init(|| std::sync::Mutex::new(Vec::new()))
    }

    pub fn reset_snapshot_load_calls() {
        snapshot_load_store().lock().unwrap().clear();
    }

    pub fn snapshot_load_calls() -> Vec<TestSnapshotLoad> {
        snapshot_load_store().lock().unwrap().clone()
    }

    #[ignore]
    #[sqlx::test(migrations = "./migrations")]
    async fn create_with_image_ids_resolves_paths(pool: sqlx::PgPool) {
        repo::reset_store();
        let hosts = HostRepository::new(pool.clone());
        let host = hosts
            .register("host", "http://127.0.0.1:1", json!({}))
            .await
            .unwrap();
        let images =
            crate::features::images::repo::ImageRepository::new(pool.clone(), "/srv/images");
        let kernel = images
            .insert(&CreateImageReq {
                kind: "kernel".into(),
                name: "vmlinux".into(),
                host_path: "/srv/images/vmlinux".into(),
                sha256: "abc".into(),
                size: 10,
                project: None,
            })
            .await
            .unwrap();
        let rootfs = images
            .insert(&CreateImageReq {
                kind: "rootfs".into(),
                name: "disk".into(),
                host_path: "/srv/images/rootfs".into(),
                sha256: "def".into(),
                size: 20,
                project: None,
            })
            .await
            .unwrap();

        let snapshots = crate::features::snapshots::repo::SnapshotRepository::new(pool.clone());
        let users = crate::features::users::repo::UserRepository::new(pool.clone());
        let shell_repo = crate::features::vms::shell::ShellRepository::new(pool.clone());
        let download_progress =
            std::sync::Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new()));
        let storage = crate::features::storage::LocalStorage::new();
        storage.init().await.unwrap();
        let state = AppState {
            db: pool.clone(),
            hosts: hosts.clone(),
            images: images.clone(),
            snapshots,
            users,
            shell_repo,
            allow_direct_image_paths: false,
            storage: storage.clone(),
            download_progress,
        };

        let vm_id = Uuid::new_v4();
        create_and_start(
            &state,
            vm_id,
            CreateVmReq {
                name: "vm".into(),
                vcpu: 1,
                mem_mib: 512,
                kernel_image_id: Some(kernel.id),
                rootfs_image_id: Some(rootfs.id),
                kernel_path: None,
                rootfs_path: None,
                source_snapshot_id: None,
                username: None,
                password: None,
                tags: vec![],
                rootfs_size_mb: None,
            },
            None,
            None,
            "test",
        )
        .await
        .unwrap();

        let stored = repo::get(&state.db, vm_id).await.unwrap();
        assert_eq!(stored.kernel_path, "/srv/images/vmlinux");
        assert_eq!(stored.rootfs_path, "/srv/images/rootfs");
        assert_eq!(stored.host_id, host.id);
    }

    #[ignore]
    #[sqlx::test(migrations = "./migrations")]
    async fn reject_direct_paths_in_prod(pool: sqlx::PgPool) {
        repo::reset_store();
        let hosts = HostRepository::new(pool.clone());
        hosts
            .register("host", "http://127.0.0.1:1", json!({}))
            .await
            .unwrap();
        let images =
            crate::features::images::repo::ImageRepository::new(pool.clone(), "/srv/images");
        let snapshots = crate::features::snapshots::repo::SnapshotRepository::new(pool.clone());
        let users = crate::features::users::repo::UserRepository::new(pool.clone());
        let shell_repo = crate::features::vms::shell::ShellRepository::new(pool.clone());
        let download_progress =
            std::sync::Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new()));
        let storage = crate::features::storage::LocalStorage::new();
        storage.init().await.unwrap();
        let state = AppState {
            db: pool,
            hosts,
            images,
            snapshots,
            users,
            shell_repo,
            allow_direct_image_paths: false,
            storage: storage.clone(),
            download_progress,
        };

        let err = create_and_start(
            &state,
            Uuid::new_v4(),
            CreateVmReq {
                name: "vm".into(),
                vcpu: 1,
                mem_mib: 512,
                kernel_image_id: None,
                rootfs_image_id: None,
                kernel_path: Some("/srv/images/vmlinux".into()),
                rootfs_path: Some("/srv/images/rootfs".into()),
                source_snapshot_id: None,
                username: None,
                password: None,
                tags: vec![],
                rootfs_size_mb: None,
            },
            None,
            None,
            "test",
        )
        .await
        .unwrap_err();

        assert!(err.to_string().contains("path not permitted"));
    }

    #[ignore]
    #[sqlx::test(migrations = "./migrations")]
    async fn restart_rejects_paths_outside_root(pool: sqlx::PgPool) {
        repo::reset_store();
        reset_snapshot_load_calls();
        let hosts = HostRepository::new(pool.clone());
        let host = hosts
            .register("host", "http://127.0.0.1:1", json!({}))
            .await
            .unwrap();
        let images =
            crate::features::images::repo::ImageRepository::new(pool.clone(), "/srv/images");
        let snapshots = crate::features::snapshots::repo::SnapshotRepository::new(pool.clone());
        let users = crate::features::users::repo::UserRepository::new(pool.clone());
        let shell_repo = crate::features::vms::shell::ShellRepository::new(pool.clone());
        let download_progress =
            std::sync::Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new()));
        let storage = crate::features::storage::LocalStorage::new();
        storage.init().await.unwrap();
        let state = AppState {
            db: pool,
            hosts,
            images,
            snapshots,
            users,
            shell_repo,
            allow_direct_image_paths: false,
            storage: storage.clone(),
            download_progress,
        };

        let vm = repo::VmRow {
            id: Uuid::new_v4(),
            name: "vm".into(),
            state: "stopped".into(),
            host_id: host.id,
            template_id: None,
            host_addr: host.addr,
            created_by_user_id: None,
            guest_ip: None,
            tags: vec![],
            api_sock: "/tmp/sock".into(),
            tap: "tap0".into(),
            log_path: "/tmp/log".into(),
            http_port: 0,
            fc_unit: "fc.scope".into(),
            vcpu: 1,
            mem_mib: 512,
            kernel_path: "/etc/passwd".into(),
            rootfs_path: "/srv/images/rootfs".into(),
            source_snapshot_id: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let err = restart_vm(&state, &vm).await.unwrap_err();
        assert!(err
            .to_string()
            .contains("not within the configured image root"));
    }

    #[ignore]
    #[sqlx::test(migrations = "./migrations")]
    async fn create_from_snapshot_persists_source(pool: sqlx::PgPool) {
        repo::reset_store();
        reset_snapshot_load_calls();

        let hosts = HostRepository::new(pool.clone());
        let host = hosts
            .register("host", "http://127.0.0.1:1", json!({"healthy": true}))
            .await
            .unwrap();
        let images =
            crate::features::images::repo::ImageRepository::new(pool.clone(), "/srv/images");
        let snapshots = crate::features::snapshots::repo::SnapshotRepository::new(pool.clone());
        let users = crate::features::users::repo::UserRepository::new(pool.clone());
        let shell_repo = crate::features::vms::shell::ShellRepository::new(pool.clone());
        let download_progress =
            std::sync::Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new()));
        let storage = crate::features::storage::LocalStorage::new();
        storage.init().await.unwrap();
        let state = AppState {
            db: pool.clone(),
            hosts: hosts.clone(),
            images: images.clone(),
            snapshots,
            users,
            shell_repo,
            allow_direct_image_paths: false,
            storage: storage.clone(),
            download_progress,
        };

        let now = chrono::Utc::now();
        let template_id = Some(Uuid::new_v4());
        let source_vm_id = Uuid::new_v4();
        let kernel_path = "/srv/images/kernel".to_string();
        let rootfs_path = "/srv/images/rootfs".to_string();
        let source_row = repo::VmRow {
            id: source_vm_id,
            name: "source".into(),
            state: "running".into(),
            host_id: host.id,
            template_id,
            host_addr: host.addr.clone(),
            api_sock: "/tmp/source.sock".into(),
            tap: "tap-source".into(),
            created_by_user_id: None,
            guest_ip: None,
            tags: vec![],
            log_path: "/tmp/source.log".into(),
            http_port: 0,
            fc_unit: "fc-source.scope".into(),
            vcpu: 2,
            mem_mib: 1024,
            kernel_path: kernel_path.clone(),
            rootfs_path: rootfs_path.clone(),
            source_snapshot_id: None,
            created_at: now,
            updated_at: now,
        };
        repo::insert(&state.db, &source_row).await.unwrap();

        let snapshot_id = Uuid::new_v4();
        let snapshot_row = SnapshotRow {
            id: snapshot_id,
            vm_id: source_vm_id,
            snapshot_path: "/srv/fc/vms/source/snapshots/snap.snapshot".into(),
            mem_path: "/srv/fc/vms/source/snapshots/snap.mem".into(),
            size_bytes: 0,
            state: "available".into(),
            snapshot_type: "Full".into(),
            parent_id: None,
            name: None,
            track_dirty_pages: false,
            created_at: now,
            updated_at: now,
        };
        let expected_snapshot_path = snapshot_row.snapshot_path.clone();
        let expected_mem_path = snapshot_row.mem_path.clone();

        let new_vm_id = Uuid::new_v4();
        super::create_from_snapshot(
            &state,
            new_vm_id,
            "clone".into(),
            None,
            snapshot_row.clone(),
            Some(source_row.clone()),
        )
        .await
        .unwrap();

        let stored = repo::get(&state.db, new_vm_id).await.unwrap();
        assert_eq!(stored.source_snapshot_id, Some(snapshot_id));
        assert_eq!(stored.kernel_path, kernel_path);
        assert_eq!(stored.rootfs_path, rootfs_path);
        assert_eq!(stored.template_id, template_id);

        let loads = snapshot_load_calls();
        assert_eq!(loads.len(), 1);
        assert_eq!(loads[0].vm_id, new_vm_id);
        assert_eq!(loads[0].snapshot_path, expected_snapshot_path);
        assert_eq!(loads[0].mem_path, expected_mem_path);
    }
}

/// Allocate next available IP from a CIDR range
/// Returns IP with CIDR notation (e.g., "10.9.0.5/24")
async fn allocate_ip_from_cidr(db: &PgPool, network_id: Uuid, cidr: &str) -> Result<String> {
    // Parse CIDR (e.g., "10.9.0.0/24")
    let parts: Vec<&str> = cidr.split('/').collect();
    if parts.len() != 2 {
        bail!("Invalid CIDR format: {}", cidr);
    }

    let network_addr = parts[0];
    let prefix_len = parts[1];

    // Parse network address octets
    let octets: Vec<&str> = network_addr.split('.').collect();
    if octets.len() != 4 {
        bail!("Invalid IP address in CIDR: {}", network_addr);
    }

    let base_octets: Result<Vec<u8>, _> = octets.iter().map(|o| o.parse()).collect();
    let base_octets = base_octets?;

    // Get all assigned IPs in this network
    let assigned_ips = sqlx::query_scalar::<_, String>(
        "SELECT assigned_ip FROM vm_network_interface WHERE network_id = $1 AND assigned_ip IS NOT NULL"
    )
    .bind(network_id)
    .fetch_all(db)
    .await?;

    // Extract just the IP part (without /XX) from assigned IPs
    let assigned_ips: Vec<String> = assigned_ips
        .iter()
        .filter_map(|ip| ip.split('/').next().map(|s| s.to_string()))
        .collect();

    // Try IPs starting from .2 (skip .0 for network, .1 for gateway)
    // For /24 networks, try up to .254 (skip .255 for broadcast)
    for last_octet in 2..=254 {
        let candidate = format!(
            "{}.{}.{}.{}",
            base_octets[0], base_octets[1], base_octets[2], last_octet
        );

        if !assigned_ips.contains(&candidate) {
            let ip_with_cidr = format!("{}/{}", candidate, prefix_len);
            info!(network_id=%network_id, cidr=%cidr, allocated_ip=%ip_with_cidr, "allocated new IP");
            return Ok(ip_with_cidr);
        }
    }

    bail!("No available IPs in network {}", cidr);
}

/// Helper function to detect connection errors that should trigger a retry
fn is_connection_error(e: &reqwest::Error) -> bool {
    e.is_timeout() || e.is_connect() || e.to_string().contains("No route to host")
}

/// Configure secondary network interfaces (eth1, eth2, etc.) via guest agent
/// This brings up the interfaces and configures DHCP or static IPs
async fn configure_secondary_nics_via_guest_agent(st: &AppState, vm_id: Uuid) -> Result<()> {
    // Fetch all NICs for this VM first
    let all_nics = super::repo::nics::list(&st.db, vm_id).await?;

    // Configure each secondary interface (skip eth0 as it's auto-configured)
    for nic in &all_nics {
        if nic.iface_id == "eth0" {
            continue; // Skip eth0 - already configured
        }

        // Build payload with static IP if assigned
        let mut payload = json!({
            "interface": nic.iface_id
        });

        if let Some(ref ip) = nic.assigned_ip {
            payload["static_ip"] = json!(ip);
        }

        // Retry loop with IP re-fetching on each attempt
        // This handles race condition where VM's guest IP changes during configuration
        let max_retries = 10;
        let mut configured = false;

        for retry in 0..max_retries {
            // Re-fetch VM to get the LATEST guest IP
            let vm = match super::repo::get(&st.db, vm_id).await {
                Ok(v) => v,
                Err(e) => {
                    warn!(vm_id=%vm_id, iface_id=%nic.iface_id, error=?e, "failed to fetch VM");
                    break;
                }
            };

            // Check if VM has guest IP
            let guest_ip = match vm.guest_ip.as_ref().filter(|ip| !ip.is_empty()) {
                Some(ip) => ip.clone(),
                None => {
                    warn!(vm_id=%vm_id, iface_id=%nic.iface_id, retry=%retry, "VM has no guest IP yet");
                    tokio::time::sleep(Duration::from_millis(500)).await;
                    continue;
                }
            };
            let guest_agent_url = format!("http://{}:9000", guest_ip);

            if retry == 0 {
                info!(vm_id=%vm_id, iface_id=%nic.iface_id, guest_ip=%guest_ip, assigned_ip=?nic.assigned_ip,
                      "configuring secondary interface via guest agent");
            } else {
                info!(vm_id=%vm_id, iface_id=%nic.iface_id, guest_ip=%guest_ip, retry=%retry,
                      "retrying interface configuration with updated guest IP");
            }

            let client = Client::builder()
                .timeout(Duration::from_secs(5))
                .build()
                .context("failed to build reqwest client")?;

            let response = client
                .post(format!("{}/configure-interface", guest_agent_url))
                .json(&payload)
                .send()
                .await;

            match response {
                Ok(resp) if resp.status().is_success() => {
                    info!(vm_id=%vm_id, iface_id=%nic.iface_id, retry=%retry, "successfully configured interface via guest agent");
                    configured = true;
                    break;
                }
                Ok(resp) => {
                    let status = resp.status();
                    let body = resp.text().await.unwrap_or_default();
                    warn!(vm_id=%vm_id, iface_id=%nic.iface_id, retry=%retry, status=?status, body=%body,
                          "guest agent returned error when configuring interface");
                    // Don't retry on HTTP errors (4xx, 5xx) - these won't be fixed by retrying
                    break;
                }
                Err(e) if is_connection_error(&e) => {
                    warn!(vm_id=%vm_id, iface_id=%nic.iface_id, retry=%retry, error=?e,
                          "connection error - will retry with fresh guest IP");

                    // Exponential backoff: 500ms, 1s, 2s, 4s, 8s...
                    let backoff = Duration::from_millis(500 * 2u64.pow(retry.min(4)));
                    tokio::time::sleep(backoff).await;
                }
                Err(e) => {
                    warn!(vm_id=%vm_id, iface_id=%nic.iface_id, retry=%retry, error=?e,
                          "unexpected error when configuring interface");
                    break;
                }
            }
        }

        if !configured {
            warn!(vm_id=%vm_id, iface_id=%nic.iface_id, "failed to configure interface after {} retries", max_retries);
        }
    }

    Ok(())
}

async fn create_all_tap_devices(
    st: &AppState,
    host_addr: &str,
    vm_id: Uuid,
    default_bridge: &str,
) -> Result<()> {
    use crate::features::networks::repo::NetworkRepository;

    let network_repo = NetworkRepository::new(st.db.clone());
    let all_nics = super::repo::nics::list(&st.db, vm_id).await?;

    for nic in &all_nics {
        let (bridge, vlan_id) = if let Some(network_id) = nic.network_id {
            // NIC has a network - fetch bridge and VLAN ID
            match network_repo.get(network_id).await {
                Ok(network) => (network.bridge_name, network.vlan_id.map(|v| v as u16)),
                Err(e) => {
                    warn!(vm_id=%vm_id, nic_id=%nic.id, network_id=%network_id, error=?e, "failed to fetch network for NIC, using default bridge");
                    (default_bridge.to_string(), None)
                }
            }
        } else {
            // Legacy NIC without network_id - use default bridge
            (default_bridge.to_string(), None)
        };

        create_tap_with_vlan(host_addr, vm_id, &nic.host_dev_name, &bridge, vlan_id).await?;
    }

    Ok(())
}

/// Create a single TAP device with optional VLAN support
async fn create_tap_with_vlan(
    host_addr: &str,
    id: Uuid,
    tap_name: &str,
    bridge: &str,
    vlan_id: Option<u16>,
) -> Result<()> {
    let http = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .context("failed to build reqwest client (create_tap_with_vlan)")?;

    info!(vm_id=%id, tap=%tap_name, %bridge, ?vlan_id, "creating TAP device on agent");

    let mut payload = json!({
        "bridge": bridge,
        "owner_user": Value::Null,
        "tap_name": tap_name  // Pass custom TAP device name
    });

    if let Some(vlan) = vlan_id {
        payload["vlan_id"] = json!(vlan);
    }

    http.post(format!("{host_addr}/agent/v1/vms/{id}/tap"))
        .json(&payload)
        .send()
        .await
        .context("create_tap_with_vlan request failed to send")?
        .error_for_status()
        .context("create_tap_with_vlan returned error status")?;

    info!(vm_id=%id, tap=%tap_name, "TAP device created successfully");
    Ok(())
}

#[cfg(not(test))]
async fn create_tap(host_addr: &str, id: Uuid, bridge: &str) -> Result<()> {
    let http = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .context("failed to build reqwest client (create_tap)")?;
    let tap = format!("tap-{}", &id.to_string()[..8]);
    info!(vm_id=%id, step="tap", %tap, "creating tap on agent");
    http.post(format!("{host_addr}/agent/v1/vms/{id}/tap"))
        .json(&json!({"bridge": bridge, "owner_user": Value::Null}))
        .send()
        .await
        .context("create_tap request failed to send")?
        .error_for_status()
        .context("create_tap returned error status")?;
    info!(vm_id=%id, step="tap", "ok");
    Ok(())
}

#[cfg(test)]
async fn create_tap(_: &str, _: Uuid, _: &str) -> Result<()> {
    Ok(())
}

#[cfg(not(test))]
async fn spawn_firecracker(
    _st: &AppState,
    host_addr: &str,
    id: Uuid,
    paths: &VmPaths,
) -> Result<()> {
    let http = Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .context("failed to build reqwest client (spawn)")?;

    info!(vm_id=%id, step="spawn", sock=%paths.sock, "requesting firecracker spawn on agent");
    // Fire-and-forget: do not block the creation flow on systemd-run latency
    match http
        .post(format!("{host_addr}/agent/v1/vms/{id}/spawn"))
        .json(&json!({
            "sock": paths.sock,
            "log_path": paths.log_path
        }))
        .send()
        .await
    {
        Ok(resp) => {
            if let Err(err) = resp.error_for_status_ref() {
                warn!(vm_id=%id, error=%err.to_string(), "spawn returned non-2xx; will poll socket");
            }
        }
        Err(err) => {
            warn!(vm_id=%id, error=%err.to_string(), "spawn request failed; will poll socket");
        }
    }

    // Poll agent inventory for the expected socket to become available
    let ready = poll_socket_ready(host_addr, id, &paths.sock, Duration::from_secs(45)).await?;
    if !ready {
        anyhow::bail!("spawn: socket not ready after timeout");
    }
    info!(vm_id=%id, step="spawn", "socket ready");
    Ok(())
}

#[derive(Deserialize)]
struct InvSocket {
    vm_id: String,
    sockets: Vec<String>,
}
#[derive(Deserialize)]
struct Inventory {
    sockets: Vec<InvSocket>,
}

#[cfg_attr(test, allow(dead_code))]
async fn poll_socket_ready(
    host_addr: &str,
    id: Uuid,
    expected_sock: &str,
    timeout: Duration,
) -> Result<bool> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
        .context("failed to build reqwest client (inventory)")?;
    let id_str = id.to_string();
    let start = Instant::now();
    while start.elapsed() < timeout {
        let resp = client
            .get(format!("{host_addr}/agent/v1/inventory"))
            .send()
            .await;
        if let Ok(ok) = resp {
            if let Ok(inv) = ok.error_for_status()?.json::<Inventory>().await {
                let found = inv
                    .sockets
                    .into_iter()
                    .any(|s| s.vm_id == id_str && s.sockets.iter().any(|p| p == expected_sock));
                if found {
                    return Ok(true);
                }
            }
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    Ok(false)
}

#[cfg(test)]
async fn spawn_firecracker(_: &AppState, _: &str, _: Uuid, _: &VmPaths) -> Result<()> {
    Ok(())
}

#[cfg(not(test))]
async fn configure_vm(
    st: &AppState,
    host_addr: &str,
    id: Uuid,
    spec: &ResolvedVmSpec,
    paths: &VmPaths,
) -> Result<()> {
    let base = format!("{host_addr}/agent/v1/vms/{id}/proxy");
    let qs = format!("?sock={}", urlencoding::encode(&paths.sock));
    let http = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .context("failed to build reqwest client")?;

    info!(vm_id=%id, step="machine-config", vcpu=%spec.vcpu, mem_mib=%spec.mem_mib, "configuring machine");
    http.put(format!("{base}/machine-config{qs}"))
        .json(&json!({
            "vcpu_count": spec.vcpu,
            "mem_size_mib": spec.mem_mib,
            "smt": false
        }))
        .send()
        .await
        .context("machine-config request failed to send")?
        .error_for_status()
        .context("machine-config returned error status")?;
    info!(vm_id=%id, step="machine-config", "ok");

    if paths.snapshot_path.is_none() {
        info!(vm_id=%id, step="boot-source", kernel_path=%spec.kernel_path, "configuring boot source");
        http.put(format!("{base}/boot-source{qs}"))
            .json(&json!({
                "kernel_image_path": spec.kernel_path,
                "boot_args": "console=ttyS0 reboot=k panic=1 pci=off init=/sbin/init",
            }))
            .send()
            .await
            .context("boot-source request failed to send")?
            .error_for_status()
            .context("boot-source returned error status")?;
        info!(vm_id=%id, step="boot-source", "ok");

        info!(vm_id=%id, step="drives", rootfs_path=%spec.rootfs_path, "attaching rootfs drive");
        http.put(format!("{base}/drives/rootfs{qs}"))
            .json(&json!({
                "drive_id": "rootfs",
                "path_on_host": spec.rootfs_path,
                "is_root_device": true,
                "is_read_only": false
            }))
            .send()
            .await
            .context("drives request failed to send")?
            .error_for_status()
            .context("drives returned error status")?;
        info!(vm_id=%id, step="drives", "ok");

        // Attach all additional drives from database
        let db_drives = super::repo::drives::list(&st.db, id).await?;
        for drive in &db_drives {
            // Validate drive path is allowed
            ensure_allowed_path(st, &drive.path_on_host)?;

            info!(vm_id=%id, drive_id=%drive.drive_id, path=%drive.path_on_host, "attaching additional drive from DB");

            // Build drive config - only include optional fields if they have values
            let mut drive_config = json!({
                "drive_id": drive.drive_id,
                "path_on_host": drive.path_on_host,
                "is_root_device": drive.is_root_device,
                "is_read_only": drive.is_read_only,
            });

            // Only add optional fields if they are Some
            if let Some(ref cache) = drive.cache_type {
                drive_config["cache_type"] = json!(cache);
            }
            if let Some(ref io) = drive.io_engine {
                drive_config["io_engine"] = json!(io);
            }
            if let Some(ref rl) = drive.rate_limiter {
                drive_config["rate_limiter"] = rl.clone();
            }

            http.put(format!("{base}/drives/{}{}", drive.drive_id, qs))
                .json(&drive_config)
                .send()
                .await
                .context("additional drive request failed to send")?
                .error_for_status()
                .context("additional drive returned error status")?;
        }
        if !db_drives.is_empty() {
            info!(vm_id=%id, count=%db_drives.len(), "attached drives from database");
        }
    }

    info!(vm_id=%id, step="network-interfaces", tap=%paths.tap, "configuring network interface");
    // Configure default eth0 interface with TAP device
    http.put(format!("{base}/network-interfaces/eth0{qs}"))
        .json(&json!({
            "iface_id": "eth0",
            "host_dev_name": paths.tap
        }))
        .send()
        .await
        .context("network-interfaces request failed to send")?
        .error_for_status()
        .context("network-interfaces returned error status")?;
    info!(vm_id=%id, step="network-interfaces", "ok");

    // Attach all additional network interfaces from database (skip eth0 as it's already configured)
    let db_nics = super::repo::nics::list(&st.db, id).await?;
    for nic in &db_nics {
        // Skip eth0 - it's the default interface already configured above
        if nic.iface_id == "eth0" {
            info!(vm_id=%id, iface_id=%nic.iface_id, "skipping eth0 (already configured as default interface)");
            continue;
        }

        info!(vm_id=%id, iface_id=%nic.iface_id, host_dev=%nic.host_dev_name, "attaching additional NIC from DB");

        // Build NIC config - only include optional fields if they have values
        let mut nic_config = json!({
            "iface_id": nic.iface_id,
            "host_dev_name": nic.host_dev_name,
        });

        // Only add optional fields if they are Some
        if let Some(ref mac) = nic.guest_mac {
            nic_config["guest_mac"] = json!(mac);
        }
        if let Some(ref rx) = nic.rx_rate_limiter {
            nic_config["rx_rate_limiter"] = normalize_rate_limiter(rx);
        }
        if let Some(ref tx) = nic.tx_rate_limiter {
            nic_config["tx_rate_limiter"] = normalize_rate_limiter(tx);
        }

        http.put(format!("{base}/network-interfaces/{}{}", nic.iface_id, qs))
            .json(&nic_config)
            .send()
            .await
            .context("additional NIC request failed to send")?
            .error_for_status()
            .context("additional NIC returned error status")?;
    }
    if !db_nics.is_empty() {
        info!(vm_id=%id, count=%db_nics.len(), "attached network interfaces from database");
    }

    info!(vm_id=%id, step="logger", log_path=%paths.log_path, "configuring logger");
    http.put(format!("{base}/logger{qs}"))
        .json(&json!({
            "log_path": paths.log_path,
            "level": "Info",
            "show_level": true,
            "show_log_origin": false
        }))
        .send()
        .await
        .context("logger request failed to send")?
        .error_for_status()
        .context("logger returned error status")?;
    info!(vm_id=%id, step="logger", "ok");

    // Configure serial console via /serial API endpoint (pre-boot only)
    // Note: Firecracker's serial console only supports OUTPUT (VM writes to a file/pipe)
    // For interactive terminal, we would need bidirectional communication which requires
    // a different approach (e.g., vsock or network-based terminal)
    // For now, we configure it for logging purposes only
    if paths.snapshot_path.is_none() {
        let console_log_path = st
            .storage
            .vm_dir(id)
            .join("logs/console.log")
            .display()
            .to_string();
        info!(vm_id=%id, step="serial", console_path=%console_log_path, "configuring serial console output");

        match http
            .put(format!("{base}/serial{qs}"))
            .json(&json!({
                "output_path": console_log_path
            }))
            .send()
            .await
        {
            Ok(resp) => match resp.error_for_status() {
                Ok(_) => {
                    info!(vm_id=%id, step="serial", "serial console configured for logging");
                }
                Err(e) => {
                    warn!(vm_id=%id, error=?e, "failed to configure serial console");
                }
            },
            Err(e) => {
                warn!(vm_id=%id, error=?e, "failed to send serial configuration request");
            }
        }
    }

    // Metrics are enabled by default; Firecracker expects a FIFO.
    let enable_metrics = std::env::var("MANAGER_DISABLE_METRICS")
        .map(|v| {
            let l = v.to_ascii_lowercase();
            !(l == "1" || l == "true" || l == "yes" || l == "on")
        })
        .unwrap_or(true); // Default to enabled
    if enable_metrics {
        // Ensure FIFO exists on the agent before configuring Firecracker metrics
        info!(vm_id=%id, step="metrics", metrics_path=%paths.metrics_path, "preparing metrics fifo");
        Client::new()
            .post(format!("{host_addr}/agent/v1/vms/{id}/metrics/prepare"))
            .json(&json!({
                "metrics_path": paths.metrics_path
            }))
            .send()
            .await
            .context("metrics prepare request failed to send")?
            .error_for_status()
            .context("metrics prepare returned error status")?;

        info!(vm_id=%id, step="metrics", metrics_path=%paths.metrics_path, "configuring metrics");
        http.put(format!("{base}/metrics{qs}"))
            .json(&json!({
                "metrics_path": paths.metrics_path,
                "level": "Info"
            }))
            .send()
            .await
            .context("metrics request failed to send")?
            .error_for_status()
            .context("metrics returned error status")?;
        info!(vm_id=%id, step="metrics", "ok");
    } else {
        info!(vm_id=%id, step="metrics", "skipped (MANAGER_ENABLE_METRICS not set)");
    }

    Ok(())
}

#[cfg(test)]
async fn configure_vm(
    _: &AppState,
    _: &str,
    _: Uuid,
    _: &ResolvedVmSpec,
    _: &VmPaths,
) -> Result<()> {
    Ok(())
}

#[cfg(not(test))]
async fn start_vm(host_addr: &str, id: Uuid, paths: &VmPaths) -> Result<()> {
    let base = format!("{host_addr}/agent/v1/vms/{id}/proxy");
    let qs = format!("?sock={}", urlencoding::encode(&paths.sock));
    Client::new()
        .put(format!("{base}/actions{qs}"))
        .json(&json!({"action_type": "InstanceStart"}))
        .send()
        .await?
        .error_for_status()?;
    Ok(())
}

#[cfg(test)]
async fn start_vm(_: &str, _: Uuid, _: &VmPaths) -> Result<()> {
    Ok(())
}

/// Auto-register network if it doesn't already exist
async fn ensure_network_registered(
    st: &AppState,
    bridge_name: &str,
    host_id: Uuid,
) -> Result<Uuid> {
    use crate::features::networks::repo::NetworkRepository;
    use tracing::info;

    let network_repo = NetworkRepository::new(st.db.clone());

    // Check if a bridge-type network (not VLAN) with this bridge already exists for this host
    let existing = network_repo.list_by_host(host_id).await?;
    info!(bridge = %bridge_name, host_id = %host_id, existing_count = existing.len(), "checking for existing networks");

    for network in existing {
        // Only match bridge-type networks (not VLANs) for eth0
        if network.bridge_name == bridge_name
            && network.type_ == "bridge"
            && network.vlan_id.is_none()
        {
            // Network already registered
            info!(bridge = %bridge_name, network_id = %network.id, network_type = %network.type_, "bridge network already registered, skipping creation");
            return Ok(network.id);
        }
    }

    // Create new network record
    let name = format!("{} Network", bridge_name);
    let description = Some("Auto-registered from VM creation");

    info!(bridge = %bridge_name, host_id = %host_id, name = %name, "creating new network record");

    let result = network_repo
        .create(
            &name,
            description,
            "bridge",
            None, // no VLAN ID for default bridge
            bridge_name,
            host_id,
            None, // CIDR will be determined by DHCP/router
            None, // Gateway will be determined by DHCP/router
        )
        .await?;

    info!(bridge = %bridge_name, network_id = %result.id, "network created successfully");

    Ok(result.id)
}

/// Auto-register data drive as a volume if it doesn't already exist
async fn ensure_data_drive_registered(
    st: &AppState,
    vm_id: Uuid,
    drive_path: &str,
    drive_id: &str,
    host_id: Uuid,
) -> Result<()> {
    use crate::features::volumes::repo::VolumeRepository;
    use std::fs;
    use tracing::info;

    let volume_repo = VolumeRepository::new(st.db.clone());

    // Check if a volume with this path already exists
    let existing = volume_repo.list_by_host(host_id).await?;
    info!(vm_id = %vm_id, drive_path = %drive_path, host_id = %host_id, "checking for existing data volume");

    for volume in existing {
        if volume.path == drive_path {
            // Volume already registered, attach it to this VM if not attached
            info!(vm_id = %vm_id, volume_id = %volume.id, status = %volume.status, "data volume already registered");
            if volume.status == "available" {
                info!(vm_id = %vm_id, volume_id = %volume.id, "attaching existing available data volume");
                let _ = volume_repo.attach(volume.id, vm_id, drive_id).await;
            }
            return Ok(());
        }
    }

    info!(vm_id = %vm_id, drive_path = %drive_path, "no existing data volume found, creating new volume record");

    // Get file size
    let size_bytes = fs::metadata(drive_path)
        .ok()
        .map(|m| m.len() as i64)
        .unwrap_or(0);

    // Determine volume type from extension
    let volume_type = if drive_path.ends_with(".ext4") {
        "ext4"
    } else if drive_path.ends_with(".qcow2") {
        "qcow2"
    } else {
        "raw"
    };

    // Get VM name to make volume name more descriptive
    let vm_name = match super::repo::get(&st.db, vm_id).await {
        Ok(vm) => vm.name,
        Err(_) => format!("vm-{}", &vm_id.to_string()[..8]),
    };

    // Extract filename from path
    let filename = std::path::Path::new(drive_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(drive_id);

    // Create descriptive name showing which VM and drive this belongs to
    let name = format!("{} ({} - {})", vm_name, drive_id, filename);

    // Create new volume record
    let description_text = format!("Data drive '{}' for VM: {}", drive_id, vm_name);
    let description = Some(description_text.as_str());

    let volume = volume_repo
        .create(
            &name,
            description,
            drive_path,
            size_bytes,
            volume_type,
            host_id,
        )
        .await?;

    info!(vm_id = %vm_id, volume_id = %volume.id, name = %name, size_bytes = %size_bytes, "data volume created successfully");

    // Attach the volume to the VM
    volume_repo.attach(volume.id, vm_id, drive_id).await?;

    info!(vm_id = %vm_id, volume_id = %volume.id, drive_id = %drive_id, "data volume attached to VM successfully");

    Ok(())
}

/// Auto-register rootfs volume if it doesn't already exist
async fn ensure_volume_registered(
    st: &AppState,
    vm_id: Uuid,
    rootfs_path: &str,
    host_id: Uuid,
) -> Result<()> {
    use crate::features::volumes::repo::VolumeRepository;
    use std::fs;
    use tracing::info;

    let volume_repo = VolumeRepository::new(st.db.clone());

    // Check if a volume with this path already exists
    let existing = volume_repo.list_by_host(host_id).await?;
    info!(vm_id = %vm_id, rootfs = %rootfs_path, host_id = %host_id, existing_count = existing.len(), "checking for existing volumes");

    for volume in existing {
        if volume.path == rootfs_path {
            // Volume already registered, attach it to this VM if not attached
            info!(vm_id = %vm_id, volume_id = %volume.id, status = %volume.status, "volume already registered");
            if volume.status == "available" {
                info!(vm_id = %vm_id, volume_id = %volume.id, "attaching existing available volume");
                let _ = volume_repo
                    .attach(
                        volume.id, vm_id, "rootfs", // drive_id
                    )
                    .await;
            }
            return Ok(());
        }
    }

    info!(vm_id = %vm_id, rootfs = %rootfs_path, "no existing volume found, creating new volume record");

    // Get file size
    let size_bytes = fs::metadata(rootfs_path)
        .ok()
        .map(|m| m.len() as i64)
        .unwrap_or(0);

    // Determine volume type from extension
    let volume_type = if rootfs_path.ends_with(".ext4") {
        "ext4"
    } else if rootfs_path.ends_with(".qcow2") {
        "qcow2"
    } else {
        "raw"
    };

    // Get VM name to make volume name more descriptive
    let vm_name = match super::repo::get(&st.db, vm_id).await {
        Ok(vm) => vm.name,
        Err(_) => format!("vm-{}", &vm_id.to_string()[..8]),
    };

    // Extract filename from path
    let filename = std::path::Path::new(rootfs_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("rootfs");

    // Create descriptive name showing which VM this belongs to
    let name = format!("{} ({})", vm_name, filename);

    // Create new volume record
    let description_text = format!("Rootfs for VM: {}", vm_name);
    let description = Some(description_text.as_str());

    let volume = volume_repo
        .create(
            &name,
            description,
            rootfs_path,
            size_bytes,
            volume_type,
            host_id,
        )
        .await?;

    info!(vm_id = %vm_id, volume_id = %volume.id, name = %name, size_gb = size_bytes / (1024 * 1024 * 1024), "volume created successfully");

    // Attach the volume to the VM
    volume_repo
        .attach(
            volume.id, vm_id, "rootfs", // drive_id
        )
        .await?;

    info!(vm_id = %vm_id, volume_id = %volume.id, "volume attached to VM successfully");

    Ok(())
}
