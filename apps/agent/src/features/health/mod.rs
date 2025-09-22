use axum::{response::IntoResponse, routing::get, Json, Router};
use crate::AppState;

pub fn router() -> Router {
    Router::new()
        .route("/agent/v1/health", get(health))
        .route("/agent/v1/capacity", get(capacity))
}

async fn health() -> impl IntoResponse {
    Json(serde_json::json!({
    "version": env!("CARGO_PKG_VERSION"),
    "kvm": std::path::Path::new("/dev/kvm").exists(),
    "time": chrono::Utc::now(),
    }))
}

async fn capacity() -> impl IntoResponse {
    Json(serde_json::json!({
    "cpu_total": num_cpus::get(),
    "cpu_free": num_cpus::get(),
    "mem_mib_total": 0,
    "mem_mib_free": 0,
    }))
}
