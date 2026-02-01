use crate::AppState;
use axum::{
    extract::Path,
    http::StatusCode,
    routing::{delete, get, post},
    Extension, Json, Router,
};
use nexus_types::{
    CreateRuntimeSnapshotReq, CreateRuntimeSnapshotResp, GetRuntimeSnapshotResp,
    ListRuntimeSnapshotsResp, OkResponse, RebuildRuntimeSnapshotResp,
    RuntimeSnapshotPathParams,
};

use super::service::RuntimeSnapshotService;

pub fn runtime_snapshot_routes() -> Router {
    Router::new()
        .route("/", get(list))
        .route("/", post(create))
        .route("/:id", get(get_one))
        .route("/:id", delete(delete_one))
        .route("/:id/rebuild", post(rebuild))
}

#[utoipa::path(
    get,
    path = "/v1/runtime-snapshots",
    responses(
        (status = 200, description = "List of runtime snapshots", body = ListRuntimeSnapshotsResp),
        (status = 500, description = "Failed to list runtime snapshots"),
    ),
    tag = "Runtime Snapshots"
)]
async fn list(
    Extension(st): Extension<AppState>,
) -> Result<Json<ListRuntimeSnapshotsResp>, StatusCode> {
    let repo = super::repo::RuntimeSnapshotRepository::new(st.db.clone());
    let service = RuntimeSnapshotService::new(repo);

    let items = service
        .list()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(ListRuntimeSnapshotsResp { items }))
}

#[utoipa::path(
    get,
    path = "/v1/runtime-snapshots/{id}",
    params(RuntimeSnapshotPathParams),
    responses(
        (status = 200, description = "Runtime snapshot details", body = GetRuntimeSnapshotResp),
        (status = 404, description = "Runtime snapshot not found"),
        (status = 500, description = "Failed to get runtime snapshot"),
    ),
    tag = "Runtime Snapshots"
)]
async fn get_one(
    Extension(st): Extension<AppState>,
    Path(RuntimeSnapshotPathParams { id }): Path<RuntimeSnapshotPathParams>,
) -> Result<Json<GetRuntimeSnapshotResp>, StatusCode> {
    let repo = super::repo::RuntimeSnapshotRepository::new(st.db.clone());
    let service = RuntimeSnapshotService::new(repo);

    let item = service
        .get(id)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        })?;

    Ok(Json(GetRuntimeSnapshotResp { item }))
}

#[utoipa::path(
    post,
    path = "/v1/runtime-snapshots",
    request_body = CreateRuntimeSnapshotReq,
    responses(
        (status = 200, description = "Runtime snapshot creation initiated", body = CreateRuntimeSnapshotResp),
        (status = 404, description = "Runtime image not found"),
        (status = 500, description = "Failed to create runtime snapshot"),
    ),
    tag = "Runtime Snapshots"
)]
async fn create(
    Extension(st): Extension<AppState>,
    Json(req): Json<CreateRuntimeSnapshotReq>,
) -> Result<Json<CreateRuntimeSnapshotResp>, StatusCode> {
    // Verify runtime image exists
    let image_root = std::env::var("MANAGER_IMAGE_ROOT").unwrap_or_else(|_| "/srv/images".to_string());
    let image_repo = crate::features::images::repo::ImageRepository::new(st.db.clone(), image_root);
    let _image = image_repo
        .get(req.runtime_image_id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    // Detect Firecracker version
    let fc_version = super::builder::detect_firecracker_version()
        .await
        .unwrap_or_else(|_| "v1.9.0".to_string());

    // Create snapshot record
    let repo = super::repo::RuntimeSnapshotRepository::new(st.db.clone());
    let service = RuntimeSnapshotService::new(repo);

    // Create storage path
    let storage_root = std::env::var("MANAGER_STORAGE_ROOT").unwrap_or_else(|_| "/srv/fc".to_string());
    let snapshot_path = format!(
        "{}/runtime-snapshots/{}",
        storage_root,
        req.runtime_image_id
    );

    let snapshot = service
        .create(req.runtime_image_id, snapshot_path.clone(), fc_version)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let snapshot_id = snapshot.id;

    // Spawn background task to create the snapshot
    tokio::spawn(async move {
        tracing::info!(
            "Starting background snapshot creation for snapshot {}",
            snapshot_id
        );

        let builder = super::builder::RuntimeSnapshotBuilder::new(st.clone());
        let repo = super::repo::RuntimeSnapshotRepository::new(st.db.clone());

        match builder.build_snapshot(snapshot_id, req.runtime_image_id, &snapshot_path).await {
            Ok(metadata) => {
                tracing::info!(
                    "Snapshot {} created successfully (size: {} bytes)",
                    snapshot_id,
                    metadata.total_size_bytes
                );

                // Update snapshot state to ready and store metadata
                if let Err(e) = repo.update_state(snapshot_id, "ready").await {
                    tracing::error!("Failed to mark snapshot {} as ready: {}", snapshot_id, e);
                }

                // Store size metadata
                let metadata_json = serde_json::json!({
                    "size_bytes": metadata.total_size_bytes,
                    "mem_size_bytes": metadata.mem_size_bytes,
                    "state_size_bytes": metadata.state_size_bytes,
                    "rootfs_size_bytes": metadata.rootfs_size_bytes,
                    "compressed": metadata.compressed,
                });

                // Update metadata in database
                if let Err(e) = sqlx::query(
                    "UPDATE runtime_snapshots SET metadata = $1 WHERE id = $2"
                )
                .bind(&metadata_json)
                .bind(snapshot_id)
                .execute(&st.db)
                .await
                {
                    tracing::error!("Failed to update snapshot metadata: {}", e);
                }
            }
            Err(e) => {
                tracing::error!("Failed to create snapshot {}: {}", snapshot_id, e);

                // Mark snapshot as unhealthy
                if let Err(e) = repo.update_state(snapshot_id, "unhealthy").await {
                    tracing::error!("Failed to mark snapshot {} as unhealthy: {}", snapshot_id, e);
                }
            }
        }
    });

    Ok(Json(CreateRuntimeSnapshotResp { id: snapshot_id }))
}

#[utoipa::path(
    delete,
    path = "/v1/runtime-snapshots/{id}",
    params(RuntimeSnapshotPathParams),
    responses(
        (status = 200, description = "Runtime snapshot deleted", body = OkResponse),
        (status = 404, description = "Runtime snapshot not found"),
        (status = 500, description = "Failed to delete runtime snapshot"),
    ),
    tag = "Runtime Snapshots"
)]
async fn delete_one(
    Extension(st): Extension<AppState>,
    Path(RuntimeSnapshotPathParams { id }): Path<RuntimeSnapshotPathParams>,
) -> Result<Json<OkResponse>, StatusCode> {
    let repo = super::repo::RuntimeSnapshotRepository::new(st.db.clone());
    let service = RuntimeSnapshotService::new(repo);

    // Get snapshot to verify it exists
    let _snapshot = service
        .get(id)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        })?;

    // Soft delete (mark as deleted)
    service
        .delete(id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // TODO: Spawn background task to delete snapshot files

    Ok(Json(OkResponse::default()))
}

#[utoipa::path(
    post,
    path = "/v1/runtime-snapshots/{id}/rebuild",
    params(RuntimeSnapshotPathParams),
    responses(
        (status = 200, description = "Snapshot rebuild initiated", body = RebuildRuntimeSnapshotResp),
        (status = 404, description = "Runtime snapshot not found"),
        (status = 500, description = "Failed to rebuild runtime snapshot"),
    ),
    tag = "Runtime Snapshots"
)]
async fn rebuild(
    Extension(st): Extension<AppState>,
    Path(RuntimeSnapshotPathParams { id }): Path<RuntimeSnapshotPathParams>,
) -> Result<Json<RebuildRuntimeSnapshotResp>, StatusCode> {
    let repo = super::repo::RuntimeSnapshotRepository::new(st.db.clone());
    let service = RuntimeSnapshotService::new(repo);

    // Get snapshot
    let snapshot = service
        .get(id)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        })?;

    let runtime_image_id = snapshot.runtime_image_id;
    let snapshot_path = snapshot.snapshot_path.clone();

    // Mark as creating
    service
        .repo
        .update_state(id, "creating")
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Spawn background task to rebuild the snapshot
    tokio::spawn(async move {
        tracing::info!("Starting snapshot rebuild for snapshot {}", id);

        let builder = super::builder::RuntimeSnapshotBuilder::new(st.clone());
        let repo = super::repo::RuntimeSnapshotRepository::new(st.db.clone());

        // Delete old snapshot files if they exist
        if tokio::fs::metadata(&snapshot_path).await.is_ok() {
            if let Err(e) = tokio::fs::remove_dir_all(&snapshot_path).await {
                tracing::warn!("Failed to delete old snapshot files: {}", e);
            }
        }

        match builder.build_snapshot(id, runtime_image_id, &snapshot_path).await {
            Ok(metadata) => {
                tracing::info!(
                    "Snapshot {} rebuilt successfully (size: {} bytes)",
                    id,
                    metadata.total_size_bytes
                );

                // Reset failure count on successful rebuild
                if let Err(e) = sqlx::query(
                    "UPDATE runtime_snapshots SET failure_count = 0, success_count = 0 WHERE id = $1"
                )
                .bind(id)
                .execute(&st.db)
                .await
                {
                    tracing::error!("Failed to reset counters: {}", e);
                }

                // Update snapshot state to ready and store metadata
                if let Err(e) = repo.update_state(id, "ready").await {
                    tracing::error!("Failed to mark snapshot {} as ready: {}", id, e);
                }

                // Store size metadata
                let metadata_json = serde_json::json!({
                    "size_bytes": metadata.total_size_bytes,
                    "mem_size_bytes": metadata.mem_size_bytes,
                    "state_size_bytes": metadata.state_size_bytes,
                    "rootfs_size_bytes": metadata.rootfs_size_bytes,
                    "compressed": metadata.compressed,
                });

                if let Err(e) = sqlx::query(
                    "UPDATE runtime_snapshots SET metadata = $1 WHERE id = $2"
                )
                .bind(&metadata_json)
                .bind(id)
                .execute(&st.db)
                .await
                {
                    tracing::error!("Failed to update snapshot metadata: {}", e);
                }
            }
            Err(e) => {
                tracing::error!("Failed to rebuild snapshot {}: {}", id, e);

                // Mark snapshot as unhealthy
                if let Err(e) = repo.update_state(id, "unhealthy").await {
                    tracing::error!("Failed to mark snapshot {} as unhealthy: {}", id, e);
                }
            }
        }
    });

    Ok(Json(RebuildRuntimeSnapshotResp {
        id: snapshot.id,
        message: "Snapshot rebuild initiated".to_string(),
    }))
}
