use axum::{extract::Path, http::StatusCode, routing::put, Extension, Json, Router};
use serde_json::Value;
use tokio::fs;

use crate::AppState;

pub fn router() -> Router {
    Router::new()
        .route("/:id/mmds", put(put_mmds).get(get_mmds))
        .route("/:id/mmds/config", put(put_mmds_config))
}

async fn put_mmds(
    Extension(st): Extension<AppState>,
    Path(id): Path<String>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let cfg_path = config_path(&st.run_dir, &id, "mmds.json");
    if let Some(parent) = cfg_path.parent() {
        fs::create_dir_all(parent).await.map_err(internal_error)?;
    }
    fs::write(
        &cfg_path,
        serde_json::to_vec_pretty(&body).map_err(internal_error)?,
    )
    .await
    .map_err(internal_error)?;
    Ok(Json(body))
}

async fn get_mmds(
    Extension(st): Extension<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let cfg_path = config_path(&st.run_dir, &id, "mmds.json");
    let data = match fs::read(&cfg_path).await {
        Ok(bytes) => serde_json::from_slice(&bytes).map_err(internal_error)?,
        Err(_) => Value::Object(Default::default()),
    };
    Ok(Json(data))
}

async fn put_mmds_config(
    Extension(st): Extension<AppState>,
    Path(id): Path<String>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let cfg_path = config_path(&st.run_dir, &id, "mmds-config.json");
    if let Some(parent) = cfg_path.parent() {
        fs::create_dir_all(parent).await.map_err(internal_error)?;
    }
    fs::write(
        &cfg_path,
        serde_json::to_vec_pretty(&body).map_err(internal_error)?,
    )
    .await
    .map_err(internal_error)?;
    Ok(Json(body))
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
