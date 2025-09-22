use crate::core::net;
use crate::AppState;
use axum::http::StatusCode;
use axum::{
    extract::Path,
    routing::post,
    Json, Router,
};
use axum::Extension;
use serde::Deserialize;

#[derive(Deserialize)]
struct TapReq {
    bridge: Option<String>,
    owner_user: Option<String>,
}

pub fn router() -> Router {
    Router::new().route("/:id/tap", post(create_tap))
}

async fn create_tap(
    Extension(st): Extension<AppState>,
    Path(id): Path<String>,
    Json(req): Json<TapReq>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let tap = format!("tap-{id}");
    let bridge = req.bridge.unwrap_or(st.bridge.clone());
    net::ensure_bridge(&bridge, None).await.map_err(internal)?;
    net::create_tap(&tap, &bridge, req.owner_user.as_deref())
        .await
        .map_err(internal)?;
    Ok(Json(
        serde_json::json!({"ok": true, "tap": tap, "bridge": bridge}),
    ))
}
fn internal<E: std::fmt::Display>(e: E) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
}
