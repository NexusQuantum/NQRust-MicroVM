//! HTTP routes that drive [`VmmDriver`] implementations on the agent.
//!
//! These are the routes the manager calls when it needs to boot, stop,
//! pause, resume, or rebind a VM via the new pluggable trait. For
//! Firecracker VMs the manager continues to use the legacy
//! `/agent/v1/vms/...` HTTP-over-UDS proxy because FC's REST API is its
//! native ABI; the trait-routed path is primarily for QEMU.

use std::path::PathBuf;

use axum::{
    extract::{ws::Message, ws::WebSocket, Path, Query, WebSocketUpgrade},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Extension, Json, Router,
};
use nexus_vmm::{
    BootMode, ShutdownMode, SnapshotKind, SnapshotMeta, SnapshotPaths, VmSpec, VmmHandle, VmmKind,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::AppState;

pub fn router() -> Router {
    Router::new()
        .route("/kinds", get(list_kinds))
        .route("/pci-devices", get(host_pci_devices))
        .route("/:id/boot", post(boot))
        .route("/:id/shutdown", post(shutdown))
        .route("/:id/pause", post(pause))
        .route("/:id/resume", post(resume))
        .route("/:id/destroy", post(destroy))
        .route("/:id/handle", get(get_handle))
        .route("/:id/rebind", post(rebind))
        .route("/:id/snapshot", post(snapshot))
        .route("/:id/cdrom/eject", post(cdrom_eject))
        .route("/:id/console/vnc/ws", get(vnc_ws_bridge))
        .route("/:id/disk/add", post(disk_add))
        .route("/:id/disk/remove", post(disk_remove))
        .route("/:id/disk/resize", post(disk_resize))
        .route("/:id/nic/add", post(nic_add))
        .route("/:id/nic/remove", post(nic_remove))
        .route("/:id/migrate/incoming", post(migrate_incoming))
        .route("/:id/migrate/outgoing", post(migrate_outgoing))
        .route("/:id/backup/disk", post(backup_disk))
        .route("/:id/metrics", get(qemu_metrics))
}

/// Query params for the QEMU host-side metrics endpoint.
#[derive(Deserialize)]
pub struct MetricsQuery {
    pub vmm_kind: VmmKind,
    /// vCPU count, so CPU% is expressed relative to the VM's allocation.
    #[serde(default)]
    pub vcpu: u32,
    /// Configured guest RAM (MiB), used as the memory denominator.
    #[serde(default)]
    pub mem_mib: u32,
}

fn read_cgroup_usage_usec(cpu_stat: &str) -> Option<u64> {
    for line in cpu_stat.lines() {
        if let Some(rest) = line.strip_prefix("usage_usec ") {
            return rest.trim().parse().ok();
        }
    }
    None
}

/// Host-side metrics for a QEMU VM, read from its systemd unit's cgroup
/// (`qemu-<id>.service`). QEMU guests have no in-VM guest-agent (especially
/// Windows), so CPU% and memory are observed from the host — exactly what
/// Proxmox shows. CPU% is sampled over a short window and normalised to the
/// VM's vCPU count; memory is the cgroup's current usage vs the configured RAM.
async fn qemu_metrics(
    Path(id): Path<Uuid>,
    Query(q): Query<MetricsQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    if q.vmm_kind != VmmKind::Qemu {
        return Err((
            StatusCode::BAD_REQUEST,
            "metrics endpoint is qemu-only".into(),
        ));
    }
    let base = format!("/sys/fs/cgroup/system.slice/qemu-{id}.service");

    // Memory: cgroup current bytes vs configured guest RAM.
    let mem_bytes: i64 = tokio::fs::read_to_string(format!("{base}/memory.current"))
        .await
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .ok_or((
            StatusCode::NOT_FOUND,
            "vm cgroup not found (not running on this host?)".to_string(),
        ))?;
    let mem_used_kb = mem_bytes / 1024;
    let mem_total_kb = (q.mem_mib as i64) * 1024;
    let mem_pct = if mem_total_kb > 0 {
        ((mem_used_kb as f64 / mem_total_kb as f64) * 100.0).min(100.0)
    } else {
        0.0
    };

    // CPU: sample cumulative usage_usec over a short window.
    let read_usage = || async {
        tokio::fs::read_to_string(format!("{base}/cpu.stat"))
            .await
            .ok()
            .and_then(|s| read_cgroup_usage_usec(&s))
    };
    let cpu_pct = match read_usage().await {
        Some(u1) => {
            let interval = std::time::Duration::from_millis(700);
            tokio::time::sleep(interval).await;
            match read_usage().await {
                Some(u2) => {
                    let vcpu = q.vcpu.max(1) as f64;
                    let interval_us = interval.as_micros() as f64;
                    let delta = u2.saturating_sub(u1) as f64;
                    ((delta / interval_us / vcpu) * 100.0).clamp(0.0, 100.0)
                }
                None => 0.0,
            }
        }
        None => 0.0,
    };

    Ok(Json(json!({
        "cpu_usage_percent": cpu_pct,
        "memory_usage_percent": mem_pct,
        "memory_used_kb": mem_used_kb,
        "memory_total_kb": mem_total_kb,
    })))
}

/// List which VMM kinds this agent has installed, with their version strings.
async fn list_kinds(Extension(st): Extension<AppState>) -> Json<serde_json::Value> {
    let mut entries = Vec::new();
    for k in st.vmm_registry.installed_kinds() {
        entries.push(json!({
            "kind": k.as_str(),
            "version": st.vmm_registry.version(k).unwrap_or("")
        }));
    }
    Json(json!({ "installed": entries }))
}

#[derive(Debug, Deserialize)]
pub struct BootRequest {
    pub vmm_kind: VmmKind,
    pub vcpu: u32,
    pub mem_mib: u32,
    pub boot: BootMode,
    #[serde(default)]
    pub disks: Vec<nexus_vmm::DiskSpec>,
    #[serde(default)]
    pub nics: Vec<nexus_vmm::NicSpec>,
    #[serde(default)]
    pub enable_vnc: bool,
    #[serde(default)]
    pub enable_tpm: bool,
    #[serde(default)]
    pub enable_balloon: bool,
    #[serde(default)]
    pub enable_rng: bool,
    #[serde(default)]
    pub no_reboot: bool,
    #[serde(default)]
    pub vsock_cid: Option<u32>,
    #[serde(default)]
    pub vfio_devices: Vec<String>,
    #[serde(default)]
    pub cpu_type: Option<String>,
    /// For target-side of live migration: spawn QEMU with `-incoming <uri>`.
    #[serde(default)]
    pub incoming_uri: Option<String>,
}

async fn boot(
    Extension(st): Extension<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<BootRequest>,
) -> Result<Json<VmmHandle>, (StatusCode, String)> {
    let driver = st.vmm_registry.get(req.vmm_kind).ok_or_else(|| {
        (
            StatusCode::PRECONDITION_FAILED,
            format!("vmm kind {} not installed on this host", req.vmm_kind),
        )
    })?;

    let run_dir = PathBuf::from(&st.run_dir);
    let spec = VmSpec {
        id,
        vcpu: req.vcpu,
        mem_mib: req.mem_mib,
        boot: req.boot,
        disks: req.disks,
        nics: req.nics,
        enable_vnc: req.enable_vnc,
        enable_tpm: req.enable_tpm,
        enable_balloon: req.enable_balloon,
        enable_rng: req.enable_rng,
        no_reboot: req.no_reboot,
        vsock_cid: req.vsock_cid,
        vfio_devices: req.vfio_devices,
        cpu_type: req.cpu_type,
        incoming_uri: req.incoming_uri,
        log_path: run_dir.join(id.to_string()).join("vmm.log"),
        run_dir,
    };

    let handle = driver
        .boot(&spec)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(handle))
}

#[derive(Debug, Deserialize)]
pub struct LifecycleRequest {
    pub vmm_kind: VmmKind,
    #[serde(default)]
    pub mode: Option<ShutdownMode>,
}

async fn shutdown(
    Extension(st): Extension<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<LifecycleRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let driver = st.vmm_registry.get(req.vmm_kind).ok_or_else(|| {
        (
            StatusCode::PRECONDITION_FAILED,
            "vmm kind not installed".into(),
        )
    })?;
    let run_dir = PathBuf::from(&st.run_dir);
    let mut handle = driver
        .rebind(&run_dir, id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "no live vmm for this vm".into()))?;
    handle.vm_id = id;
    driver
        .shutdown(&handle, req.mode.unwrap_or(ShutdownMode::Graceful))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(json!({"ok": true})))
}

async fn pause(
    Extension(st): Extension<AppState>,
    Path(id): Path<Uuid>,
    Query(req): Query<LifecycleQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let driver = st.vmm_registry.get(req.vmm_kind).ok_or_else(|| {
        (
            StatusCode::PRECONDITION_FAILED,
            "vmm kind not installed".into(),
        )
    })?;
    let run_dir = PathBuf::from(&st.run_dir);
    let handle = driver
        .rebind(&run_dir, id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "no live vmm".into()))?;
    driver
        .pause(&handle)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(json!({"ok": true})))
}

async fn resume(
    Extension(st): Extension<AppState>,
    Path(id): Path<Uuid>,
    Query(req): Query<LifecycleQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let driver = st.vmm_registry.get(req.vmm_kind).ok_or_else(|| {
        (
            StatusCode::PRECONDITION_FAILED,
            "vmm kind not installed".into(),
        )
    })?;
    let run_dir = PathBuf::from(&st.run_dir);
    let handle = driver
        .rebind(&run_dir, id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "no live vmm".into()))?;
    driver
        .resume(&handle)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(json!({"ok": true})))
}

async fn destroy(
    Extension(st): Extension<AppState>,
    Path(id): Path<Uuid>,
    Query(req): Query<LifecycleQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let driver = st.vmm_registry.get(req.vmm_kind).ok_or_else(|| {
        (
            StatusCode::PRECONDITION_FAILED,
            "vmm kind not installed".into(),
        )
    })?;
    let run_dir = PathBuf::from(&st.run_dir);
    if let Some(handle) = driver
        .rebind(&run_dir, id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    {
        driver
            .destroy(handle)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }
    Ok(Json(json!({"ok": true})))
}

#[derive(Debug, Deserialize)]
pub struct LifecycleQuery {
    pub vmm_kind: VmmKind,
}

async fn get_handle(
    Extension(st): Extension<AppState>,
    Path(id): Path<Uuid>,
    Query(req): Query<LifecycleQuery>,
) -> Result<Json<Option<VmmHandle>>, (StatusCode, String)> {
    let driver = st.vmm_registry.get(req.vmm_kind).ok_or_else(|| {
        (
            StatusCode::PRECONDITION_FAILED,
            "vmm kind not installed".into(),
        )
    })?;
    let run_dir = PathBuf::from(&st.run_dir);
    let h = driver
        .rebind(&run_dir, id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(h))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RebindResp {
    pub bound: bool,
    pub handle: Option<VmmHandle>,
}

async fn rebind(
    Extension(st): Extension<AppState>,
    Path(id): Path<Uuid>,
    Query(req): Query<LifecycleQuery>,
) -> Result<Json<RebindResp>, (StatusCode, String)> {
    let driver = st.vmm_registry.get(req.vmm_kind).ok_or_else(|| {
        (
            StatusCode::PRECONDITION_FAILED,
            "vmm kind not installed".into(),
        )
    })?;
    let run_dir = PathBuf::from(&st.run_dir);
    let handle = driver
        .rebind(&run_dir, id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(RebindResp {
        bound: handle.is_some(),
        handle,
    }))
}

#[derive(Debug, Deserialize)]
pub struct SnapshotRequest {
    pub vmm_kind: VmmKind,
    /// Filesystem path where the agent should write the state stream.
    pub state_path: PathBuf,
    #[serde(default)]
    pub kind: Option<SnapshotKind>,
}

/// Take a snapshot of the running VM. Driver-specific:
/// - QEMU: QMP `migrate "exec:cat > <path>"` (full state + memory).
/// - Firecracker: returns NotSupported because FC snapshots still go through
///   the legacy `/agent/v1/vms/:id/snapshot/...` proxy.
async fn snapshot(
    Extension(st): Extension<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<SnapshotRequest>,
) -> Result<Json<SnapshotMeta>, (StatusCode, String)> {
    let driver = st.vmm_registry.get(req.vmm_kind).ok_or_else(|| {
        (
            StatusCode::PRECONDITION_FAILED,
            "vmm kind not installed".into(),
        )
    })?;
    let run_dir = PathBuf::from(&st.run_dir);
    let handle = driver
        .rebind(&run_dir, id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "no live vmm".into()))?;
    let paths = SnapshotPaths {
        state_path: req.state_path,
        mem_path: None,
        diff_dir: None,
    };
    let meta = driver
        .snapshot(&handle, &paths, req.kind.unwrap_or(SnapshotKind::Full))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(meta))
}

#[derive(Debug, Deserialize)]
pub struct CdromEjectRequest {
    pub vmm_kind: VmmKind,
    /// QMP device id of the cdrom to eject (matches the DiskSpec.drive_id).
    pub drive_id: String,
}

/// QMP-driven CD-ROM eject. Used to detach the installer ISO once the guest
/// finishes its first install pass. Sends `device_del` for the virtio-blk
/// PCI device + `drive_del` for the backing drive.
async fn cdrom_eject(
    Extension(st): Extension<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<CdromEjectRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    if req.vmm_kind != VmmKind::Qemu {
        return Err((StatusCode::BAD_REQUEST, "cdrom eject is qemu-only".into()));
    }
    let driver = st.vmm_registry.get(req.vmm_kind).ok_or_else(|| {
        (
            StatusCode::PRECONDITION_FAILED,
            "vmm kind not installed".into(),
        )
    })?;
    let run_dir = PathBuf::from(&st.run_dir);
    let handle = driver
        .rebind(&run_dir, id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "no live vmm".into()))?;
    let mut qmp = crate::vmm::qmp::QmpClient::connect(&handle.api_sock)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    // Eject the medium from the CD-ROM drive. The installer ISO is an `ide-cd`
    // on an AHCI controller (see QemuDriver::build_args), so QMP `eject` removes
    // the disc and the guest boots the installed disk on its next reboot. We do
    // NOT `device_del` the device: CD-ROMs on the q35 root complex can't be
    // hot-unplugged, and keeping the (now empty) drive is the correct CD-eject
    // semantic. `force` overrides any guest media lock held by the installer.
    // The device id is the drive_id + "-dev" suffix per build_args.
    let dev_id = format!("{}-dev", req.drive_id);
    qmp.execute("eject", Some(json!({ "id": dev_id, "force": true })))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(json!({"ok": true, "drive_id": req.drive_id})))
}

/// WebSocket ↔ VNC UDS bridge. The browser's noVNC client speaks the RFB
/// protocol over WebSocket binary frames; QEMU exposes the same protocol
/// over the per-VM Unix domain socket. This handler is a transparent
/// bytes-in / bytes-out proxy between the two.
async fn vnc_ws_bridge(
    Extension(st): Extension<AppState>,
    Path(id): Path<Uuid>,
    Query(req): Query<LifecycleQuery>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    if req.vmm_kind != VmmKind::Qemu {
        return (StatusCode::BAD_REQUEST, "vnc is qemu-only").into_response();
    }
    let Some(driver) = st.vmm_registry.get(req.vmm_kind) else {
        return (StatusCode::PRECONDITION_FAILED, "vmm kind not installed").into_response();
    };
    let run_dir = PathBuf::from(&st.run_dir);
    let handle = match driver.rebind(&run_dir, id).await {
        Ok(Some(h)) => h,
        Ok(None) => return (StatusCode::NOT_FOUND, "no live vmm").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };
    let Some(vnc_listen) = handle.vnc.clone() else {
        return (StatusCode::BAD_REQUEST, "VM has no VNC enabled").into_response();
    };
    // We support "unix:/path/to/vnc.sock" only.
    let Some(sock_path) = vnc_listen.strip_prefix("unix:") else {
        return (
            StatusCode::BAD_REQUEST,
            "VNC listener is not a UDS — TCP VNC bridging not supported in 0.5.0",
        )
            .into_response();
    };
    let sock_path = PathBuf::from(sock_path);
    ws.on_upgrade(move |socket| vnc_proxy(socket, sock_path))
}

async fn vnc_proxy(ws: WebSocket, sock_path: PathBuf) {
    use futures::{SinkExt, StreamExt};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let uds = match tokio::net::UnixStream::connect(&sock_path).await {
        Ok(s) => s,
        Err(err) => {
            tracing::warn!(?err, sock=%sock_path.display(), "vnc proxy: connect failed");
            return;
        }
    };
    let (mut uds_r, mut uds_w) = uds.into_split();
    let (mut ws_sink, mut ws_stream) = ws.split();

    // ws → uds
    let ws_to_uds = async {
        while let Some(msg) = ws_stream.next().await {
            match msg {
                Ok(Message::Binary(b)) => {
                    if uds_w.write_all(&b).await.is_err() {
                        break;
                    }
                }
                Ok(Message::Text(t)) => {
                    if uds_w.write_all(t.as_bytes()).await.is_err() {
                        break;
                    }
                }
                Ok(Message::Close(_)) | Err(_) => break,
                Ok(_) => {}
            }
        }
        let _ = uds_w.shutdown().await;
    };

    // uds → ws
    let uds_to_ws = async {
        let mut buf = vec![0u8; 8192];
        loop {
            match uds_r.read(&mut buf).await {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    if ws_sink
                        .send(Message::Binary(buf[..n].to_vec()))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
            }
        }
        let _ = ws_sink.send(Message::Close(None)).await;
    };

    tokio::select! {
        _ = ws_to_uds => {}
        _ = uds_to_ws => {}
    }
}

// ============================================================================
// Hot-add / hot-remove device routes (QEMU only)
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct DiskAddRequest {
    pub vmm_kind: VmmKind,
    pub drive_id: String,
    pub source: PathBuf,
    #[serde(default)]
    pub read_only: bool,
    #[serde(default)]
    pub format: Option<String>,
    #[serde(default)]
    pub cdrom: bool,
}

async fn disk_add(
    Extension(st): Extension<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<DiskAddRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    if req.vmm_kind != VmmKind::Qemu {
        return Err((StatusCode::BAD_REQUEST, "hot-add is qemu-only".into()));
    }
    let handle = qmp_handle(&st, id).await?;
    let mut qmp = crate::vmm::qmp::QmpClient::connect(&handle.api_sock)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let fmt = req.format.as_deref().unwrap_or("raw");
    // blockdev-add creates a node QEMU can attach to a device.
    let node_args = serde_json::json!({
        "driver": fmt,
        "node-name": req.drive_id,
        "file": {
            "driver": "file",
            "filename": req.source.display().to_string(),
        },
        "read-only": req.read_only || req.cdrom,
    });
    qmp.execute("blockdev-add", Some(node_args))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    // device_add must land on a hot-pluggable PCIe root-port — the q35 root
    // complex (pcie.0, the default target) rejects hotplug. Try each
    // pre-allocated root-port (see QemuDriver::build_args) until one has a free
    // slot. On total failure, roll back the blockdev so we don't leak the node.
    let dev_id = format!("{}-dev", req.drive_id);
    let mut last_err = String::new();
    let mut attached = false;
    for n in 0..crate::vmm::qemu::HOTPLUG_ROOT_PORTS {
        let dev_args = serde_json::json!({
            "driver": "virtio-blk-pci",
            "drive": req.drive_id,
            "id": dev_id,
            "bus": format!("rphp{n}"),
        });
        match qmp.execute("device_add", Some(dev_args)).await {
            Ok(_) => {
                attached = true;
                break;
            }
            Err(e) => last_err = e.to_string(),
        }
    }
    if !attached {
        let _ = qmp
            .execute("blockdev-del", Some(json!({ "node-name": req.drive_id })))
            .await;
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("no free PCIe root-port for hot-add: {last_err}"),
        ));
    }
    Ok(Json(json!({"ok": true, "drive_id": req.drive_id})))
}

#[derive(Debug, Deserialize)]
pub struct DiskRemoveRequest {
    pub vmm_kind: VmmKind,
    pub drive_id: String,
}

async fn disk_remove(
    Extension(st): Extension<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<DiskRemoveRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    if req.vmm_kind != VmmKind::Qemu {
        return Err((StatusCode::BAD_REQUEST, "hot-remove is qemu-only".into()));
    }
    let handle = qmp_handle(&st, id).await?;
    let mut qmp = crate::vmm::qmp::QmpClient::connect(&handle.api_sock)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let dev_id = format!("{}-dev", req.drive_id);
    qmp.execute("device_del", Some(json!({ "id": dev_id })))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    // blockdev-del may fail until the guest releases the device; non-fatal.
    let _ = qmp
        .execute("blockdev-del", Some(json!({ "node-name": req.drive_id })))
        .await;
    Ok(Json(json!({"ok": true, "drive_id": req.drive_id})))
}

#[derive(Debug, Deserialize)]
pub struct DiskResizeRequest {
    pub vmm_kind: VmmKind,
    pub drive_id: String,
    pub source: PathBuf,
    /// New total size in bytes (grow-only).
    pub size_bytes: u64,
}

async fn disk_resize(
    Extension(st): Extension<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<DiskResizeRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    if req.vmm_kind != VmmKind::Qemu {
        return Err((StatusCode::BAD_REQUEST, "resize is qemu-only".into()));
    }
    // If the VM is running, grow it online via QMP (this also extends the
    // backing file). If it isn't, resize the image file directly.
    match qmp_handle(&st, id).await {
        Ok(handle) => {
            let mut qmp = crate::vmm::qmp::QmpClient::connect(&handle.api_sock)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            // Hot-added disks are referenced by their blockdev node-name (== drive_id);
            // boot-time -drive disks by the legacy "device" id. Try node-name first,
            // then fall back to device so both kinds resize online.
            let by_node = qmp
                .execute(
                    "block_resize",
                    Some(json!({ "node-name": req.drive_id, "size": req.size_bytes })),
                )
                .await;
            if let Err(e1) = by_node {
                qmp.execute(
                    "block_resize",
                    Some(json!({ "device": req.drive_id, "size": req.size_bytes })),
                )
                .await
                .map_err(|e2| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("block_resize (node-name: {e1}; device: {e2})"),
                    )
                })?;
            }
        }
        Err(_) => {
            let out = tokio::process::Command::new("qemu-img")
                .arg("resize")
                .arg(req.source.display().to_string())
                .arg(req.size_bytes.to_string())
                .output()
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("qemu-img: {e}")))?;
            if !out.status.success() {
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!(
                        "qemu-img resize failed: {}",
                        String::from_utf8_lossy(&out.stderr)
                    ),
                ));
            }
        }
    }
    Ok(Json(
        json!({"ok": true, "drive_id": req.drive_id, "size_bytes": req.size_bytes}),
    ))
}

/// List host PCI devices available for VFIO passthrough. Enumerates
/// `/sys/bus/pci/devices` (authoritative: BDF + IDs + bound driver) and enriches
/// each with a human-readable label + class name from `lspci` when available.
async fn host_pci_devices() -> Json<serde_json::Value> {
    use std::collections::HashMap;

    // Friendly names: bdf -> (class_name, label "Vendor Device").
    let mut names: HashMap<String, (String, String)> = HashMap::new();
    if let Ok(out) = tokio::process::Command::new("lspci")
        .args(["-Dmmnn"])
        .output()
        .await
    {
        if out.status.success() {
            for line in String::from_utf8_lossy(&out.stdout).lines() {
                let Some(bdf) = line.split_whitespace().next() else {
                    continue;
                };
                // Quoted fields: [_, class, _, vendor, _, device, ...]
                let q: Vec<&str> = line.split('"').collect();
                let class = q.get(1).copied().unwrap_or("").to_string();
                let vendor = q.get(3).copied().unwrap_or("");
                let device = q.get(5).copied().unwrap_or("");
                let label = format!("{vendor} {device}").trim().to_string();
                names.insert(bdf.to_string(), (class, label));
            }
        }
    }

    let mut devices = Vec::new();
    if let Ok(mut entries) = tokio::fs::read_dir("/sys/bus/pci/devices").await {
        while let Ok(Some(e)) = entries.next_entry().await {
            let bdf = e.file_name().to_string_lossy().to_string();
            let p = e.path();
            let read = |f: &str| {
                std::fs::read_to_string(p.join(f))
                    .map(|s| s.trim().to_string())
                    .unwrap_or_default()
            };
            let driver = std::fs::read_link(p.join("driver"))
                .ok()
                .and_then(|d| d.file_name().map(|n| n.to_string_lossy().to_string()))
                .unwrap_or_default();
            let (class_name, label) = names.get(&bdf).cloned().unwrap_or_default();
            devices.push(json!({
                "bdf": bdf,
                "vendor": read("vendor"),
                "device": read("device"),
                "class": read("class"),
                "class_name": class_name,
                "label": if label.is_empty() { read("device") } else { label },
                "driver": driver,
            }));
        }
    }
    devices.sort_by(|a, b| a["bdf"].as_str().cmp(&b["bdf"].as_str()));
    Json(json!({ "items": devices }))
}

#[derive(Debug, Deserialize)]
pub struct NicAddRequest {
    pub vmm_kind: VmmKind,
    pub iface_id: String,
    pub host_dev: String,
    pub mac: String,
}

async fn nic_add(
    Extension(st): Extension<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<NicAddRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    if req.vmm_kind != VmmKind::Qemu {
        return Err((StatusCode::BAD_REQUEST, "hot-add is qemu-only".into()));
    }
    let handle = qmp_handle(&st, id).await?;
    let mut qmp = crate::vmm::qmp::QmpClient::connect(&handle.api_sock)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let netdev_args = serde_json::json!({
        "type": "tap",
        "id": req.iface_id,
        "ifname": req.host_dev,
        "script": "no",
        "downscript": "no",
    });
    qmp.execute("netdev_add", Some(netdev_args))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let dev_args = serde_json::json!({
        "driver": "virtio-net-pci",
        "netdev": req.iface_id,
        "mac": req.mac,
        "id": format!("{}-dev", req.iface_id),
    });
    qmp.execute("device_add", Some(dev_args))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(json!({"ok": true, "iface_id": req.iface_id})))
}

#[derive(Debug, Deserialize)]
pub struct NicRemoveRequest {
    pub vmm_kind: VmmKind,
    pub iface_id: String,
}

async fn nic_remove(
    Extension(st): Extension<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<NicRemoveRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    if req.vmm_kind != VmmKind::Qemu {
        return Err((StatusCode::BAD_REQUEST, "hot-remove is qemu-only".into()));
    }
    let handle = qmp_handle(&st, id).await?;
    let mut qmp = crate::vmm::qmp::QmpClient::connect(&handle.api_sock)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let dev_id = format!("{}-dev", req.iface_id);
    qmp.execute("device_del", Some(json!({ "id": dev_id })))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let _ = qmp
        .execute("netdev_del", Some(json!({ "id": req.iface_id })))
        .await;
    Ok(Json(json!({"ok": true, "iface_id": req.iface_id})))
}

/// Helper: rebind to the live VmmHandle.
async fn qmp_handle(st: &AppState, id: Uuid) -> Result<nexus_vmm::VmmHandle, (StatusCode, String)> {
    let driver = st
        .vmm_registry
        .get(VmmKind::Qemu)
        .ok_or_else(|| (StatusCode::PRECONDITION_FAILED, "qemu not installed".into()))?;
    let run_dir = PathBuf::from(&st.run_dir);
    driver
        .rebind(&run_dir, id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "no live vmm".into()))
}

// ============================================================================
// Live migration routes (QEMU only)
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct MigrateIncomingRequest {
    pub vmm_kind: VmmKind,
    /// Listen on this TCP port for the inbound migration stream.
    pub listen_port: u16,
    /// Full VmSpec for the target QEMU. Mirrors BootRequest so the target
    /// can reconstruct the same machine config the source had.
    pub vcpu: u32,
    pub mem_mib: u32,
    pub boot: BootMode,
    #[serde(default)]
    pub disks: Vec<nexus_vmm::DiskSpec>,
    #[serde(default)]
    pub nics: Vec<nexus_vmm::NicSpec>,
    #[serde(default)]
    pub enable_vnc: bool,
    #[serde(default)]
    pub enable_tpm: bool,
    #[serde(default)]
    pub enable_balloon: bool,
    #[serde(default)]
    pub enable_rng: bool,
    #[serde(default)]
    pub vsock_cid: Option<u32>,
    #[serde(default)]
    pub vfio_devices: Vec<String>,
    #[serde(default)]
    pub cpu_type: Option<String>,
}

/// Configure this agent's QEMU to start in "incoming" migration mode. The
/// guest is paused until the source completes the migrate; once the
/// stream replays fully, QEMU transitions to running automatically.
async fn migrate_incoming(
    Extension(st): Extension<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<MigrateIncomingRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    if req.vmm_kind != VmmKind::Qemu {
        return Err((StatusCode::BAD_REQUEST, "migration is qemu-only".into()));
    }
    let driver = st
        .vmm_registry
        .get(req.vmm_kind)
        .ok_or_else(|| (StatusCode::PRECONDITION_FAILED, "qemu not installed".into()))?;
    let run_dir = PathBuf::from(&st.run_dir);
    let spec = VmSpec {
        id,
        vcpu: req.vcpu,
        mem_mib: req.mem_mib,
        boot: req.boot,
        disks: req.disks,
        nics: req.nics,
        enable_vnc: req.enable_vnc,
        enable_tpm: req.enable_tpm,
        enable_balloon: req.enable_balloon,
        enable_rng: req.enable_rng,
        no_reboot: false,
        vsock_cid: req.vsock_cid,
        vfio_devices: req.vfio_devices,
        cpu_type: req.cpu_type,
        incoming_uri: Some(format!("tcp:0.0.0.0:{}", req.listen_port)),
        log_path: run_dir.join(id.to_string()).join("vmm.log"),
        run_dir,
    };
    let handle = driver
        .boot(&spec)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(json!({
        "ok": true,
        "listen_port": req.listen_port,
        "api_sock": handle.api_sock,
    })))
}

#[derive(Debug, Deserialize)]
pub struct MigrateOutgoingRequest {
    /// `tcp:host:port` URI of the target's incoming-migration listener.
    pub target_uri: String,
}

/// Drive QMP `migrate` to send the running guest's state to a target host.
/// Polls `query-migrate` until completion or failure.
async fn migrate_outgoing(
    Extension(st): Extension<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<MigrateOutgoingRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let handle = qmp_handle(&st, id).await?;
    let mut qmp = crate::vmm::qmp::QmpClient::connect(&handle.api_sock)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    qmp.execute(
        "migrate",
        Some(serde_json::json!({ "uri": req.target_uri })),
    )
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    // Poll for completion. 10 minute hard cap.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(600);
    loop {
        let s: serde_json::Value = qmp
            .execute::<serde_json::Value>("query-migrate", None)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        match s.get("status").and_then(|v| v.as_str()) {
            Some("completed") => break,
            Some("failed") | Some("cancelled") => {
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("migrate {:?}", s),
                ));
            }
            _ => {}
        }
        if std::time::Instant::now() >= deadline {
            return Err((
                StatusCode::REQUEST_TIMEOUT,
                "migrate timed out after 10 minutes".into(),
            ));
        }
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }
    Ok(Json(json!({"ok": true})))
}

// ============================================================================
// QEMU backup primitive — qemu-img convert disk to a target path
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct BackupDiskRequest {
    pub vmm_kind: VmmKind,
    /// Source disk path on the agent host (typically the rootfs path).
    pub source: PathBuf,
    /// Destination path on the agent host for the qcow2 copy.
    pub destination: PathBuf,
    /// qcow2 / raw. Defaults to qcow2 for a compact backup.
    #[serde(default)]
    pub format: Option<String>,
    /// Pass `-c` to qemu-img for a compressed (smaller, slower) backup.
    #[serde(default)]
    pub compress: bool,
}

/// Snapshot a running QEMU VM's disk to a backup target file. Pauses the
/// guest briefly via QMP so the disk is consistent, runs `qemu-img
/// convert` to the destination path, then resumes. Destination can be on
/// any agent-visible filesystem (local, NFS, S3 mount). Restore is just
/// a normal VM create with the resulting qcow2 as the disk image.
async fn backup_disk(
    Extension(st): Extension<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<BackupDiskRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    if req.vmm_kind != VmmKind::Qemu {
        return Err((StatusCode::BAD_REQUEST, "backup is qemu-only".into()));
    }
    let handle = qmp_handle(&st, id).await?;
    let mut qmp = crate::vmm::qmp::QmpClient::connect(&handle.api_sock)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    qmp.execute::<serde_json::Value>("stop", None)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if let Some(p) = req.destination.parent() {
        let _ = tokio::fs::create_dir_all(p).await;
    }
    let fmt = req.format.as_deref().unwrap_or("qcow2");
    let mut cmd = tokio::process::Command::new("qemu-img");
    // `-U` skips qemu-img's shared-lock check: the live QEMU still holds the
    // qcow2's write lock even while the guest is QMP-`stop`ped (pausing vCPUs
    // doesn't close the file), so without this the convert fails with
    // "Failed to get shared write lock". The preceding `stop` quiesces the
    // guest, so reading the source unlocked is crash-consistent.
    cmd.arg("convert").arg("-U").arg("-O").arg(fmt);
    if req.compress {
        cmd.arg("-c");
    }
    cmd.arg(&req.source).arg(&req.destination);
    let out = cmd.output().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("spawn qemu-img: {e}"),
        )
    })?;
    // Always resume the guest, even if convert failed.
    let _ = qmp.execute::<serde_json::Value>("cont", None).await;
    if !out.status.success() {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!(
                "qemu-img convert failed: {}",
                String::from_utf8_lossy(&out.stderr)
            ),
        ));
    }
    let size = tokio::fs::metadata(&req.destination)
        .await
        .map(|m| m.len())
        .unwrap_or(0);
    Ok(Json(json!({
        "ok": true,
        "destination": req.destination,
        "size_bytes": size,
    })))
}
