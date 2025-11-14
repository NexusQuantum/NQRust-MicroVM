use crate::features::volumes::repo::{VolumeRepository, VolumeRow};
use crate::AppState;
use axum::{extract::Path, http::StatusCode, Extension, Json};
use serde::{Deserialize, Serialize};
use tracing::error;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct CreateVolumeRequest {
    pub name: String,
    pub description: Option<String>,
    pub size_gb: i64,
    #[serde(rename = "type")]
    pub volume_type: String, // "raw", "qcow2", "ext4"
    pub host_id: Uuid,
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
    let host_name = st.hosts.get(volume.host_id).await.ok().map(|h| h.name);

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
        host_id: volume.host_id,
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

    // Get host to verify it exists
    let host = st.hosts.get(req.host_id).await.map_err(|err| match err {
        sqlx::Error::RowNotFound => StatusCode::NOT_FOUND,
        other => {
            error!(error = ?other, "failed to get host");
            StatusCode::INTERNAL_SERVER_ERROR
        }
    })?;

    // Create volume file path
    let volume_id = Uuid::new_v4();
    let run_dir = host
        .capabilities_json
        .get("run_dir")
        .and_then(|v| v.as_str())
        .unwrap_or("/srv/fc");
    let path = format!("{}/volumes/vol-{}.{}", run_dir, volume_id, req.volume_type);

    // Note: Volume file will be created on the agent host when first attached to a VM
    // This allows for lazy allocation and avoids pre-allocating large files
    let size_bytes = req.size_gb * 1024 * 1024 * 1024;

    // Create database record
    let volume_repo = VolumeRepository::new(st.db.clone());
    let volume = volume_repo
        .create(
            &req.name,
            req.description.as_deref(),
            &path,
            size_bytes,
            &req.volume_type,
            req.host_id,
        )
        .await
        .map_err(|err| {
            error!(?err, "failed to create volume");
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
) -> Result<Json<OkResponse>, StatusCode> {
    let volume_repo = VolumeRepository::new(st.db.clone());

    // Verify volume exists and is available
    let volume = volume_repo.get(id).await.map_err(|err| match err {
        sqlx::Error::RowNotFound => StatusCode::NOT_FOUND,
        other => {
            error!(error = ?other, "failed to get volume");
            StatusCode::INTERNAL_SERVER_ERROR
        }
    })?;

    if volume.status != "available" {
        return Err(StatusCode::CONFLICT);
    }

    // Attach volume
    volume_repo
        .attach(id, req.vm_id, &req.drive_id)
        .await
        .map_err(|err| {
            error!(?err, "failed to attach volume");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(OkResponse {
        message: "Volume attached successfully".to_string(),
    }))
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

    // Don't allow deletion if volume is attached
    if volume.status == "attached" {
        return Err(StatusCode::CONFLICT);
    }

    // Delete file if it exists
    if let Err(err) = tokio::fs::remove_file(&volume.path).await {
        error!(?err, path = %volume.path, "failed to delete volume file");
        // Continue anyway - database cleanup is more important
    }

    // Delete database record
    volume_repo.delete(id).await.map_err(|err| {
        error!(?err, "failed to delete volume from database");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(OkResponse {
        message: "Volume deleted successfully".to_string(),
    }))
}
