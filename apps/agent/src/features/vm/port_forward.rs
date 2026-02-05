use crate::core::net;
use axum::http::StatusCode;
use axum::{extract::Path, routing::{delete, post}, Json, Router};
use serde::Deserialize;

#[derive(Deserialize)]
struct PortForwardReq {
    guest_ip: String,
    host_port: u16,
    guest_port: u16,
    #[serde(default = "default_protocol")]
    protocol: String,
}

fn default_protocol() -> String {
    "tcp".to_string()
}

pub fn router() -> Router {
    Router::new()
        .route("/:id/port-forward", post(add_forward))
        .route("/:id/port-forward", delete(remove_forward))
}

async fn add_forward(
    Path(_id): Path<String>,
    Json(req): Json<PortForwardReq>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    net::add_port_forward(req.host_port, &req.guest_ip, req.guest_port, &req.protocol)
        .await
        .map_err(internal)?;

    Ok(Json(serde_json::json!({
        "ok": true,
        "host_port": req.host_port,
        "guest_port": req.guest_port,
        "protocol": req.protocol,
    })))
}

async fn remove_forward(
    Path(_id): Path<String>,
    Json(req): Json<PortForwardReq>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    net::remove_port_forward(req.host_port, &req.guest_ip, req.guest_port, &req.protocol)
        .await
        .map_err(internal)?;

    Ok(Json(serde_json::json!({"ok": true})))
}

fn internal<E: std::fmt::Display>(e: E) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
}
