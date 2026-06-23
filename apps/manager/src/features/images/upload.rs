use anyhow::{Context, Result};
use axum::extract::multipart::Field;
use std::path::PathBuf;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

/// Stream a single multipart `file` field straight to disk while hashing it in
/// one pass. Unlike buffering the whole part in memory (`field.bytes()`), this
/// handles multi-GB ISOs (Windows installers, virtio-win, cloud images) without
/// blowing up RAM. Returns (path, sha256-hex, size-bytes).
pub async fn write_field_to_disk(
    mut field: Field<'_>,
    upload_dir: PathBuf,
    kind: &str,
) -> Result<(PathBuf, String, i64)> {
    use sha2::{Digest, Sha256};

    tokio::fs::create_dir_all(&upload_dir)
        .await
        .context("Failed to create upload directory")?;

    let original_filename = field.file_name().map(|s| s.to_string());
    let filename = match &original_filename {
        Some(fname) => sanitize_filename(fname),
        None => format!("{}-{}.img", kind, uuid::Uuid::new_v4()),
    };
    let path = upload_dir.join(&filename);

    let mut file = File::create(&path).await?;
    let mut hasher = Sha256::new();
    let mut size: i64 = 0;
    while let Some(chunk) = field.chunk().await? {
        file.write_all(&chunk).await?;
        hasher.update(&chunk);
        size += chunk.len() as i64;
    }
    file.flush().await?;

    let sha256 = format!("{:x}", hasher.finalize());
    tracing::info!("Uploaded file: {:?} ({} bytes)", path, size);
    Ok((path, sha256, size))
}

/// Move a staged upload into its final directory once the destination is known.
///
/// Uploads are streamed to a staging directory first so that multipart field
/// ordering does not matter (browsers send the `file` part before the `kind`
/// text field). A rename is used when source and destination share a
/// filesystem; otherwise we fall back to copy + remove.
pub async fn move_into_dir(staged: &std::path::Path, dest_dir: PathBuf) -> Result<PathBuf> {
    tokio::fs::create_dir_all(&dest_dir)
        .await
        .context("Failed to create destination directory")?;
    let filename = staged
        .file_name()
        .map(|n| n.to_owned())
        .context("staged upload has no filename")?;
    let dest = dest_dir.join(filename);

    if tokio::fs::rename(staged, &dest).await.is_err() {
        // Cross-filesystem move: copy then remove the staged file.
        tokio::fs::copy(staged, &dest)
            .await
            .context("Failed to copy staged upload to destination")?;
        let _ = tokio::fs::remove_file(staged).await;
    }
    Ok(dest)
}

/// Sanitize filename to prevent path traversal
fn sanitize_filename(filename: &str) -> String {
    filename
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '.' || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect::<String>()
        .chars()
        .take(255)
        .collect()
}

/// Load Docker image from tarball into local Docker daemon
pub async fn load_docker_image_from_tarball(tarball_path: &PathBuf) -> Result<String> {
    use tokio::process::Command;

    tracing::info!("Loading Docker image from tarball: {:?}", tarball_path);

    let output = Command::new("docker")
        .arg("load")
        .arg("-i")
        .arg(tarball_path)
        .output()
        .await
        .context("Failed to execute docker load")?;

    if !output.status.success() {
        anyhow::bail!(
            "Docker load failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    // Parse loaded image name from output
    // Format: "Loaded image: nginx:latest"
    let stdout = String::from_utf8_lossy(&output.stdout);
    let image_name = stdout
        .lines()
        .find(|line| line.contains("Loaded image:"))
        .and_then(|line| line.split(':').nth(1))
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown:latest".to_string());

    Ok(image_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("test.tar"), "test.tar");
        assert_eq!(sanitize_filename("../etc/passwd"), ".._etc_passwd");
        assert_eq!(sanitize_filename("my file.tar"), "my_file.tar");
        assert_eq!(sanitize_filename("test@#$.tar"), "test___.tar");
    }
}
