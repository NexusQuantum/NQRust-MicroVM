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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_path_layout_is_stable() {
        let path = config_path("/srv/fc", "vm-xyz", "entropy.json");
        assert_eq!(
            path,
            std::path::PathBuf::from("/srv/fc/vms/vm-xyz/config/entropy.json")
        );
    }

    #[test]
    fn entropy_req_round_trips() {
        // Default: rate_limiter omitted entirely.
        let req: EntropyReq = serde_json::from_str("{}").expect("default rate_limiter");
        assert!(req.rate_limiter.is_none());

        // With a value: opaque JSON forwarded as-is to Firecracker.
        let payload = r#"{"rate_limiter":{"bandwidth":{"size":1000,"refill_time":100}}}"#;
        let req: EntropyReq = serde_json::from_str(payload).unwrap();
        let limiter = req.rate_limiter.expect("rate_limiter present");
        assert_eq!(limiter["bandwidth"]["size"], 1000);
        assert_eq!(limiter["bandwidth"]["refill_time"], 100);

        // Round-trip back to JSON keeps the inner object intact.
        let encoded = serde_json::to_string(&EntropyReq {
            rate_limiter: Some(limiter),
        })
        .unwrap();
        assert!(encoded.contains("\"size\":1000"));
    }
}
