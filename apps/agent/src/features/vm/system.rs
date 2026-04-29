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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bridge_info_serializes_as_simple_object() {
        let info = BridgeInfo {
            bridge: "fcbr0".into(),
        };
        let encoded = serde_json::to_string(&info).unwrap();
        assert_eq!(encoded, r#"{"bridge":"fcbr0"}"#);
    }
}
