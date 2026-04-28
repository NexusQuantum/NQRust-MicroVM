use crate::features::storage::registry::HostBackendRegistry;
use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use nexus_storage::{AttachedPath, BackendKind, VolumeHandle};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Clone)]
pub struct StorageState {
    pub registry: HostBackendRegistry,
}

#[derive(Deserialize)]
pub struct AttachReq {
    pub volume: VolumeHandle,
}

#[derive(Serialize)]
pub struct AttachResp {
    pub attached: AttachedPath,
}

#[derive(Deserialize)]
pub struct DetachReq {
    pub volume: VolumeHandle,
    pub attached: AttachedPath,
}

#[derive(Deserialize)]
pub struct PopulateReq {
    pub backend_kind: BackendKind,
    pub attached: AttachedPath,
    pub source_path: PathBuf,
    pub target_size_bytes: u64,
}

pub async fn attach(
    State(s): State<Arc<StorageState>>,
    Json(req): Json<AttachReq>,
) -> impl IntoResponse {
    let backend = match s.registry.get(req.volume.backend_kind) {
        Some(b) => b,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "unsupported backend kind"})),
            )
                .into_response()
        }
    };
    match backend.attach(&req.volume).await {
        Ok(attached) => (StatusCode::OK, Json(AttachResp { attached })).into_response(),
        Err(e) => {
            tracing::error!("attach failed: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response()
        }
    }
}

pub async fn detach(
    State(s): State<Arc<StorageState>>,
    Json(req): Json<DetachReq>,
) -> impl IntoResponse {
    let backend = match s.registry.get(req.volume.backend_kind) {
        Some(b) => b,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "unsupported backend kind"})),
            )
                .into_response()
        }
    };
    match backend.detach(&req.volume, req.attached).await {
        Ok(()) => (StatusCode::OK, Json(serde_json::json!({}))).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

pub async fn populate(
    State(s): State<Arc<StorageState>>,
    Json(req): Json<PopulateReq>,
) -> impl IntoResponse {
    let backend = match s.registry.get(req.backend_kind) {
        Some(b) => b,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "unsupported backend kind"})),
            )
                .into_response()
        }
    };
    match backend
        .populate_streaming(&req.attached, &req.source_path, req.target_size_bytes)
        .await
    {
        Ok(()) => (StatusCode::OK, Json(serde_json::json!({}))).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

#[derive(Deserialize)]
pub struct Resize2fsReq {
    pub attached: AttachedPath,
}

pub async fn resize2fs(Json(req): Json<Resize2fsReq>) -> impl IntoResponse {
    let path = req.attached.path();
    let _ = tokio::process::Command::new("e2fsck")
        .args(["-f", "-y"])
        .arg(path)
        .output()
        .await
        .ok();
    let resize = tokio::process::Command::new("resize2fs")
        .arg(path)
        .output()
        .await;
    match resize {
        Ok(o) if o.status.success() => {
            (StatusCode::OK, Json(serde_json::json!({}))).into_response()
        }
        Ok(o) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"stderr": String::from_utf8_lossy(&o.stderr).to_string()})),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

pub async fn supported_kinds(State(s): State<Arc<StorageState>>) -> impl IntoResponse {
    let kinds: Vec<&'static str> = s
        .registry
        .supported_kinds()
        .iter()
        .map(|k| k.as_db_str())
        .collect();
    (StatusCode::OK, Json(serde_json::json!({"kinds": kinds}))).into_response()
}

pub fn router(state: Arc<StorageState>) -> Router {
    Router::new()
        .route("/attach", post(attach))
        .route("/detach", post(detach))
        .route("/populate", post(populate))
        .route("/resize2fs", post(resize2fs))
        .route("/supported_kinds", get(supported_kinds))
        .with_state(state)
}
