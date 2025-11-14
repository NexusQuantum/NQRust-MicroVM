use anyhow::{Context, Result};
use axum::extract::Multipart;
use std::path::PathBuf;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

/// Handle file upload and return (path, sha256, size)
pub async fn handle_file_upload(
    mut multipart: Multipart,
    upload_dir: PathBuf,
    kind: &str,
) -> Result<(PathBuf, String, i64)> {
    // Create upload directory if it doesn't exist
    tokio::fs::create_dir_all(&upload_dir)
        .await
        .context("Failed to create upload directory")?;

    let mut file_path: Option<PathBuf> = None;
    let mut _original_filename: Option<String> = None;

    // Process multipart form data
    while let Some(field) = multipart.next_field().await? {
        let name = field.name().unwrap_or("").to_string();

        if name == "file" {
            // Get original filename
            _original_filename = field.file_name().map(|s| s.to_string());

            // Generate safe filename
            let filename = match &_original_filename {
                Some(fname) => sanitize_filename(fname),
                None => format!("{}-{}.img", kind, uuid::Uuid::new_v4()),
            };

            let path = upload_dir.join(&filename);

            // Write file to disk
            let data = field.bytes().await?;
            let mut file = File::create(&path).await?;
            file.write_all(&data).await?;
            file.flush().await?;

            tracing::info!("Uploaded file: {:?} ({} bytes)", path, data.len());
            file_path = Some(path);
            break;
        }
    }

    let path = file_path.context("No file uploaded")?;

    // Calculate SHA256
    let sha256 = calculate_sha256(&path).await?;

    // Get file size
    let metadata = tokio::fs::metadata(&path).await?;
    let size = metadata.len() as i64;

    Ok((path, sha256, size))
}

/// Calculate SHA256 hash of a file
async fn calculate_sha256(path: &PathBuf) -> Result<String> {
    use sha2::{Digest, Sha256};
    use tokio::io::AsyncReadExt;

    let mut file = File::open(path).await?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 8192];

    loop {
        let n = file.read(&mut buffer).await?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }

    let hash = hasher.finalize();
    Ok(format!("{:x}", hash))
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
        .args(["load", "-i", tarball_path.to_str().unwrap()])
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
        assert_eq!(sanitize_filename("../etc/passwd"), "__.etc_passwd");
        assert_eq!(sanitize_filename("my file.tar"), "my_file.tar");
        assert_eq!(sanitize_filename("test@#$.tar"), "test___.tar");
    }
}
