use crate::features::storage::config::{validate, RawBackendEntry};
use crate::features::storage_backends::repo::{StorageBackendRepository, StorageBackendRow};
use crate::AppState;
use axum::{extract::Path, http::StatusCode, response::IntoResponse, Extension, Json};
use nexus_types::{Capabilities, StorageBackend};
use serde::Deserialize;
use serde_json::Value as JsonValue;
use uuid::Uuid;

fn row_to_wire(row: StorageBackendRow) -> Result<StorageBackend, StatusCode> {
    let kind: nexus_types::BackendKind =
        serde_json::from_value(serde_json::Value::String(row.kind.clone()))
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let capabilities: Capabilities = match serde_json::from_value(row.capabilities_json) {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(
                "storage_backend '{}' has malformed capabilities_json; using default: {e}",
                row.name
            );
            Capabilities::default()
        }
    };
    Ok(StorageBackend {
        id: row.id,
        name: row.name,
        kind,
        capabilities,
        is_default: row.is_default,
        created_at: row.created_at,
        deleted_at: row.deleted_at,
    })
}

#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct StorageBackendListResponse {
    pub items: Vec<StorageBackend>,
}

#[utoipa::path(
    get,
    path = "/v1/storage_backends",
    responses(
        (status = 200, body = StorageBackendListResponse),
    ),
    tag = "StorageBackends",
)]
pub async fn list(Extension(st): Extension<AppState>) -> impl IntoResponse {
    let repo = StorageBackendRepository::new(st.db.clone());
    match repo.list_active().await {
        Ok(rows) => {
            let mut items = Vec::with_capacity(rows.len());
            for r in rows {
                match row_to_wire(r) {
                    Ok(w) => items.push(w),
                    Err(s) => {
                        return (s, Json(serde_json::json!({"error": "row deserialization"})))
                            .into_response()
                    }
                }
            }
            (StatusCode::OK, Json(StorageBackendListResponse { items })).into_response()
        }
        Err(e) => {
            tracing::error!("storage_backends list failed: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "db"})),
            )
                .into_response()
        }
    }
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CreateStorageBackendReq {
    pub name: String,
    pub kind: nexus_storage::BackendKind,
    #[serde(default)]
    pub is_default: bool,
    #[serde(default)]
    pub config: JsonValue,
}

#[utoipa::path(
    post,
    path = "/v1/storage_backends",
    request_body = CreateStorageBackendReq,
    responses(
        (status = 201, body = StorageBackend),
        (status = 400, description = "Validation failed"),
        (status = 409, description = "Backend with this name already exists"),
    ),
    tag = "StorageBackends",
)]
pub async fn create(
    Extension(st): Extension<AppState>,
    Json(req): Json<CreateStorageBackendReq>,
) -> impl IntoResponse {
    let validated = match validate(RawBackendEntry {
        name: req.name.clone(),
        kind: req.kind,
        is_default: req.is_default,
        config: req.config,
    }) {
        Ok(v) => v,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response();
        }
    };
    let capabilities_json = match serde_json::to_value(validated.capabilities) {
        Ok(v) => v,
        Err(e) => {
            tracing::error!("encode capabilities: {e}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "encode capabilities"})),
            )
                .into_response();
        }
    };
    let repo = StorageBackendRepository::new(st.db.clone());
    // Idempotent on (name). A second POST with the same name updates
    // the existing row in place — that's the behaviour operators
    // expect when iterating on a backend's config in the UI form.
    match repo
        .upsert(
            &validated.name,
            validated.kind.as_db_str(),
            &validated.config,
            &capabilities_json,
            validated.is_default,
        )
        .await
    {
        Ok(row) => match row_to_wire(row) {
            Ok(w) => (StatusCode::CREATED, Json(w)).into_response(),
            Err(s) => {
                (s, Json(serde_json::json!({"error": "row deserialization"}))).into_response()
            }
        },
        Err(e) => {
            tracing::error!("storage_backends create failed: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "db"})),
            )
                .into_response()
        }
    }
}

#[utoipa::path(
    delete,
    path = "/v1/storage_backends/{id}",
    params(("id" = Uuid, Path, description = "Storage backend ID")),
    responses(
        (status = 204, description = "Soft-deleted"),
        (status = 404),
        (status = 409, description = "Backend has live volumes"),
    ),
    tag = "StorageBackends",
)]
pub async fn delete(Extension(st): Extension<AppState>, Path(id): Path<Uuid>) -> impl IntoResponse {
    let repo = StorageBackendRepository::new(st.db.clone());
    let row = match repo.get(id).await {
        Ok(Some(r)) => r,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "not found"})),
            )
                .into_response();
        }
        Err(e) => {
            tracing::error!("storage_backends get for delete failed: {e}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "db"})),
            )
                .into_response();
        }
    };
    // Refuse delete if any active volume references this backend.
    // A foreign-key cascade isn't right here — soft-deleting a backend
    // with live volumes attached would orphan them.
    let live_count: Result<i64, _> = sqlx::query_scalar(
        r#"SELECT COUNT(*) FROM volume WHERE backend_id = $1 AND status != 'deleted'"#,
    )
    .bind(row.id)
    .fetch_one(&st.db)
    .await;
    if let Ok(n) = live_count {
        if n > 0 {
            return (
                StatusCode::CONFLICT,
                Json(serde_json::json!({
                    "error": format!("backend has {n} live volume(s); delete or migrate them before removing the backend"),
                })),
            )
                .into_response();
        }
    }
    if let Err(e) = repo.soft_delete_by_name(&row.name).await {
        tracing::error!("storage_backends soft-delete failed: {e}");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "db"})),
        )
            .into_response();
    }
    StatusCode::NO_CONTENT.into_response()
}

#[utoipa::path(
    get,
    path = "/v1/storage_backends/{id}",
    params(("id" = Uuid, Path, description = "Storage backend ID")),
    responses(
        (status = 200, body = StorageBackend),
        (status = 404),
    ),
    tag = "StorageBackends",
)]
pub async fn get_one(
    Extension(st): Extension<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let repo = StorageBackendRepository::new(st.db.clone());
    match repo.get(id).await {
        Ok(Some(row)) => match row_to_wire(row) {
            Ok(w) => (StatusCode::OK, Json(w)).into_response(),
            Err(s) => {
                (s, Json(serde_json::json!({"error": "row deserialization"}))).into_response()
            }
        },
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "not found"})),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("storage_backends get failed: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "db"})),
            )
                .into_response()
        }
    }
}

use crate::features::storage_backends::discovery::{
    discover_iscsi_targets, discover_nfs_exports, IscsiTarget, NfsExport,
};

#[derive(Debug, Deserialize)]
pub struct NfsScanQuery {
    pub server: String,
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct NfsScanResponse {
    pub exports: Vec<NfsExportWire>,
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct NfsExportWire {
    pub path: String,
    pub allowed: String,
}

impl From<NfsExport> for NfsExportWire {
    fn from(e: NfsExport) -> Self {
        Self {
            path: e.path,
            allowed: e.allowed,
        }
    }
}

#[utoipa::path(
    get,
    path = "/v1/storage_backends/scan/nfs",
    params(("server" = String, Query, description = "NFS server hostname or IP")),
    responses(
        (status = 200, body = NfsScanResponse),
        (status = 400, description = "server query param missing"),
        (status = 502, description = "Discovery failed (timeout, unreachable, command missing)"),
    ),
    tag = "StorageBackends",
)]
pub async fn scan_nfs(
    Extension(_st): Extension<AppState>,
    axum::extract::Query(q): axum::extract::Query<NfsScanQuery>,
) -> impl IntoResponse {
    if q.server.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "server query param is required"})),
        )
            .into_response();
    }
    match discover_nfs_exports(&q.server).await {
        Ok(exports) => (
            StatusCode::OK,
            Json(NfsScanResponse {
                exports: exports.into_iter().map(NfsExportWire::from).collect(),
            }),
        )
            .into_response(),
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            Json(serde_json::json!({"error": e})),
        )
            .into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub struct IscsiScanQuery {
    pub portal: String,
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct IscsiScanResponse {
    pub targets: Vec<IscsiTargetWire>,
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct IscsiTargetWire {
    pub portal: String,
    pub iqn: String,
}

impl From<IscsiTarget> for IscsiTargetWire {
    fn from(t: IscsiTarget) -> Self {
        Self {
            portal: t.portal,
            iqn: t.iqn,
        }
    }
}

#[utoipa::path(
    get,
    path = "/v1/storage_backends/scan/iscsi",
    params(("portal" = String, Query, description = "iSCSI portal as host:port")),
    responses(
        (status = 200, body = IscsiScanResponse),
        (status = 400),
        (status = 502),
    ),
    tag = "StorageBackends",
)]
pub async fn scan_iscsi(
    Extension(_st): Extension<AppState>,
    axum::extract::Query(q): axum::extract::Query<IscsiScanQuery>,
) -> impl IntoResponse {
    if q.portal.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "portal query param is required"})),
        )
            .into_response();
    }
    match discover_iscsi_targets(&q.portal).await {
        Ok(targets) => (
            StatusCode::OK,
            Json(IscsiScanResponse {
                targets: targets.into_iter().map(IscsiTargetWire::from).collect(),
            }),
        )
            .into_response(),
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            Json(serde_json::json!({"error": e})),
        )
            .into_response(),
    }
}
