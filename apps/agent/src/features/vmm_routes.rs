//! HTTP routes that drive [`VmmDriver`] implementations on the agent.
//!
//! These are the routes the manager calls when it needs to boot, stop,
//! pause, resume, or rebind a VM via the new pluggable trait. For
//! Firecracker VMs the manager continues to use the legacy
//! `/agent/v1/vms/...` HTTP-over-UDS proxy because FC's REST API is its
//! native ABI; the trait-routed path is primarily for QEMU.

use std::path::PathBuf;

use axum::{
    extract::{Path, Query},
    http::StatusCode,
    routing::{get, post},
    Extension, Json, Router,
};
use nexus_vmm::{BootMode, ShutdownMode, VmSpec, VmmHandle, VmmKind};
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
