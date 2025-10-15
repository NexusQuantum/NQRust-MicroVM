use axum::extract::Path;
use axum::{http::StatusCode, routing::put, Extension, Json, Router};
use serde::{Deserialize, Serialize};
use std::path::Path as StdPath;
use tokio::fs;

use crate::AppState;

#[derive(Deserialize)]
struct VsockReq {
    guest_cid: u32,
    uds_path: String,
    #[serde(default)]
    vsock_id: Option<String>,
}

#[derive(Serialize)]
struct VsockResp {
    guest_cid: u32,
    uds_path: String,
    vsock_id: Option<String>,
}

pub fn router() -> Router {
    Router::new().route("/:id/vsock", put(configure_vsock))
}

async fn configure_vsock(
    Extension(st): Extension<AppState>,
    Path(id): Path<String>,
    Json(req): Json<VsockReq>,
) -> Result<Json<VsockResp>, (StatusCode, String)> {
    if let Some(parent) = StdPath::new(&req.uds_path).parent() {
        fs::create_dir_all(parent).await.map_err(internal_error)?;
    }

    if fs::metadata(&req.uds_path).await.is_ok() {
        fs::remove_file(&req.uds_path)
            .await
            .map_err(internal_error)?;
    }

    let config_dir = StdPath::new(&st.run_dir)
        .join("vms")
        .join(&id)
        .join("config");
    fs::create_dir_all(&config_dir)
        .await
        .map_err(internal_error)?;

    let cfg_path = config_dir.join("vsock.json");
    let body = serde_json::json!({
        "guest_cid": req.guest_cid,
        "uds_path": req.uds_path,
        "vsock_id": req.vsock_id,
    });
    fs::write(
        &cfg_path,
        serde_json::to_vec_pretty(&body).map_err(internal_error)?,
    )
    .await
    .map_err(internal_error)?;

    Ok(Json(VsockResp {
        guest_cid: req.guest_cid,
        uds_path: req.uds_path,
        vsock_id: req.vsock_id,
    }))
}

fn internal_error<E: std::fmt::Display>(err: E) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}
