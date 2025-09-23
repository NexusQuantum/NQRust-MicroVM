use crate::core::{net, systemd};
use crate::AppState;
use axum::http::StatusCode;
use axum::{routing::post, Extension, Json, Router};
use serde::Deserialize;

#[derive(Deserialize)]
struct StopReq {
    tap: String,
    sock: String,
    fc_unit: String,
}

pub fn router() -> Router {
    Router::new().route("/:id/stop", post(stop_vm))
}

async fn stop_vm(
    Extension(_st): Extension<AppState>,
    Json(req): Json<StopReq>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let _ = systemd::stop_unit(&req.fc_unit).await;
    let _ = net::delete_tap(&req.tap).await;
    let _ = tokio::fs::remove_file(&req.sock).await;
    Ok(Json(serde_json::json!({"ok": true})))
}
