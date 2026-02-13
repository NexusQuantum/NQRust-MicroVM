use crate::core::net;
use axum::http::StatusCode;
use axum::{
    extract::Path,
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;

#[derive(Deserialize)]
struct ProvisionReq {
    network_type: String,
    bridge_name: String,
    #[serde(default)]
    cidr: String,
    #[serde(default)]
    gateway: String,
    #[serde(default = "default_true")]
    dhcp_enabled: bool,
    #[serde(default)]
    dhcp_range_start: String,
    #[serde(default)]
    dhcp_range_end: String,
    /// Required for bridged networks: the physical NIC to attach
    uplink_interface: Option<String>,
    /// VXLAN fields
    vni: Option<u32>,
    local_ip: Option<String>,
    #[serde(default)]
    is_gateway: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Deserialize)]
struct TeardownReq {
    network_type: String,
    bridge_name: String,
    #[serde(default)]
    cidr: String,
    /// VXLAN fields
    vni: Option<u32>,
    #[serde(default)]
    is_gateway: bool,
}

#[derive(Deserialize)]
struct PeerReq {
    vni: u32,
    peer_ip: String,
}

pub fn router() -> Router {
    Router::new()
        .route("/provision", post(provision))
        .route("/teardown", post(teardown))
        .route("/interfaces", get(list_interfaces))
        .route("/status/:bridge", get(status))
        .route("/peers/add", post(add_peer))
        .route("/peers/remove", post(remove_peer))
}

async fn provision(
    Json(req): Json<ProvisionReq>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    match req.network_type.as_str() {
        "nat" => {
            net::provision_nat_network(
                &req.bridge_name,
                &req.cidr,
                &req.gateway,
                req.dhcp_enabled,
                &req.dhcp_range_start,
                &req.dhcp_range_end,
            )
            .await
            .map_err(internal)?;
        }
        "isolated" => {
            net::provision_isolated_network(
                &req.bridge_name,
                &req.cidr,
                &req.gateway,
                req.dhcp_enabled,
                &req.dhcp_range_start,
                &req.dhcp_range_end,
            )
            .await
            .map_err(internal)?;
        }
        "bridged" => {
            let uplink = req.uplink_interface.as_deref().ok_or_else(|| {
                (
                    StatusCode::BAD_REQUEST,
                    "uplink_interface is required for bridged networks".to_string(),
                )
            })?;
            net::provision_bridged_network(&req.bridge_name, uplink)
                .await
                .map_err(internal)?;
        }
        "vxlan" => {
            let vni = req.vni.ok_or_else(|| {
                (
                    StatusCode::BAD_REQUEST,
                    "vni is required for VXLAN networks".to_string(),
                )
            })?;
            let local_ip = req.local_ip.as_deref().ok_or_else(|| {
                (
                    StatusCode::BAD_REQUEST,
                    "local_ip is required for VXLAN networks".to_string(),
                )
            })?;
            net::provision_vxlan_network(
                &req.bridge_name,
                vni,
                local_ip,
                &req.cidr,
                &req.gateway,
                req.is_gateway,
                req.dhcp_enabled,
                &req.dhcp_range_start,
                &req.dhcp_range_end,
            )
            .await
            .map_err(internal)?;
        }
        other => {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("unsupported network type for provisioning: {other}"),
            ));
        }
    }

    Ok(Json(serde_json::json!({
        "ok": true,
        "bridge": req.bridge_name,
        "network_type": req.network_type,
    })))
}

async fn teardown(
    Json(req): Json<TeardownReq>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    match req.network_type.as_str() {
        "bridged" => {
            net::teardown_bridged_network(&req.bridge_name)
                .await
                .map_err(internal)?;
        }
        "vxlan" => {
            let vni = req.vni.ok_or_else(|| {
                (
                    StatusCode::BAD_REQUEST,
                    "vni is required for VXLAN teardown".to_string(),
                )
            })?;
            net::teardown_vxlan_network(&req.bridge_name, vni, &req.cidr, req.is_gateway)
                .await
                .map_err(internal)?;
        }
        _ => {
            net::teardown_network(&req.network_type, &req.bridge_name, &req.cidr)
                .await
                .map_err(internal)?;
        }
    }

    Ok(Json(serde_json::json!({
        "ok": true,
        "bridge": req.bridge_name,
    })))
}

async fn add_peer(
    Json(req): Json<PeerReq>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    net::add_vxlan_peer(req.vni, &req.peer_ip)
        .await
        .map_err(internal)?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

async fn remove_peer(
    Json(req): Json<PeerReq>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    net::remove_vxlan_peer(req.vni, &req.peer_ip)
        .await
        .map_err(internal)?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

async fn list_interfaces() -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let interfaces = net::list_interfaces().await.map_err(internal)?;
    Ok(Json(serde_json::json!({ "interfaces": interfaces })))
}

async fn status(
    Path(bridge): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let result = net::check_network_status(&bridge).await.map_err(internal)?;
    Ok(Json(result))
}

fn internal<E: std::fmt::Display>(e: E) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
}
