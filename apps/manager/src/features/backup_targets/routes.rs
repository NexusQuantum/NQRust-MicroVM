use crate::features::backup_targets::envelope;
use crate::features::backup_targets::repo::{
    BackupTargetRepository, BackupTargetRow, CreateParams,
};
use crate::AppState;
use axum::{extract::Path, http::StatusCode, response::IntoResponse, Extension, Json};
use nexus_types::{BackupTarget, CreateBackupTargetRequest};
use rand::RngCore;
use uuid::Uuid;

fn row_to_wire(row: BackupTargetRow) -> BackupTarget {
    BackupTarget {
        id: row.id,
        name: row.name,
        endpoint: row.endpoint,
        region: row.region,
        bucket: row.bucket,
        prefix: row.prefix,
        access_key_id: row.access_key_id,
        gc_hour: row.gc_hour as u8,
        created_at: row.created_at,
        deleted_at: row.deleted_at,
    }
}

#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct BackupTargetListResponse {
    pub items: Vec<BackupTarget>,
}

pub async fn list(Extension(st): Extension<AppState>) -> impl IntoResponse {
    let repo = BackupTargetRepository::new(st.db.clone());
    match repo.list_active().await {
        Ok(rows) => (
            StatusCode::OK,
            Json(BackupTargetListResponse {
                items: rows.into_iter().map(row_to_wire).collect(),
            }),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("backup_targets list: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "db"})),
            )
                .into_response()
        }
    }
}

pub async fn get_one(
    Extension(st): Extension<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let repo = BackupTargetRepository::new(st.db.clone());
    match repo.get(id).await {
        Ok(Some(row)) => (StatusCode::OK, Json(row_to_wire(row))).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error":"not found"})),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("backup_targets get: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error":"db"})),
            )
                .into_response()
        }
    }
}

pub async fn create(
    Extension(st): Extension<AppState>,
    Json(req): Json<CreateBackupTargetRequest>,
) -> impl IntoResponse {
    let repo = BackupTargetRepository::new(st.db.clone());

    let mut target_key = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut target_key);

    let enc_secret = match envelope::wrap(req.secret_access_key.as_bytes()) {
        Ok(b) => b,
        Err(e) => {
            tracing::error!("envelope wrap secret: {e:#}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error":"envelope"})),
            )
                .into_response();
        }
    };
    let enc_target = match envelope::wrap(&target_key) {
        Ok(b) => b,
        Err(e) => {
            tracing::error!("envelope wrap target_key: {e:#}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error":"envelope"})),
            )
                .into_response();
        }
    };

    match repo
        .create(CreateParams {
            name: &req.name,
            endpoint: &req.endpoint,
            region: req.region.as_deref(),
            bucket: &req.bucket,
            prefix: &req.prefix,
            access_key_id: &req.access_key_id,
            encrypted_secret_access_key: &enc_secret,
            encrypted_target_key: &enc_target,
            gc_hour: req.gc_hour as i16,
        })
        .await
    {
        Ok(row) => (StatusCode::CREATED, Json(row_to_wire(row))).into_response(),
        Err(sqlx::Error::Database(e)) if e.code().as_deref() == Some("23505") => (
            StatusCode::CONFLICT,
            Json(serde_json::json!({"error":"name already exists"})),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("backup_targets create: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error":"db"})),
            )
                .into_response()
        }
    }
}

pub async fn update(
    Extension(_st): Extension<AppState>,
    Path(_id): Path<Uuid>,
    Json(_req): Json<CreateBackupTargetRequest>,
) -> impl IntoResponse {
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(serde_json::json!({"error":"update not in v1"})),
    )
}

pub async fn soft_delete(
    Extension(st): Extension<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let repo = BackupTargetRepository::new(st.db.clone());
    match repo.count_backups_for_target(id).await {
        Ok(n) if n > 0 => (
            StatusCode::CONFLICT,
            Json(serde_json::json!({
                "error": format!("target has {n} backups; delete them first"),
            })),
        )
            .into_response(),
        Ok(_) => match repo.soft_delete(id).await {
            Ok(()) => (StatusCode::NO_CONTENT, ()).into_response(),
            Err(e) => {
                tracing::error!("backup_targets soft_delete: {e}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({"error":"db"})),
                )
                    .into_response()
            }
        },
        Err(e) => {
            tracing::error!("backup_targets count: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error":"db"})),
            )
                .into_response()
        }
    }
}

pub async fn trigger_gc(
    Extension(_st): Extension<AppState>,
    Path(_id): Path<Uuid>,
) -> impl IntoResponse {
    // Wired by Task B.T17 once the GC task exists.
    (
        StatusCode::ACCEPTED,
        Json(serde_json::json!({"queued": true})),
    )
}
