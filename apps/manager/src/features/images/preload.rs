use anyhow::Result;
use std::path::PathBuf;

use super::dockerhub::DockerHubClient;
use super::repo::ImageRepository;
use nexus_types::CreateImageReq;

/// Default images to pre-load into the registry
const DEFAULT_IMAGES: &[&str] = &[
    "nginx:latest",
    "postgres:15",
    "redis:7",
    "node:20",
    "python:3.11",
];

/// Pre-load default Docker images into the registry
pub async fn preload_default_images(
    image_root: PathBuf,
    image_repo: &ImageRepository,
    progress_tracker: crate::DownloadProgressTracker,
) -> Result<Vec<uuid::Uuid>> {
    let dockerhub = DockerHubClient::new(image_root);
    let mut loaded_ids = Vec::new();

    tracing::info!(
        "Starting pre-load of {} default images",
        DEFAULT_IMAGES.len()
    );

    for image_name in DEFAULT_IMAGES {
        tracing::info!("Pre-loading image: {}", image_name);

        // Check if image already exists in registry
        let filter = nexus_types::ImageFilter {
            kind: Some("docker".to_string()),
            name: Some(image_name.to_string()),
            project: None,
        };

        let existing = image_repo.list(&filter).await?;
        if !existing.is_empty() {
            tracing::info!("Image {} already exists in registry, skipping", image_name);
            loaded_ids.push(existing[0].id);
            continue;
        }

        // Download the image
        match dockerhub
            .download_image(image_name, None, progress_tracker.clone())
            .await
        {
            Ok((tarball_path, sha256, size)) => {
                // Register in database
                let image_req = CreateImageReq {
                    kind: "docker".to_string(),
                    name: image_name.to_string(),
                    host_path: tarball_path.to_string_lossy().to_string(),
                    sha256,
                    size,
                    project: Some("preloaded".to_string()),
                };

                match image_repo.insert(&image_req).await {
                    Ok(image) => {
                        tracing::info!("✅ Successfully pre-loaded {}: {}", image_name, image.id);
                        loaded_ids.push(image.id);
                    }
                    Err(e) => {
                        tracing::error!("Failed to register {} in database: {}", image_name, e);
                    }
                }
            }
            Err(e) => {
                tracing::error!("Failed to download {}: {}", image_name, e);
            }
        }
    }

    tracing::info!(
        "Pre-load complete: {} of {} images loaded",
        loaded_ids.len(),
        DEFAULT_IMAGES.len()
    );

    Ok(loaded_ids)
}

/// Pre-load a custom list of images
#[allow(dead_code)]
pub async fn preload_custom_images(
    image_root: PathBuf,
    image_repo: &ImageRepository,
    images: &[String],
    progress_tracker: crate::DownloadProgressTracker,
) -> Result<Vec<uuid::Uuid>> {
    let dockerhub = DockerHubClient::new(image_root);
    let mut loaded_ids = Vec::new();

    tracing::info!("Starting pre-load of {} custom images", images.len());

    for image_name in images {
        tracing::info!("Pre-loading image: {}", image_name);

        // Download the image
        match dockerhub
            .download_image(image_name, None, progress_tracker.clone())
            .await
        {
            Ok((tarball_path, sha256, size)) => {
                // Register in database
                let image_req = CreateImageReq {
                    kind: "docker".to_string(),
                    name: image_name.clone(),
                    host_path: tarball_path.to_string_lossy().to_string(),
                    sha256,
                    size,
                    project: Some("custom".to_string()),
                };

                match image_repo.insert(&image_req).await {
                    Ok(image) => {
                        tracing::info!("✅ Successfully pre-loaded {}: {}", image_name, image.id);
                        loaded_ids.push(image.id);
                    }
                    Err(e) => {
                        tracing::error!("Failed to register {} in database: {}", image_name, e);
                    }
                }
            }
            Err(e) => {
                tracing::error!("Failed to download {}: {}", image_name, e);
            }
        }
    }

    tracing::info!(
        "Custom pre-load complete: {} of {} images loaded",
        loaded_ids.len(),
        images.len()
    );

    Ok(loaded_ids)
}
