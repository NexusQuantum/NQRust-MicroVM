use axum::{extract::Path, http::StatusCode, routing::put, Extension, Json, Router};
use serde::{Deserialize, Serialize};
use tokio::fs;

use crate::AppState;

#[derive(Deserialize, Serialize)]
struct EntropyReq {
    #[serde(default)]
    rate_limiter: Option<serde_json::Value>,
}

pub fn router() -> Router {
    Router::new().route("/:id/entropy", put(configure_entropy))
}

async fn configure_entropy(
    Extension(st): Extension<AppState>,
    Path(id): Path<String>,
    Json(req): Json<EntropyReq>,
) -> Result<Json<EntropyReq>, (StatusCode, String)> {
    let cfg_path = config_path(&st.run_dir, &id, "entropy.json");
    fs::create_dir_all(cfg_path.parent().unwrap())
        .await
        .map_err(internal_error)?;
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
