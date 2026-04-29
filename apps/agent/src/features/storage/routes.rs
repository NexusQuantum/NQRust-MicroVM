use crate::features::storage::backup::{run_backup, run_restore, BackupParams, RestoreParams};
use crate::features::storage::registry::HostBackendRegistry;
use crate::features::storage::s3::BackupTargetConfig as S3Config;
use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use nexus_backup::ChunkerParams as NexusChunkerParams;
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
    pub backend_kind: BackendKind,
    pub attached: AttachedPath,
}

pub async fn resize2fs(
    State(s): State<Arc<StorageState>>,
    Json(req): Json<Resize2fsReq>,
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
    match backend.resize2fs(&req.attached).await {
        Ok(()) => (StatusCode::OK, Json(serde_json::json!({}))).into_response(),
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

#[derive(Deserialize)]
pub struct BackupReq {
    pub backup_id: uuid::Uuid,
    pub snapshot: nexus_storage::VolumeSnapshotHandle,
    #[allow(dead_code)] // wire field; backend_kind is read from snapshot.backend_kind
    pub backend_kind: nexus_storage::BackendKind,
    pub target: BackupTargetWire,
    pub encryption_key: [u8; 32],
    pub chunker_params: ChunkerParamsWire,
}

#[derive(Deserialize)]
pub struct BackupTargetWire {
    pub endpoint: String,
    #[serde(default)]
    pub region: Option<String>,
    pub bucket: String,
    #[serde(default)]
    pub prefix: String,
    pub access_key_id: String,
    pub secret_access_key: String,
}

#[derive(Deserialize)]
pub struct ChunkerParamsWire {
    pub min_size: u32,
    pub avg_size: u32,
    pub max_size: u32,
}

#[derive(Serialize)]
pub struct BackupRespWire {
    pub manifest_object_key: String,
    pub chunk_count: u64,
    pub bytes_written: u64,
    pub bytes_unique: u64,
    pub duration_ms: u64,
}

pub async fn backup(
    State(s): State<Arc<StorageState>>,
    Json(req): Json<BackupReq>,
) -> impl IntoResponse {
    let target = S3Config {
        endpoint: req.target.endpoint,
        region: req.target.region,
        bucket: req.target.bucket,
        prefix: req.target.prefix,
        access_key_id: req.target.access_key_id,
        secret_access_key: req.target.secret_access_key,
    };
    let params = BackupParams {
        backup_id: req.backup_id,
        snapshot: req.snapshot,
        target,
        encryption_key: req.encryption_key,
        chunker_params: NexusChunkerParams {
            min_size: req.chunker_params.min_size,
            avg_size: req.chunker_params.avg_size,
            max_size: req.chunker_params.max_size,
        },
    };
    match run_backup(Arc::new(s.registry.clone()), params).await {
        Ok(o) => (
            StatusCode::OK,
            Json(BackupRespWire {
                manifest_object_key: o.manifest_object_key,
                chunk_count: o.chunk_count,
                bytes_written: o.bytes_written,
                bytes_unique: o.bytes_unique,
                duration_ms: o.duration_ms,
            }),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("agent backup failed: {e:#}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response()
        }
    }
}

#[derive(Deserialize)]
pub struct RestoreReq {
    pub target_volume: nexus_storage::VolumeHandle,
    pub target_attached: nexus_storage::AttachedPath,
    pub manifest_object_key: String,
    pub target: BackupTargetWire,
    pub encryption_key: [u8; 32],
}

#[derive(Serialize)]
pub struct RestoreRespWire {
    pub bytes_written: u64,
    pub duration_ms: u64,
}

pub async fn restore(
    State(_s): State<Arc<StorageState>>,
    Json(req): Json<RestoreReq>,
) -> impl IntoResponse {
    let target = S3Config {
        endpoint: req.target.endpoint,
        region: req.target.region,
        bucket: req.target.bucket,
        prefix: req.target.prefix,
        access_key_id: req.target.access_key_id,
        secret_access_key: req.target.secret_access_key,
    };
    let params = RestoreParams {
        target_volume: req.target_volume,
        target_attached: req.target_attached,
        manifest_object_key: req.manifest_object_key,
        target,
        encryption_key: req.encryption_key,
    };
    match run_restore(params).await {
        Ok(o) => (
            StatusCode::OK,
            Json(RestoreRespWire {
                bytes_written: o.bytes_written,
                duration_ms: o.duration_ms,
            }),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("agent restore failed: {e:#}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response()
        }
    }
}

pub fn router(state: Arc<StorageState>) -> Router {
    Router::new()
        .route("/attach", post(attach))
        .route("/detach", post(detach))
        .route("/populate", post(populate))
        .route("/resize2fs", post(resize2fs))
        .route("/supported_kinds", get(supported_kinds))
        .route("/backup", post(backup))
        .route("/restore", post(restore))
        .with_state(state)
}
