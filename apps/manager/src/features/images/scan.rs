//! Auto-scan and register base images from the filesystem.
//!
//! This module scans the image root directory on startup and registers
//! any base images (kernels, rootfs, runtimes) that aren't already in the database.

use std::path::Path;

use nexus_types::{CreateImageReq, ImageFilter};
use sha2::{Digest, Sha256};
use tracing::{info, warn};

use super::repo::ImageRepository;

/// Known base images with their metadata
/// Format: (filename_pattern, kind, display_name, project)
const BASE_IMAGES: &[(&str, &str, &str, &str)] = &[
    // Kernels
    (
        "vmlinux-5.10.fc.bin",
        "kernel",
        "firecracker-v5.10",
        "firecracker-official",
    ),
    (
        "vmlinux.bin",
        "kernel",
        "firecracker-kernel",
        "firecracker-official",
    ),
    // Rootfs images
    (
        "alpine-3.18-minimal.ext4",
        "rootfs",
        "alpine-minimal",
        "alpine",
    ),
    ("busybox-1.35.ext4", "rootfs", "busybox-minimal", "busybox"),
    ("busybox-1.36.ext4", "rootfs", "busybox-minimal", "busybox"),
    (
        "ubuntu-24.04-minimal.ext4",
        "rootfs",
        "ubuntu-24.04-minimal",
        "ubuntu",
    ),
    (
        "ubuntu-22.04-minimal.ext4",
        "rootfs",
        "ubuntu-22.04-minimal",
        "ubuntu",
    ),
    // Function runtimes
    ("node-runtime.ext4", "rootfs", "node-runtime", "runtime"),
    ("python-runtime.ext4", "rootfs", "python-runtime", "runtime"),
    ("ruby-runtime.ext4", "rootfs", "ruby-runtime", "runtime"),
    // Container runtime
    (
        "container-runtime.ext4",
        "rootfs",
        "container-runtime",
        "container",
    ),
];

/// Scan the image root directory and register any base images not already in the database
pub async fn scan_and_register_base_images(image_repo: &ImageRepository) -> anyhow::Result<usize> {
    let image_root = image_repo.root();

    if !image_root.exists() {
        info!(
            "Image root {} does not exist, skipping base image scan",
            image_root.display()
        );
        return Ok(0);
    }

    info!("Scanning {} for base images...", image_root.display());

    let mut registered_count = 0;

    for (filename, kind, display_name, project) in BASE_IMAGES {
        let file_path = image_root.join(filename);

        if !file_path.exists() {
            continue;
        }

        // Check if already registered by checking for exact filename match
        let filter = ImageFilter {
            kind: Some(kind.to_string()),
            name: Some((*display_name).to_string()),
            project: None,
        };

        let existing = image_repo.list(&filter).await?;

        // Also check by host_path to avoid duplicates
        let host_path_str = file_path.to_string_lossy().to_string();
        let already_registered = existing.iter().any(|img| img.host_path == host_path_str);

        if already_registered {
            continue;
        }

        // Get file size and compute sha256
        let metadata = std::fs::metadata(&file_path)?;
        let size = metadata.len() as i64;

        let sha256 = compute_sha256(&file_path).await.unwrap_or_else(|e| {
            warn!("Failed to compute sha256 for {}: {}", filename, e);
            "unknown".to_string()
        });

        // Register the image
        let req = CreateImageReq {
            kind: kind.to_string(),
            name: display_name.to_string(),
            host_path: host_path_str,
            sha256,
            size,
            project: Some(project.to_string()),
        };

        match image_repo.insert(&req).await {
            Ok(image) => {
                info!("✅ Registered base image: {} ({})", display_name, image.id);
                registered_count += 1;
            }
            Err(e) => {
                warn!("Failed to register {}: {}", display_name, e);
            }
        }
    }

    if registered_count > 0 {
        info!("Registered {} base images", registered_count);
    } else {
        info!("No new base images to register");
    }

    Ok(registered_count)
}

/// Compute SHA256 hash of a file
async fn compute_sha256(path: &Path) -> anyhow::Result<String> {
    let path = path.to_path_buf();

    // Run in blocking task since file I/O can be slow for large images
    tokio::task::spawn_blocking(move || {
        let mut file = std::fs::File::open(&path)?;
        let mut hasher = Sha256::new();
        std::io::copy(&mut file, &mut hasher)?;
        let result = hasher.finalize();
        Ok(format!("{:x}", result))
    })
    .await?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_images_have_valid_kinds() {
        for (_, kind, _, _) in BASE_IMAGES {
            assert!(
                *kind == "kernel" || *kind == "rootfs",
                "Invalid kind: {}",
                kind
            );
        }
    }
}
