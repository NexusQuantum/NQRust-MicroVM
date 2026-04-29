use crate::features::backups::repo::{BackupRepository, BackupRow};
use crate::features::backups::service;
use crate::AppState;
use axum::{
    extract::{Path, Query},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Extension, Json, Router,
};
use nexus_types::{Backup, BackupStatus, CreateBackupRequest, RestoreRequest};
use serde::Deserialize;
use uuid::Uuid;

fn row_to_wire(row: BackupRow) -> Backup {
    Backup {
        id: row.id,
        source_volume_id: row.source_volume_id,
        target_id: row.target_id,
        size_bytes: row.size_bytes,
        unique_bytes: row.unique_bytes,
        chunk_count: row.chunk_count,
        status: match row.status.as_str() {
            "running" => BackupStatus::Running,
            "completed" => BackupStatus::Completed,
            "failed" => BackupStatus::Failed,
            "pruning" => BackupStatus::Pruning,
            _ => BackupStatus::Failed,
        },
        error_message: row.error_message,
        created_at: row.created_at,
        completed_at: row.completed_at,
    }
}

#[derive(Deserialize)]
pub struct ListQuery {
    pub volume_id: Option<Uuid>,
}

pub async fn list(
    Extension(st): Extension<AppState>,
    Query(q): Query<ListQuery>,
) -> impl IntoResponse {
    let repo = BackupRepository::new(st.db.clone());
    let rows = if let Some(vid) = q.volume_id {
        repo.list_for_volume(vid).await
    } else {
        sqlx::query_as::<_, BackupRow>(r#"SELECT * FROM backup ORDER BY created_at DESC LIMIT 200"#)
            .fetch_all(&st.db)
            .await
    };
    match rows {
        Ok(rs) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "items": rs.into_iter().map(row_to_wire).collect::<Vec<_>>(),
            })),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("backups list: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error":"db"})),
            )
                .into_response()
        }
    }
}

pub async fn get_one(
    Extension(st): Extension<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let repo = BackupRepository::new(st.db.clone());
    match repo.get(id).await {
        Ok(Some(row)) => (StatusCode::OK, Json(row_to_wire(row))).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error":"not found"})),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("backups get: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error":"db"})),
            )
                .into_response()
        }
    }
}

pub async fn create_for_volume(
    Extension(st): Extension<AppState>,
    Path(volume_id): Path<Uuid>,
    Json(req): Json<CreateBackupRequest>,
) -> impl IntoResponse {
    match service::create_backup(&st, volume_id, req.target_id).await {
        Ok(id) => (
            StatusCode::CREATED,
            Json(serde_json::json!({"backup_id": id})),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("create_backup: {e:#}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response()
        }
    }
}

pub async fn restore(
    Extension(st): Extension<AppState>,
    Path(backup_id): Path<Uuid>,
    Json(req): Json<RestoreRequest>,
) -> impl IntoResponse {
    match service::restore_backup(&st, backup_id, req.target_backend_id).await {
        Ok(volume_id) => (
            StatusCode::CREATED,
            Json(serde_json::json!({"volume_id": volume_id})),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("restore_backup: {e:#}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response()
        }
    }
}

pub async fn delete_one(
    Extension(st): Extension<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let repo = BackupRepository::new(st.db.clone());
    let backup = match repo.get(id).await {
        Ok(Some(b)) => b,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error":"not found"})),
            )
                .into_response()
        }
        Err(e) => {
            tracing::error!("backups delete lookup: {e}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error":"db"})),
            )
                .into_response();
        }
    };

    // Mark pruning so concurrent operations see the intent.
    sqlx::query(r#"UPDATE backup SET status = 'pruning', updated_at = now() WHERE id = $1"#)
        .bind(id)
        .execute(&st.db)
        .await
        .ok();

    // Delete the manifest from S3 first so GC can reclaim the chunks on its
    // next pass. If S3 delete fails, log a warning but still drop the DB row;
    // the rebuild tool can detect orphan manifests later.
    if let Some(mkey) = backup.manifest_object_key.as_deref() {
        if let Ok(Some(target)) =
            crate::features::backup_targets::repo::BackupTargetRepository::new(st.db.clone())
                .get(backup.target_id)
                .await
        {
            if let Ok(secret) = crate::features::backup_targets::envelope::unwrap_to_string(
                &target.encrypted_secret_access_key,
            ) {
                let creds = aws_credential_types::Credentials::new(
                    &target.access_key_id,
                    &secret,
                    None,
                    None,
                    "nqrust-mgr-delete",
                );
                let region = aws_sdk_s3::config::Region::new(
                    target.region.clone().unwrap_or_else(|| "us-east-1".into()),
                );
                let s3_cfg = aws_sdk_s3::config::Builder::new()
                    .behavior_version_latest()
                    .endpoint_url(&target.endpoint)
                    .credentials_provider(creds)
                    .region(region)
                    .force_path_style(true)
                    .build();
                let client = aws_sdk_s3::Client::from_conf(s3_cfg);
                if let Err(e) = client
                    .delete_object()
                    .bucket(&target.bucket)
                    .key(mkey)
                    .send()
                    .await
                {
                    tracing::warn!(backup_id=%id, manifest=%mkey, "S3 manifest delete failed: {e:#}");
                }
            }
        }
    }

    match repo.delete_row(id).await {
        Ok(()) => (StatusCode::NO_CONTENT, ()).into_response(),
        Err(e) => {
            tracing::error!("backups delete row: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error":"db"})),
            )
                .into_response()
        }
    }
}

pub fn router() -> Router {
    Router::new()
        .route("/", get(list))
        .route("/:id", get(get_one).delete(delete_one))
        .route("/:id/restore", post(restore))
}

pub fn volume_backup_router() -> Router {
    Router::new().route("/", post(create_for_volume))
}
