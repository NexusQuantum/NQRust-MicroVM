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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stop_req_round_trips() {
        let payload = r#"{
            "tap":"tap-vm01",
            "sock":"/srv/fc/vms/vm01/sock/fc.sock",
            "fc_unit":"fc-vm01.scope",
            "storage_path":"/srv/fc/vms/vm01"
        }"#;
        let req: StopReq = serde_json::from_str(payload).expect("valid StopReq");
        assert_eq!(req.tap, "tap-vm01");
        assert_eq!(req.sock, "/srv/fc/vms/vm01/sock/fc.sock");
        assert_eq!(req.fc_unit, "fc-vm01.scope");
        assert_eq!(req.storage_path.as_deref(), Some("/srv/fc/vms/vm01"));

        let encoded = serde_json::to_value(&req).unwrap();
        assert_eq!(encoded["tap"], "tap-vm01");
        assert_eq!(encoded["fc_unit"], "fc-vm01.scope");
        assert_eq!(encoded["storage_path"], "/srv/fc/vms/vm01");
    }

    #[test]
    fn stop_req_storage_path_optional() {
        // storage_path uses #[serde(default)] — payload may omit it entirely.
        let payload = r#"{"tap":"tap-x","sock":"/x.sock","fc_unit":"fc-x.scope"}"#;
        let req: StopReq = serde_json::from_str(payload).expect("optional storage_path");
        assert!(req.storage_path.is_none());
    }
}
