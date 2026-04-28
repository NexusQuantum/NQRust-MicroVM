use crate::features::storage_backends::repo::{StorageBackendRepository, StorageBackendRow};
use crate::AppState;
use axum::{extract::Path, http::StatusCode, response::IntoResponse, Extension, Json};
use nexus_types::{BackendKind, Capabilities, StorageBackend};
use uuid::Uuid;

fn row_to_wire(row: StorageBackendRow) -> Result<StorageBackend, StatusCode> {
    let kind: BackendKind = serde_json::from_value(serde_json::Value::String(row.kind.clone()))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let capabilities: Capabilities =
        serde_json::from_value(row.capabilities_json).unwrap_or_default();
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
