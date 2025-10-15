use crate::core::{net, systemd};
use crate::AppState;
use axum::{extract::Extension, http::StatusCode, routing::post, Json, Router};
use serde::{Deserialize, Serialize};
#[derive(Deserialize, Serialize)]
struct StopReq {
    tap: String,
    sock: String,
    fc_unit: String,
    #[serde(default)]
    storage_path: Option<String>,
}

pub fn router() -> Router {
    Router::new().route("/:id/stop", post(stop_vm))
}

async fn stop_vm(
    Extension(_st): Extension<AppState>,
    Json(req): Json<StopReq>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    if let Err(e) = systemd::stop_unit(&req.fc_unit).await {
        tracing::warn!(error = ?e, "failed to stop systemd unit");
    }
    if let Err(e) = net::delete_tap(&req.tap).await {
        tracing::warn!(error = ?e, "failed to delete tap device");
    }
    let _ = tokio::fs::remove_file(&req.sock).await;
    if let Some(path) = req.storage_path {
        if let Err(e) = tokio::fs::remove_dir_all(&path).await {
            tracing::warn!(error = ?e, path = %path, "failed to cleanup storage directory");
        }
    }
    Ok(Json(serde_json::json!({"ok": true})))
}
