use axum::{extract::Path, http::StatusCode, routing::put, Extension, Json, Router};
use serde::{Deserialize, Serialize};
use tokio::fs;

use crate::AppState;

#[derive(Deserialize, Serialize)]
struct SerialReq {
    #[serde(default)]
    output_path: Option<String>,
}

pub fn router() -> Router {
    Router::new().route("/:id/serial", put(configure_serial))
}

async fn configure_serial(
    Extension(st): Extension<AppState>,
    Path(id): Path<String>,
    Json(req): Json<SerialReq>,
) -> Result<Json<SerialReq>, (StatusCode, String)> {
    if let Some(path) = req.output_path.as_ref() {
        if let Some(parent) = std::path::Path::new(path).parent() {
            fs::create_dir_all(parent).await.map_err(internal_error)?;
        }
    }

    let cfg_path = config_path(&st.run_dir, &id, "serial.json");
    if let Some(parent) = cfg_path.parent() {
        fs::create_dir_all(parent).await.map_err(internal_error)?;
    }
    fs::write(
        &cfg_path,
        serde_json::to_vec_pretty(&req).map_err(internal_error)?,
    )
    .await
    .map_err(internal_error)?;

    Ok(Json(req))
}

fn config_path(run_dir: &str, vm_id: &str, file: &str) -> std::path::PathBuf {
    std::path::Path::new(run_dir)
        .join("vms")
        .join(vm_id)
        .join("config")
        .join(file)
}

fn internal_error<E: std::fmt::Display>(err: E) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}
