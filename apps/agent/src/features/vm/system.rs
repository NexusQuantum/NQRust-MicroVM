use axum::{routing::get, Json, Router};
use serde::Serialize;

#[derive(Serialize)]
struct BridgeInfo {
    bridge: String,
}

pub fn router() -> Router {
    Router::new().route("/system/bridge", get(get_bridge))
}

async fn get_bridge() -> Json<BridgeInfo> {
    Json(BridgeInfo {
        bridge: std::env::var("AGENT_BRIDGE").unwrap_or_else(|_| "fcbr0".into()),
    })
}
