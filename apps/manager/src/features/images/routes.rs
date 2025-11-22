use std::path::Path as StdPath;

use crate::{AppState, DownloadProgress};
use axum::{
    extract::{Multipart, Path, Query},
    http::StatusCode,
    Extension, Json,
};
use nexus_types::{
    CreateImageReq, CreateImageResp, DockerHubSearchReq, DockerHubSearchResp, DockerImageTagsResp,
    DownloadDockerImageReq, DownloadDockerImageResp, GetImageResp, ImageFilter, ImagePathParams,
    ListImagesResp, OkResponse,
};

#[utoipa::path(
    post,
    path = "/v1/images",
    request_body = CreateImageReq,
    responses(
        (status = 200, description = "Image registered", body = CreateImageResp),
        (status = 400, description = "Invalid image path"),
        (status = 500, description = "Failed to store image metadata"),
    ),
    tag = "Images"
)]
pub async fn create(
    Extension(st): Extension<AppState>,
    Json(req): Json<CreateImageReq>,
) -> Result<Json<CreateImageResp>, StatusCode> {
    if !st.images.is_path_allowed(StdPath::new(&req.host_path)) {
        return Err(StatusCode::BAD_REQUEST);
    }

    let image = st.images.insert(&req).await.map_err(map_repo_error)?;

    Ok(Json(CreateImageResp { id: image.id }))
}

#[utoipa::path(
    get,
    path = "/v1/images",
    params(ImageFilter),
    responses(
        (status = 200, description = "Images listed", body = ListImagesResp),
        (status = 500, description = "Failed to list images"),
    ),
    tag = "Images"
)]
pub async fn list(
    Extension(st): Extension<AppState>,
    Query(filter): Query<ImageFilter>,
) -> Result<Json<ListImagesResp>, StatusCode> {
    let items = st.images.list(&filter).await.map_err(map_repo_error)?;
    Ok(Json(ListImagesResp { items }))
}

#[utoipa::path(
    get,
    path = "/v1/images/{id}",
    params(ImagePathParams),
    responses(
        (status = 200, description = "Image fetched", body = GetImageResp),
        (status = 404, description = "Image not found"),
        (status = 500, description = "Failed to fetch image"),
    ),
    tag = "Images"
)]
pub async fn get(
    Extension(st): Extension<AppState>,
    Path(ImagePathParams { id }): Path<ImagePathParams>,
) -> Result<Json<GetImageResp>, StatusCode> {
    let item = st.images.get(id).await.map_err(map_repo_error)?;
    Ok(Json(GetImageResp { item }))
}

#[utoipa::path(
    delete,
    path = "/v1/images/{id}",
    params(ImagePathParams),
    responses(
        (status = 200, description = "Image deleted", body = OkResponse),
        (status = 404, description = "Image not found"),
        (status = 500, description = "Failed to delete image"),
    ),
    tag = "Images"
)]
pub async fn delete(
    Extension(st): Extension<AppState>,
    Path(ImagePathParams { id }): Path<ImagePathParams>,
) -> Result<Json<OkResponse>, StatusCode> {
    st.images.delete(id).await.map_err(map_repo_error)?;
    Ok(Json(OkResponse::default()))
}

fn map_repo_error(err: super::repo::ImageRepoError) -> StatusCode {
    match err {
        super::repo::ImageRepoError::InvalidPath(_) => StatusCode::BAD_REQUEST,
        super::repo::ImageRepoError::Sql(sqlx::Error::RowNotFound) => StatusCode::NOT_FOUND,
        super::repo::ImageRepoError::Sql(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

// Docker Hub API routes

#[utoipa::path(
    post,
    path = "/v1/images/dockerhub/search",
    request_body = DockerHubSearchReq,
    responses(
        (status = 200, description = "Docker Hub search results", body = DockerHubSearchResp),
        (status = 500, description = "Failed to search Docker Hub"),
    ),
    tag = "Images"
)]
pub async fn dockerhub_search(
    Extension(st): Extension<AppState>,
    Json(req): Json<DockerHubSearchReq>,
) -> Result<Json<DockerHubSearchResp>, StatusCode> {
    let dockerhub = super::dockerhub::DockerHubClient::new(st.images.root().to_path_buf());

    let items = dockerhub.search(&req.query, req.limit).await.map_err(|e| {
        tracing::error!("Docker Hub search failed: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(DockerHubSearchResp { items }))
}

#[utoipa::path(
    post,
    path = "/v1/images/dockerhub/tags",
    request_body = inline(String),
    responses(
        (status = 200, description = "Docker image tags", body = DockerImageTagsResp),
        (status = 500, description = "Failed to get image tags"),
    ),
    tag = "Images"
)]
pub async fn dockerhub_tags(
    Extension(st): Extension<AppState>,
    Json(image_name): Json<String>,
) -> Result<Json<DockerImageTagsResp>, StatusCode> {
    let dockerhub = super::dockerhub::DockerHubClient::new(st.images.root().to_path_buf());

    let items = dockerhub.get_tags(&image_name).await.map_err(|e| {
        tracing::error!("Failed to get Docker image tags: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(DockerImageTagsResp { items }))
}

#[utoipa::path(
    post,
    path = "/v1/images/dockerhub/download",
    request_body = DownloadDockerImageReq,
    responses(
        (status = 200, description = "Docker image downloaded and cached", body = DownloadDockerImageResp),
        (status = 500, description = "Failed to download image"),
    ),
    tag = "Images"
)]
pub async fn dockerhub_download(
    Extension(st): Extension<AppState>,
    Json(req): Json<DownloadDockerImageReq>,
) -> Result<Json<DownloadDockerImageResp>, StatusCode> {
    tracing::info!("Starting Docker image download: {}", req.image);

    // Initialize progress tracking
    {
        let mut progress_map = st.download_progress.lock().await;
        progress_map.insert(
            req.image.clone(),
            DownloadProgress {
                image: req.image.clone(),
                status: "Initializing...".to_string(),
                current_bytes: 0,
                total_bytes: 0,
                completed: false,
                error: None,
            },
        );
    }

    let dockerhub = super::dockerhub::DockerHubClient::new(st.images.root().to_path_buf());

    // Download the image and save as tarball
    let download_result = dockerhub
        .download_image(
            &req.image,
            req.registry_auth.as_ref(),
            st.download_progress.clone(),
        )
        .await;

    let (tarball_path, sha256, size) = match download_result {
        Ok(result) => result,
        Err(e) => {
            tracing::error!("Failed to download Docker image '{}': {}", req.image, e);
            // Update progress with error
            let mut progress_map = st.download_progress.lock().await;
            if let Some(progress) = progress_map.get_mut(&req.image) {
                progress.error = Some(e.to_string());
                progress.completed = true;
                progress.status = "Failed".to_string();
            }
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    // Register the image in the database
    let image_req = CreateImageReq {
        kind: "docker".to_string(),
        name: req.image.clone(),
        host_path: tarball_path.to_string_lossy().to_string(),
        sha256: sha256.clone(),
        size,
        project: Some("dockerhub".to_string()),
    };

    let image = st.images.insert(&image_req).await.map_err(map_repo_error)?;

    // Mark download as completed
    {
        let mut progress_map = st.download_progress.lock().await;
        if let Some(progress) = progress_map.get_mut(&req.image) {
            progress.completed = true;
            progress.status = "Completed".to_string();
            progress.current_bytes = size;
            progress.total_bytes = size;
        }
    }

    Ok(Json(DownloadDockerImageResp {
        id: image.id,
        path: tarball_path.to_string_lossy().to_string(),
    }))
}

#[utoipa::path(
    get,
    path = "/v1/images/dockerhub/download/progress/{image_name}",
    params(
        ("image_name" = String, Path, description = "Docker image name (e.g., nginx:latest)")
    ),
    responses(
        (status = 200, description = "Download progress", body = DownloadProgress),
        (status = 404, description = "Download not found"),
    ),
    tag = "Images"
)]
pub async fn dockerhub_download_progress(
    Extension(st): Extension<AppState>,
    Path(image_name): Path<String>,
) -> Result<Json<DownloadProgress>, StatusCode> {
    // Decode URL-encoded image name (e.g., "postgres%3Alatest" -> "postgres:latest")
    let decoded_name = urlencoding::decode(&image_name)
        .map_err(|_| StatusCode::BAD_REQUEST)?
        .to_string();

    let progress_map = st.download_progress.lock().await;

    // Try both encoded and decoded names for compatibility
    if let Some(progress) = progress_map
        .get(&decoded_name)
        .or_else(|| progress_map.get(&image_name))
    {
        Ok(Json(progress.clone()))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

#[utoipa::path(
    post,
    path = "/v1/images/dockerhub/preload",
    responses(
        (status = 200, description = "Default images pre-loaded", body = inline(Vec<String>)),
        (status = 500, description = "Failed to pre-load images"),
    ),
    tag = "Images"
)]
pub async fn dockerhub_preload(
    Extension(st): Extension<AppState>,
) -> Result<Json<Vec<String>>, StatusCode> {
    let loaded_ids = super::preload::preload_default_images(
        st.images.root().to_path_buf(),
        &st.images,
        st.download_progress.clone(),
    )
    .await
    .map_err(|e| {
        tracing::error!("Failed to pre-load default images: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(
        loaded_ids.into_iter().map(|id| id.to_string()).collect(),
    ))
}

#[utoipa::path(
    post,
    path = "/v1/images/upload",
    request_body(content = inline(String), description = "Multipart form data with 'file' and 'kind' fields", content_type = "multipart/form-data"),
    responses(
        (status = 200, description = "Image uploaded successfully", body = CreateImageResp),
        (status = 400, description = "Invalid file or missing fields"),
        (status = 500, description = "Upload failed"),
    ),
    tag = "Images"
)]
pub async fn upload_image(
    Extension(st): Extension<AppState>,
    mut multipart: Multipart,
) -> Result<Json<CreateImageResp>, StatusCode> {
    let mut kind: Option<String> = None;
    let mut name: Option<String> = None;
    let mut project: Option<String> = None;

    // Extract metadata fields first
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?
    {
        let field_name = field.name().unwrap_or("").to_string();

        match field_name.as_str() {
            "kind" => {
                kind = Some(field.text().await.map_err(|_| StatusCode::BAD_REQUEST)?);
            }
            "name" => {
                name = Some(field.text().await.map_err(|_| StatusCode::BAD_REQUEST)?);
            }
            "project" => {
                project = Some(field.text().await.map_err(|_| StatusCode::BAD_REQUEST)?);
            }
            "file" => {
                // We'll handle file in the next pass
                break;
            }
            _ => {}
        }
    }

    let kind = kind.ok_or(StatusCode::BAD_REQUEST)?;
    let upload_dir = match kind.as_str() {
        "docker" => st.images.root().join("docker"),
        "kernel" | "rootfs" => st.images.root().to_path_buf(),
        _ => return Err(StatusCode::BAD_REQUEST),
    };

    // Handle file upload
    let (file_path, sha256, size) = super::upload::handle_file_upload(multipart, upload_dir, &kind)
        .await
        .map_err(|e| {
            tracing::error!("File upload failed: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // If Docker image, load it to get the actual image name
    let default_name = || {
        file_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string())
    };
    let image_name = if kind == "docker" {
        match super::upload::load_docker_image_from_tarball(&file_path).await {
            Ok(name) => name,
            Err(e) => {
                tracing::warn!("Failed to load Docker image, using filename: {}", e);
                name.unwrap_or_else(default_name)
            }
        }
    } else {
        name.unwrap_or_else(default_name)
    };

    // Register in database
    let image_req = CreateImageReq {
        kind,
        name: image_name,
        host_path: file_path.to_string_lossy().to_string(),
        sha256,
        size,
        project: project.or(Some("uploaded".to_string())),
    };

    let image = st.images.insert(&image_req).await.map_err(map_repo_error)?;

    Ok(Json(CreateImageResp { id: image.id }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::hosts::repo::HostRepository;
    use crate::features::images::repo::ImageRepository;
    use axum::{extract::Path, Extension};
    use nexus_types::CreateImageReq;

    #[ignore]
    #[sqlx::test(migrations = "./migrations")]
    async fn create_and_list_images(pool: sqlx::PgPool) {
        let hosts = HostRepository::new(pool.clone());
        let images = ImageRepository::new(pool.clone(), "/srv/images");
        let snapshots = crate::features::snapshots::repo::SnapshotRepository::new(pool.clone());
        let storage = crate::features::storage::LocalStorage::new();
        storage.init().await.unwrap();
        let shell_repo = crate::features::vms::shell::ShellRepository::new(pool.clone());
        let download_progress =
            std::sync::Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new()));
        let users = crate::features::users::repo::UserRepository::new(pool.clone());
        let state = crate::AppState {
            db: pool.clone(),
            hosts,
            images: images.clone(),
            snapshots,
            users,
            shell_repo,
            allow_direct_image_paths: true,
            storage,
            download_progress,
        };

        let req = CreateImageReq {
            kind: "kernel".into(),
            name: "linux".into(),
            host_path: "/srv/images/vmlinux".into(),
            sha256: "deadbeef".into(),
            size: 1234,
            project: Some("default".into()),
        };

        let Json(resp) = super::create(Extension(state.clone()), Json(req.clone()))
            .await
            .unwrap();

        let Json(list) = super::list(Extension(state.clone()), Query(ImageFilter::default()))
            .await
            .unwrap();
        assert_eq!(list.items.len(), 1);
        assert_eq!(list.items[0].id, resp.id);

        let Json(item) = super::get(
            Extension(state.clone()),
            Path(ImagePathParams { id: resp.id }),
        )
        .await
        .unwrap();
        assert_eq!(item.item.name, req.name);

        let Json(ok) = super::delete(
            Extension(state.clone()),
            Path(ImagePathParams { id: resp.id }),
        )
        .await
        .unwrap();
        assert_eq!(ok, OkResponse::default());
    }

    #[ignore]
    #[sqlx::test(migrations = "./migrations")]
    async fn reject_out_of_root_path(pool: sqlx::PgPool) {
        let hosts = HostRepository::new(pool.clone());
        let images = ImageRepository::new(pool.clone(), "/srv/images");
        let snapshots = crate::features::snapshots::repo::SnapshotRepository::new(pool.clone());
        let shell_repo = crate::features::vms::shell::ShellRepository::new(pool.clone());
        let storage = crate::features::storage::LocalStorage::new();
        storage.init().await.unwrap();
        let download_progress =
            std::sync::Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new()));
        let users = crate::features::users::repo::UserRepository::new(pool.clone());
        let state = crate::AppState {
            db: pool,
            hosts,
            images,
            snapshots,
            users,
            shell_repo,
            allow_direct_image_paths: true,
            storage,
            download_progress,
        };

        let req = CreateImageReq {
            kind: "kernel".into(),
            name: "bad".into(),
            host_path: "/etc/passwd".into(),
            sha256: "deadbeef".into(),
            size: 1234,
            project: None,
        };

        let result = super::create(Extension(state), Json(req)).await;
        assert_eq!(result.unwrap_err(), StatusCode::BAD_REQUEST);
    }
}
