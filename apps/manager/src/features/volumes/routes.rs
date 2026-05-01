use crate::features::volumes::repo::{VolumeRepository, VolumeRow};
use crate::AppState;
use axum::{
    extract::Path,
    http::StatusCode,
    response::{IntoResponse, Response},
    Extension, Json,
};
use serde::{Deserialize, Serialize};
use tracing::error;
use uuid::Uuid;

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct PatchBackupScheduleRequest {
    pub cron: Option<String>,
    pub retain_count: Option<i32>,
    pub target_id: Option<uuid::Uuid>,
}

pub async fn patch_backup_schedule(
    Extension(st): Extension<AppState>,
    Path(id): Path<uuid::Uuid>,
    Json(req): Json<PatchBackupScheduleRequest>,
) -> impl IntoResponse {
    use std::str::FromStr as _;
    if let Some(c) = &req.cron {
        if let Err(e) = cron::Schedule::from_str(c) {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": format!("invalid cron: {e}")})),
            )
                .into_response();
        }
    }
    let res = sqlx::query(
        r#"UPDATE volume SET backup_cron = COALESCE($1, backup_cron),
                              backup_retain_count = COALESCE($2, backup_retain_count),
                              backup_target_id = COALESCE($3, backup_target_id)
           WHERE id = $4"#,
    )
    .bind(req.cron)
    .bind(req.retain_count)
    .bind(req.target_id)
    .bind(id)
    .execute(&st.db)
    .await;
    match res {
        Ok(_) => (StatusCode::NO_CONTENT, ()).into_response(),
        Err(e) => {
            tracing::error!("patch_backup_schedule: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error":"db"})),
            )
                .into_response()
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateVolumeRequest {
    pub name: String,
    pub description: Option<String>,
    pub size_gb: i64,
    #[serde(rename = "type")]
    pub volume_type: String, // "raw", "qcow2", "ext4"
    pub host_id: Uuid,
    #[serde(default)]
    pub backend_id: Option<Uuid>,
}

#[derive(Debug, Serialize)]
pub struct CreateVolumeResponse {
    pub id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct VolumeListItem {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub path: String,
    pub size_bytes: i64,
    pub size_gb: i64,
    #[serde(rename = "type")]
    pub volume_type: String,
    pub status: String,
    pub host_id: Uuid,
    pub host_name: Option<String>,
    pub attached_to_vm_id: Option<Uuid>,
    pub attached_to_vm_name: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize)]
pub struct VolumeListResponse {
    pub items: Vec<VolumeListItem>,
}

#[derive(Debug, Serialize)]
pub struct VolumeDetailResponse {
    pub item: VolumeListItem,
}

#[derive(Debug, Deserialize)]
pub struct AttachVolumeRequest {
    pub vm_id: Uuid,
    pub drive_id: String,
}

#[derive(Debug, Deserialize)]
pub struct DetachVolumeRequest {
    pub vm_id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct OkResponse {
    pub message: String,
}

async fn volume_to_list_item(
    volume: VolumeRow,
    st: &AppState,
) -> Result<VolumeListItem, StatusCode> {
    let volume_repo = VolumeRepository::new(st.db.clone());

    // Get host name
    let host_name = if let Some(hid) = volume.host_id {
        st.hosts.get(hid).await.ok().map(|h| h.name)
    } else {
        None
    };

    // Get attached VM if any
    let attached_vm_id = volume_repo.get_attached_vm(volume.id).await.ok().flatten();

    let attached_vm_name = if let Some(vm_id) = attached_vm_id {
        sqlx::query_as::<_, (String,)>(r#"SELECT name FROM vm WHERE id = $1"#)
            .bind(vm_id)
            .fetch_optional(&st.db)
            .await
            .ok()
            .flatten()
            .map(|(name,)| name)
    } else {
        None
    };

    Ok(VolumeListItem {
        id: volume.id,
        name: volume.name,
        description: volume.description,
        path: volume.path,
        size_bytes: volume.size_bytes,
        size_gb: volume.size_bytes / (1024 * 1024 * 1024),
        volume_type: volume.type_,
        status: volume.status,
        host_id: volume.host_id.unwrap_or_default(),
        host_name,
        attached_to_vm_id: attached_vm_id,
        attached_to_vm_name: attached_vm_name,
        created_at: volume.created_at,
    })
}

#[utoipa::path(
    post,
    path = "/v1/volumes",
    request_body = CreateVolumeRequest,
    responses(
        (status = 200, description = "Volume created", body = CreateVolumeResponse),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Failed to create volume"),
    ),
    tag = "Volumes"
)]
pub async fn create(
    Extension(st): Extension<AppState>,
    Json(req): Json<CreateVolumeRequest>,
) -> Result<Json<CreateVolumeResponse>, StatusCode> {
    // Validate volume type
    if req.volume_type != "raw" && req.volume_type != "qcow2" && req.volume_type != "ext4" {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Validate size
    if req.size_gb <= 0 {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Verify host exists.
    let _host = st.hosts.get(req.host_id).await.map_err(|err| match err {
        sqlx::Error::RowNotFound => StatusCode::NOT_FOUND,
        other => {
            error!(error = ?other, "failed to get host");
            StatusCode::INTERNAL_SERVER_ERROR
        }
    })?;

    let size_bytes = req.size_gb * 1024 * 1024 * 1024;

    let backend_id = req
        .backend_id
        .or_else(|| st.registry.default_id())
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    // Drive the backend's `provision()` so the underlying resource (raft
    // block group, lvol, iSCSI LUN, local file) is actually allocated and
    // the row's `path` is the real backend locator. Without this, the
    // standalone volumes API previously stored a synthetic path string
    // and never asked the backend for storage at all — which left
    // raft_spdk / spdk_lvol / iSCSI volumes as DB-only ghosts.
    let alloc = crate::features::storage::rootfs_allocator::allocate_data_disk(
        &st.registry,
        backend_id,
        size_bytes as u64,
        &req.name,
    )
    .await
    .map_err(|err| {
        error!(?err, "backend.provision failed for standalone volume");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Persist the row with the backend-minted volume_id and locator so a
    // later attach/destroy can reconstruct the same VolumeHandle.
    let volume_repo = VolumeRepository::new(st.db.clone());
    let volume = volume_repo
        .create_with_id(
            Some(alloc.volume_id),
            &req.name,
            req.description.as_deref(),
            &alloc.locator,
            alloc.size_bytes as i64,
            &req.volume_type,
            Some(req.host_id),
            backend_id,
        )
        .await
        .map_err(|err| {
            error!(?err, "failed to create volume row after provision");
            // Best-effort backend rollback — if we can't record the row,
            // the backend resource we just created is orphaned.
            let registry = st.registry.clone();
            let handle = alloc.clone();
            tokio::spawn(async move {
                if let Some(backend) = registry.get(handle.backend_id.0).cloned() {
                    if let Err(e) = backend.destroy(handle).await {
                        tracing::warn!(error = ?e, "failed to roll back backend volume after DB insert error");
                    }
                }
            });
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(CreateVolumeResponse { id: volume.id }))
}

#[utoipa::path(
    get,
    path = "/v1/volumes",
    responses(
        (status = 200, description = "List of volumes", body = VolumeListResponse),
        (status = 500, description = "Failed to list volumes"),
    ),
    tag = "Volumes"
)]
pub async fn list(
    Extension(st): Extension<AppState>,
) -> Result<Json<VolumeListResponse>, StatusCode> {
    let volume_repo = VolumeRepository::new(st.db.clone());
    let volumes = volume_repo.list().await.map_err(|err| {
        error!(?err, "failed to list volumes");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let mut items = Vec::new();
    for volume in volumes {
        items.push(volume_to_list_item(volume, &st).await?);
    }

    Ok(Json(VolumeListResponse { items }))
}

#[utoipa::path(
    get,
    path = "/v1/volumes/{id}",
    responses(
        (status = 200, description = "Volume details", body = VolumeDetailResponse),
        (status = 404, description = "Volume not found"),
        (status = 500, description = "Failed to get volume"),
    ),
    tag = "Volumes"
)]
pub async fn get(
    Extension(st): Extension<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<VolumeDetailResponse>, StatusCode> {
    let volume_repo = VolumeRepository::new(st.db.clone());
    let volume = volume_repo.get(id).await.map_err(|err| match err {
        sqlx::Error::RowNotFound => StatusCode::NOT_FOUND,
        other => {
            error!(error = ?other, "failed to get volume");
            StatusCode::INTERNAL_SERVER_ERROR
        }
    })?;

    let item = volume_to_list_item(volume, &st).await?;

    Ok(Json(VolumeDetailResponse { item }))
}

#[utoipa::path(
    post,
    path = "/v1/volumes/{id}/attach",
    request_body = AttachVolumeRequest,
    responses(
        (status = 200, description = "Volume attached", body = OkResponse),
        (status = 404, description = "Volume or VM not found"),
        (status = 409, description = "Volume already attached"),
        (status = 500, description = "Failed to attach volume"),
    ),
    tag = "Volumes"
)]
pub async fn attach(
    Extension(st): Extension<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<AttachVolumeRequest>,
) -> Response {
    let volume_repo = VolumeRepository::new(st.db.clone());

    // Verify volume exists and is available
    let volume = match volume_repo.get(id).await {
        Ok(v) => v,
        Err(sqlx::Error::RowNotFound) => {
            return StatusCode::NOT_FOUND.into_response();
        }
        Err(other) => {
            error!(error = ?other, "failed to get volume");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    if volume.status != "available" {
        return (
            StatusCode::CONFLICT,
            Json(serde_json::json!({"error": "volume already attached"})),
        )
            .into_response();
    }

    // Attach volume
    let res = volume_repo.attach(id, req.vm_id, &req.drive_id).await;

    match res {
        Ok(_) => Json(OkResponse {
            message: "Volume attached successfully".to_string(),
        })
        .into_response(),
        Err(sqlx::Error::Database(db_err)) if db_err.code().as_deref() == Some("23505") => (
            StatusCode::CONFLICT,
            Json(serde_json::json!({"error": "volume already attached"})),
        )
            .into_response(),
        Err(e) => {
            error!("attach failed: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "db"})),
            )
                .into_response()
        }
    }
}

#[utoipa::path(
    post,
    path = "/v1/volumes/{id}/detach",
    request_body = DetachVolumeRequest,
    responses(
        (status = 200, description = "Volume detached", body = OkResponse),
        (status = 404, description = "Volume not found"),
        (status = 500, description = "Failed to detach volume"),
    ),
    tag = "Volumes"
)]
pub async fn detach(
    Extension(st): Extension<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<DetachVolumeRequest>,
) -> Result<Json<OkResponse>, StatusCode> {
    let volume_repo = VolumeRepository::new(st.db.clone());

    // Verify volume exists
    let _ = volume_repo.get(id).await.map_err(|err| match err {
        sqlx::Error::RowNotFound => StatusCode::NOT_FOUND,
        other => {
            error!(error = ?other, "failed to get volume");
            StatusCode::INTERNAL_SERVER_ERROR
        }
    })?;

    // Detach volume
    volume_repo.detach(id, req.vm_id).await.map_err(|err| {
        error!(?err, "failed to detach volume");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(OkResponse {
        message: "Volume detached successfully".to_string(),
    }))
}

#[utoipa::path(
    delete,
    path = "/v1/volumes/{id}",
    responses(
        (status = 200, description = "Volume deleted", body = OkResponse),
        (status = 404, description = "Volume not found"),
        (status = 409, description = "Volume is attached"),
        (status = 500, description = "Failed to delete volume"),
    ),
    tag = "Volumes"
)]
pub async fn delete(
    Extension(st): Extension<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<OkResponse>, StatusCode> {
    let volume_repo = VolumeRepository::new(st.db.clone());

    // Get volume to check status and get path
    let volume = volume_repo.get(id).await.map_err(|err| match err {
        sqlx::Error::RowNotFound => StatusCode::NOT_FOUND,
        other => {
            error!(error = ?other, "failed to get volume");
            StatusCode::INTERNAL_SERVER_ERROR
        }
    })?;

    // Don't allow deletion if volume is attached.
    if volume.status == "attached" {
        return Err(StatusCode::CONFLICT);
    }

    // Drive the backend's destroy() so backend resources (raft block group,
    // SPDK manifest + stub, lvol, iSCSI LUN) are released. Without this,
    // deleting a non-local_file volume row leaks the entire backend
    // resource and the next agent restart reloads an orphan group.
    //
    // We refuse to drop the DB row when destroy fails, mirroring the
    // VM-delete flow's "no silent backend/DB drift" contract: an operator
    // sees the volume row is still present and can fix the backend or
    // retry. local_file's destroy is idempotent (NotFound is treated as
    // success) so a stale row whose disk file is already gone still
    // deletes cleanly.
    if let Some(backend) = st.registry.get(volume.backend_id).cloned() {
        let handle = nexus_storage::VolumeHandle {
            volume_id: volume.id,
            backend_id: nexus_storage::BackendInstanceId(volume.backend_id),
            backend_kind: backend.kind(),
            locator: volume.path.clone(),
            size_bytes: volume.size_bytes.try_into().unwrap_or(0),
        };
        if let Err(err) = backend.destroy(handle).await {
            error!(
                volume_id = %id,
                backend_id = %volume.backend_id,
                error = ?err,
                "backend.destroy failed; volume row preserved so the backend resource stays visible to operators"
            );
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    } else {
        // The volume row references a backend that's no longer in the
        // registry (config rolled back, soft-deleted, etc.). We can't
        // call destroy, but we also can't leave the row dangling — log
        // and proceed with DB cleanup. The on-disk locator is best-effort
        // unlinked below.
        error!(
            volume_id = %id,
            backend_id = %volume.backend_id,
            "backend missing from registry; skipping backend.destroy and unlinking locator best-effort"
        );
        if let Err(err) = tokio::fs::remove_file(&volume.path).await {
            error!(?err, path = %volume.path, "failed to delete volume file");
        }
    }

    // Delete database record.
    volume_repo.delete(id).await.map_err(|err| {
        error!(?err, "failed to delete volume from database");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(OkResponse {
        message: "Volume deleted successfully".to_string(),
    }))
}
