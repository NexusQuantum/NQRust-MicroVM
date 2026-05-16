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
    pub vsock_cid: Option<u32>,
    #[serde(default)]
    pub vfio_devices: Vec<String>,
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
        vsock_cid: req.vsock_cid,
        vfio_devices: req.vfio_devices,
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
    // The device id in QEMU is the drive_id + "-dev" suffix per QemuDriver::build_args.
    let dev_id = format!("{}-dev", req.drive_id);
    qmp.execute("device_del", Some(json!({ "id": dev_id })))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    // drive_del may fail if QEMU has already collected the drive; non-fatal.
    let _ = qmp
        .execute("drive_del", Some(json!({ "id": req.drive_id })))
        .await;
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
