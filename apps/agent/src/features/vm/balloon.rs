use axum::{
    extract::Path,
    http::StatusCode,
    routing::{get, put},
    Extension, Json, Router,
};
use serde::{Deserialize, Serialize};
use tokio::fs;

use crate::AppState;

#[derive(Deserialize, Serialize, Clone)]
struct BalloonConfig {
    amount_mib: u64,
    deflate_on_oom: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    stats_polling_interval_s: Option<u64>,
}

#[derive(Deserialize, Serialize, Clone, Default)]
struct BalloonStatsConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    stats_polling_interval_s: Option<u64>,
}

#[derive(Serialize, Default)]
struct BalloonStats {
    target_pages: u64,
    actual_pages: u64,
    target_mib: u64,
    actual_mib: u64,
    swap_in: u64,
    swap_out: u64,
    major_faults: u64,
    minor_faults: u64,
    free_memory: u64,
    total_memory: u64,
    available_memory: u64,
    disk_caches: u64,
    hugetlb_allocations: u64,
    hugetlb_failures: u64,
}

pub fn router() -> Router {
    Router::new()
        .route(
            "/:id/balloon",
            put(put_balloon).patch(patch_balloon).get(get_balloon),
        )
        .route(
            "/:id/balloon/statistics",
            get(get_balloon_statistics).patch(patch_balloon_statistics),
        )
}

fn config_dir(state: &AppState, id: &str) -> std::path::PathBuf {
    std::path::Path::new(&state.run_dir)
        .join("vms")
        .join(id)
        .join("config")
}

async fn put_balloon(
    Extension(state): Extension<AppState>,
    Path(id): Path<String>,
    Json(req): Json<BalloonConfig>,
) -> Result<Json<BalloonConfig>, (StatusCode, String)> {
    let dir = config_dir(&state, &id);
    fs::create_dir_all(&dir).await.map_err(int)?;
    let path = dir.join("balloon.json");
    fs::write(&path, serde_json::to_vec_pretty(&req).map_err(int)?)
        .await
        .map_err(int)?;
    Ok(Json(req))
}

async fn patch_balloon(
    Extension(state): Extension<AppState>,
    Path(id): Path<String>,
    Json(req): Json<BalloonConfig>,
) -> Result<Json<BalloonConfig>, (StatusCode, String)> {
    put_balloon(Extension(state), Path(id), Json(req)).await
}

async fn get_balloon(
    Extension(state): Extension<AppState>,
    Path(id): Path<String>,
) -> Result<Json<BalloonConfig>, (StatusCode, String)> {
    let path = config_dir(&state, &id).join("balloon.json");
    let data = fs::read(&path)
        .await
        .map_err(|_| (StatusCode::NOT_FOUND, "balloon config not set".into()))?;
    let cfg: BalloonConfig = serde_json::from_slice(&data).map_err(int)?;
    Ok(Json(cfg))
}

async fn patch_balloon_statistics(
    Extension(state): Extension<AppState>,
    Path(id): Path<String>,
    Json(req): Json<BalloonStatsConfig>,
) -> Result<Json<BalloonStatsConfig>, (StatusCode, String)> {
    let dir = config_dir(&state, &id);
    fs::create_dir_all(&dir).await.map_err(int)?;
    let path = dir.join("balloon-stats.json");
    fs::write(&path, serde_json::to_vec_pretty(&req).map_err(int)?)
        .await
        .map_err(int)?;
    Ok(Json(req))
}

async fn get_balloon_statistics(
    Extension(_state): Extension<AppState>,
    Path(_id): Path<String>,
) -> Result<Json<BalloonStats>, (StatusCode, String)> {
    Ok(Json(BalloonStats::default()))
}

fn int<E: std::fmt::Display>(err: E) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state_with(run_dir: &str) -> AppState {
        AppState {
            run_dir: run_dir.to_string(),
            bridge: "fcbr0".into(),
            storage_registry: Default::default(),
            nfs_config: None,
        }
    }

    #[test]
    fn config_dir_layout_is_stable() {
        let st = state_with("/srv/fc");
        let dir = config_dir(&st, "vm-42");
        assert_eq!(dir, std::path::PathBuf::from("/srv/fc/vms/vm-42/config"));
    }

    #[test]
    fn balloon_config_round_trips_with_optional_field() {
        // stats_polling_interval_s should be omitted from output when None
        // (skip_serializing_if = "Option::is_none").
        let cfg = BalloonConfig {
            amount_mib: 256,
            deflate_on_oom: true,
            stats_polling_interval_s: None,
        };
        let encoded = serde_json::to_string(&cfg).unwrap();
        assert!(encoded.contains("\"amount_mib\":256"));
        assert!(encoded.contains("\"deflate_on_oom\":true"));
        assert!(
            !encoded.contains("stats_polling_interval_s"),
            "None field must be skipped, got {encoded}"
        );

        // With a value present, round-trip preserves it.
        let cfg2 = BalloonConfig {
            amount_mib: 512,
            deflate_on_oom: false,
            stats_polling_interval_s: Some(5),
        };
        let json = serde_json::to_string(&cfg2).unwrap();
        let decoded: BalloonConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.amount_mib, 512);
        assert!(!decoded.deflate_on_oom);
        assert_eq!(decoded.stats_polling_interval_s, Some(5));
    }

    #[test]
    fn balloon_stats_config_defaults_empty() {
        // Empty JSON object must deserialize cleanly thanks to #[serde(default)].
        let cfg: BalloonStatsConfig = serde_json::from_str("{}").expect("empty stats config");
        assert!(cfg.stats_polling_interval_s.is_none());

        // And the empty-omitted field stays out on serialize.
        let encoded = serde_json::to_string(&cfg).unwrap();
        assert_eq!(encoded, "{}");
    }
}
