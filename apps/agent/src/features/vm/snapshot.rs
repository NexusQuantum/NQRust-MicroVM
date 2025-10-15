use std::path::{Path, PathBuf};

use axum::{extract::Path as AxumPath, http::StatusCode, routing::post, Extension, Json, Router};
use serde::{Deserialize, Serialize};
use tokio::fs;
use uuid::Uuid;

use crate::AppState;

pub fn router() -> Router {
    Router::new().route("/:id/snapshots/prepare", post(prepare))
}

#[derive(Deserialize)]
struct PrepareSnapshotRequest {
    snapshot_id: Uuid,
    #[serde(default)]
    snapshot_type: Option<String>,
}

#[derive(Serialize)]
struct PrepareSnapshotResponse {
    snapshot_path: String,
    mem_path: Option<String>,
    diff_dir: Option<String>,
    snapshot_size_bytes: Option<u64>,
    mem_size_bytes: Option<u64>,
}

async fn prepare(
    Extension(st): Extension<AppState>,
    AxumPath(vm_id): AxumPath<Uuid>,
    Json(req): Json<PrepareSnapshotRequest>,
) -> Result<Json<PrepareSnapshotResponse>, (StatusCode, String)> {
    let run_dir = PathBuf::from(&st.run_dir);
    let base_dir = snapshot_base_dir(&run_dir, &vm_id, &req.snapshot_id);
    fs::create_dir_all(&base_dir)
        .await
        .map_err(internal_error)?;
    let base_dir = canonicalize_dir(&base_dir).await?;

    let snapshot_type = req.snapshot_type.as_deref().unwrap_or("Full").to_string();

    let snapshot_path = base_dir.join(match snapshot_type.as_str() {
        "Diff" => "diff.fc",
        _ => "snapshot.fc",
    });

    let mem_path = if snapshot_type == "Diff" {
        None
    } else {
        let mem_dir = base_dir.join("mem");
        fs::create_dir_all(&mem_dir).await.map_err(internal_error)?;
        let mem_dir = canonicalize_dir(&mem_dir).await?;
        Some(mem_dir.join("mem.fc"))
    };

    let diff_dir = if snapshot_type == "Diff" {
        let dir = base_dir.join("diff");
        fs::create_dir_all(&dir).await.map_err(internal_error)?;
        let dir = canonicalize_dir(&dir).await?;
        Some(dir)
    } else {
        None
    };

    let (_, snapshot_size_bytes) = file_status(&snapshot_path).await?;
    let mem_size_bytes = match &mem_path {
        Some(path) => file_status(path).await?.1,
        None => None,
    };

    Ok(Json(PrepareSnapshotResponse {
        snapshot_path: path_to_string(&snapshot_path)?,
        mem_path: mem_path.map(|p| path_to_string(&p)).transpose()?,
        diff_dir: diff_dir.map(|p| path_to_string(&p)).transpose()?,
        snapshot_size_bytes,
        mem_size_bytes,
    }))
}

fn snapshot_base_dir(run_dir: &Path, vm_id: &Uuid, snapshot_id: &Uuid) -> PathBuf {
    run_dir
        .join("vms")
        .join(vm_id.to_string())
        .join("snapshots")
        .join(snapshot_id.to_string())
}

async fn canonicalize_dir(path: &Path) -> Result<PathBuf, (StatusCode, String)> {
    fs::canonicalize(path).await.map_err(internal_error)
}

async fn file_status(path: &Path) -> Result<(bool, Option<u64>), (StatusCode, String)> {
    match fs::metadata(path).await {
        Ok(meta) => {
            if meta.is_file() {
                Ok((true, Some(meta.len())))
            } else {
                Ok((true, None))
            }
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok((false, None)),
        Err(err) => Err(internal_error(err)),
    }
}

fn path_to_string(path: &Path) -> Result<String, (StatusCode, String)> {
    path.to_str().map(|s| s.to_owned()).ok_or_else(|| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "path encoding error".into(),
        )
    })
}

fn internal_error<E: std::fmt::Display>(err: E) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_dir_includes_vm_and_snapshot() {
        let vm_id = Uuid::new_v4();
        let snapshot_id = Uuid::new_v4();
        let base = snapshot_base_dir(Path::new("/srv/fc"), &vm_id, &snapshot_id);
        assert!(base.ends_with(format!("{snapshot_id}")));
        assert!(base.starts_with(Path::new("/srv/fc/vms")));
    }

    #[tokio::test]
    async fn file_status_reports_sizes() {
        let tmp = tempfile::tempdir().unwrap();
        let file_path = tmp.path().join("file.bin");
        assert_eq!(file_status(&file_path).await.unwrap(), (false, None));

        tokio::fs::write(&file_path, &[1u8; 8]).await.unwrap();
        assert_eq!(file_status(&file_path).await.unwrap(), (true, Some(8)));
    }
}
