use axum::{http::StatusCode, routing::post, Json, Router};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::process::Command;

#[derive(Deserialize)]
struct PrepareMetricsReq {
    metrics_path: String,
}

#[derive(Serialize)]
struct PrepareMetricsResp {
    metrics_path: String,
}

pub fn router() -> Router {
    Router::new().route("/:id/metrics/prepare", post(prepare))
}

async fn prepare(Json(req): Json<PrepareMetricsReq>) -> Result<Json<PrepareMetricsResp>, (StatusCode, String)> {
    // Ensure parent directory exists
    if let Some(parent) = Path::new(&req.metrics_path).parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(internal_error)?;
    }

    // If file exists but is not a FIFO, remove it
    if let Ok(md) = tokio::fs::metadata(&req.metrics_path).await {
        #[cfg(unix)]
        {
            use std::os::unix::fs::FileTypeExt as _;
            if !md.file_type().is_fifo() {
                tokio::fs::remove_file(&req.metrics_path)
                    .await
                    .map_err(internal_error)?;
            }
        }
        #[cfg(not(unix))]
        {
            // On non-unix, just remove and recreate
            tokio::fs::remove_file(&req.metrics_path)
                .await
                .map_err(internal_error)?;
        }
    }

    // Create FIFO if missing
    if tokio::fs::symlink_metadata(&req.metrics_path)
        .await
        .is_err()
    {
        let status = Command::new("sudo")
            .args(["-n", "mkfifo", &req.metrics_path])
            .status()
            .await
            .map_err(internal_error)?;
        if !status.success() {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to create metrics fifo".into(),
            ));
        }
    }

    // Set permissive permissions so Firecracker can open it
    let _ = Command::new("sudo")
        .args(["-n", "chmod", "666", &req.metrics_path])
        .status()
        .await;

    Ok(Json(PrepareMetricsResp {
        metrics_path: req.metrics_path,
    }))
}

fn internal_error<E: std::fmt::Display>(err: E) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}



