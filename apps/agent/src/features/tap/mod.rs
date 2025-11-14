use crate::core::net;
use crate::AppState;
use axum::http::StatusCode;
use axum::Extension;
use axum::{extract::Path, routing::post, Json, Router};
use serde::Deserialize;

#[derive(Deserialize)]
struct TapReq {
    bridge: Option<String>,
    owner_user: Option<String>,
    vlan_id: Option<u16>,
    tap_name: Option<String>, // Allow custom TAP device name
}

pub fn router() -> Router {
    Router::new().route("/:id/tap", post(create_tap))
}

async fn create_tap(
    Extension(st): Extension<AppState>,
    Path(id): Path<String>,
    Json(req): Json<TapReq>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    // Use custom tap_name if provided, otherwise default to tap-{vm-short-id}
    let tap = req.tap_name.unwrap_or_else(|| format!("tap-{}", &id[..8]));
    let bridge = req.bridge.unwrap_or(st.bridge.clone());
    net::ensure_bridge(&bridge, None).await.map_err(internal)?;
    net::create_tap_with_vlan(&tap, &bridge, req.vlan_id, req.owner_user.as_deref())
        .await
        .map_err(internal)?;

    let mut response = serde_json::json!({"ok": true, "tap": tap, "bridge": bridge});
    if let Some(vlan) = req.vlan_id {
        response["vlan_id"] = serde_json::json!(vlan);
        response["vlan_bridge"] = serde_json::json!(format!("vlan{}-br", vlan));
    }

    Ok(Json(response))
}
fn internal<E: std::fmt::Display>(e: E) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
}
