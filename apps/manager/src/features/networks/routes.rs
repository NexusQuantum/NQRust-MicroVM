use crate::features::networks::repo::NetworkRepository;
use crate::features::networks::service;
use crate::AppState;
use axum::extract::Query;
use axum::{extract::Path, http::StatusCode, Extension, Json};
use serde::{Deserialize, Serialize};
use tracing::error;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct CreateNetworkRequest {
    pub name: String,
    pub description: Option<String>,
    #[serde(rename = "type")]
    pub network_type: String, // "nat", "isolated", "bridged", or "vxlan"
    pub vlan_id: Option<i32>,
    pub host_id: Uuid,
    pub cidr: Option<String>,
    pub dhcp_enabled: Option<bool>,
    pub dhcp_range_start: Option<String>,
    pub dhcp_range_end: Option<String>,
    /// Required for bridged networks: the physical NIC to attach
    pub uplink_interface: Option<String>,
    /// Required for VXLAN networks: the gateway host that runs DHCP + NAT
    pub gateway_host_id: Option<Uuid>,
}

#[derive(Debug, Serialize)]
pub struct NetworkListItem {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    #[serde(rename = "type")]
    pub network_type: String,
    pub vlan_id: Option<i32>,
    pub vni: Option<i32>,
    pub bridge_name: String,
    pub host_id: Option<Uuid>,
    pub host_name: Option<String>,
    pub cidr: Option<String>,
    pub gateway: Option<String>,
    pub status: String,
    pub error_message: Option<String>,
    pub managed: bool,
    pub dhcp_enabled: bool,
    pub dhcp_range_start: Option<String>,
    pub dhcp_range_end: Option<String>,
    pub vm_count: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub participating_hosts: Option<i64>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize)]
pub struct NetworkListResponse {
    pub items: Vec<NetworkListItem>,
}

#[derive(Debug, Serialize)]
pub struct NetworkDetailResponse {
    pub item: NetworkListItem,
}

#[derive(Debug, Deserialize)]
pub struct UpdateNetworkRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub cidr: Option<String>,
    pub gateway: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct OkResponse {
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct SuggestQuery {
    pub host_id: Uuid,
}

fn network_to_list_item(
    network: &crate::features::networks::repo::NetworkRow,
    host_name: Option<String>,
    vm_count: i64,
    participating_hosts: Option<i64>,
) -> NetworkListItem {
    NetworkListItem {
        id: network.id,
        name: network.name.clone(),
        description: network.description.clone(),
        network_type: network.type_.clone(),
        vlan_id: network.vlan_id,
        vni: network.vni,
        bridge_name: network.bridge_name.clone(),
        host_id: network.host_id,
        host_name,
        cidr: network.cidr.clone(),
        gateway: network.gateway.clone(),
        status: network.status.clone(),
        error_message: network.error_message.clone(),
        managed: network.managed,
        dhcp_enabled: network.dhcp_enabled,
        dhcp_range_start: network.dhcp_range_start.clone(),
        dhcp_range_end: network.dhcp_range_end.clone(),
        vm_count,
        participating_hosts,
        created_at: network.created_at,
        updated_at: network.updated_at,
    }
}

#[utoipa::path(
    post,
    path = "/v1/networks",
    request_body = CreateNetworkRequest,
    responses(
        (status = 201, description = "Network created", body = NetworkDetailResponse),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Failed to create network"),
    ),
    tag = "Networks"
)]
pub async fn create(
    Extension(st): Extension<AppState>,
    Json(req): Json<CreateNetworkRequest>,
) -> Result<(StatusCode, Json<NetworkDetailResponse>), (StatusCode, Json<OkResponse>)> {
    let params = service::CreateNetworkParams {
        name: req.name,
        description: req.description,
        network_type: req.network_type,
        host_id: req.host_id,
        cidr: req.cidr,
        vlan_id: req.vlan_id,
        dhcp_enabled: req.dhcp_enabled,
        dhcp_range_start: req.dhcp_range_start,
        dhcp_range_end: req.dhcp_range_end,
        uplink_interface: req.uplink_interface,
        gateway_host_id: req.gateway_host_id,
    };

    match service::create_network(&st, params).await {
        Ok(network) => {
            let host_name = if let Some(hid) = network.host_id {
                st.hosts.get(hid).await.ok().map(|h| h.name)
            } else {
                None
            };
            let participating_hosts = if network.type_ == "vxlan" {
                Some(1)
            } else {
                None
            };
            Ok((
                StatusCode::CREATED,
                Json(NetworkDetailResponse {
                    item: network_to_list_item(&network, host_name, 0, participating_hosts),
                }),
            ))
        }
        Err(e) => {
            let msg = e.to_string();
            let status = if msg.contains("must be") || msg.contains("required") {
                StatusCode::BAD_REQUEST
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            Err((status, Json(OkResponse { message: msg })))
        }
    }
}

#[utoipa::path(
    get,
    path = "/v1/networks",
    responses(
        (status = 200, description = "List of networks", body = NetworkListResponse),
        (status = 500, description = "Failed to list networks"),
    ),
    tag = "Networks"
)]
pub async fn list(
    Extension(st): Extension<AppState>,
) -> Result<Json<NetworkListResponse>, StatusCode> {
    let network_repo = NetworkRepository::new(st.db.clone());
    let networks = network_repo.list().await.map_err(|err| {
        error!(?err, "failed to list networks");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let mut items = Vec::new();
    for network in &networks {
        let vm_count = network_repo.get_vm_count(network.id).await.unwrap_or(0);
        let host_name = if let Some(host_id) = network.host_id {
            st.hosts.get(host_id).await.ok().map(|h| h.name)
        } else {
            None
        };
        let participating_hosts = if network.type_ == "vxlan" {
            Some(
                network_repo
                    .count_network_hosts(network.id)
                    .await
                    .unwrap_or(0),
            )
        } else {
            None
        };
        items.push(network_to_list_item(
            network,
            host_name,
            vm_count,
            participating_hosts,
        ));
    }

    Ok(Json(NetworkListResponse { items }))
}

#[utoipa::path(
    get,
    path = "/v1/networks/{id}",
    responses(
        (status = 200, description = "Network details", body = NetworkDetailResponse),
        (status = 404, description = "Network not found"),
        (status = 500, description = "Failed to get network"),
    ),
    tag = "Networks"
)]
pub async fn get(
    Extension(st): Extension<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<NetworkDetailResponse>, StatusCode> {
    let network_repo = NetworkRepository::new(st.db.clone());
    let network = network_repo.get(id).await.map_err(|err| match err {
        sqlx::Error::RowNotFound => StatusCode::NOT_FOUND,
        other => {
            error!(error = ?other, "failed to get network");
            StatusCode::INTERNAL_SERVER_ERROR
        }
    })?;

    let vm_count = network_repo.get_vm_count(id).await.unwrap_or(0);
    let host_name = if let Some(host_id) = network.host_id {
        st.hosts.get(host_id).await.ok().map(|h| h.name)
    } else {
        None
    };
    let participating_hosts = if network.type_ == "vxlan" {
        Some(network_repo.count_network_hosts(id).await.unwrap_or(0))
    } else {
        None
    };

    Ok(Json(NetworkDetailResponse {
        item: network_to_list_item(&network, host_name, vm_count, participating_hosts),
    }))
}

#[utoipa::path(
    patch,
    path = "/v1/networks/{id}",
    request_body = UpdateNetworkRequest,
    responses(
        (status = 200, description = "Network updated", body = NetworkDetailResponse),
        (status = 404, description = "Network not found"),
        (status = 500, description = "Failed to update network"),
    ),
    tag = "Networks"
)]
pub async fn update(
    Extension(st): Extension<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateNetworkRequest>,
) -> Result<Json<NetworkDetailResponse>, StatusCode> {
    let network_repo = NetworkRepository::new(st.db.clone());
    let network = network_repo
        .update(
            id,
            req.name.as_deref(),
            req.description.as_deref(),
            req.cidr.as_deref(),
            req.gateway.as_deref(),
        )
        .await
        .map_err(|err| match err {
            sqlx::Error::RowNotFound => StatusCode::NOT_FOUND,
            other => {
                error!(error = ?other, "failed to update network");
                StatusCode::INTERNAL_SERVER_ERROR
            }
        })?;

    let vm_count = network_repo.get_vm_count(id).await.unwrap_or(0);
    let host_name = if let Some(host_id) = network.host_id {
        st.hosts.get(host_id).await.ok().map(|h| h.name)
    } else {
        None
    };
    let participating_hosts = if network.type_ == "vxlan" {
        Some(network_repo.count_network_hosts(id).await.unwrap_or(0))
    } else {
        None
    };

    Ok(Json(NetworkDetailResponse {
        item: network_to_list_item(&network, host_name, vm_count, participating_hosts),
    }))
}

#[utoipa::path(
    delete,
    path = "/v1/networks/{id}",
    responses(
        (status = 200, description = "Network deleted", body = OkResponse),
        (status = 404, description = "Network not found"),
        (status = 409, description = "Network has attached VMs"),
        (status = 500, description = "Failed to delete network"),
    ),
    tag = "Networks"
)]
pub async fn delete(
    Extension(st): Extension<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<OkResponse>, (StatusCode, Json<OkResponse>)> {
    match service::delete_network(&st, id).await {
        Ok(()) => Ok(Json(OkResponse {
            message: "Network deleted successfully".to_string(),
        })),
        Err(e) => {
            let msg = e.to_string();
            let status = if msg.contains("attached VMs") {
                StatusCode::CONFLICT
            } else if msg.contains("not found") {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            Err((status, Json(OkResponse { message: msg })))
        }
    }
}

#[utoipa::path(
    post,
    path = "/v1/networks/{id}/retry",
    responses(
        (status = 200, description = "Network provisioning retried", body = NetworkDetailResponse),
        (status = 400, description = "Network not in error state"),
        (status = 500, description = "Failed to retry"),
    ),
    tag = "Networks"
)]
pub async fn retry(
    Extension(st): Extension<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<NetworkDetailResponse>, (StatusCode, Json<OkResponse>)> {
    match service::retry_network(&st, id).await {
        Ok(network) => {
            let host_name = if let Some(hid) = network.host_id {
                st.hosts.get(hid).await.ok().map(|h| h.name)
            } else {
                None
            };
            let participating_hosts = if network.type_ == "vxlan" {
                let network_repo = NetworkRepository::new(st.db.clone());
                Some(
                    network_repo
                        .count_network_hosts(network.id)
                        .await
                        .unwrap_or(0),
                )
            } else {
                None
            };
            Ok(Json(NetworkDetailResponse {
                item: network_to_list_item(&network, host_name, 0, participating_hosts),
            }))
        }
        Err(e) => {
            let msg = e.to_string();
            let status = if msg.contains("only retry") {
                StatusCode::BAD_REQUEST
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            Err((status, Json(OkResponse { message: msg })))
        }
    }
}

pub async fn suggest(
    Extension(st): Extension<AppState>,
    Query(q): Query<SuggestQuery>,
) -> Result<Json<service::NetworkSuggestion>, (StatusCode, Json<OkResponse>)> {
    match service::suggest_network(&st, q.host_id).await {
        Ok(suggestion) => Ok(Json(suggestion)),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(OkResponse {
                message: e.to_string(),
            }),
        )),
    }
}

#[utoipa::path(
    get,
    path = "/v1/networks/{id}/vms",
    responses(
        (status = 200, description = "List of VM IDs on this network"),
        (status = 404, description = "Network not found"),
        (status = 500, description = "Failed to get VMs"),
    ),
    tag = "Networks"
)]
pub async fn get_vms(
    Extension(st): Extension<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let network_repo = NetworkRepository::new(st.db.clone());

    let _ = network_repo.get(id).await.map_err(|err| match err {
        sqlx::Error::RowNotFound => StatusCode::NOT_FOUND,
        other => {
            error!(error = ?other, "failed to get network");
            StatusCode::INTERNAL_SERVER_ERROR
        }
    })?;

    let vm_ids = network_repo.get_vms(id).await.map_err(|err| {
        error!(error = ?err, "failed to get VMs for network");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(serde_json::json!({ "vm_ids": vm_ids })))
}

#[derive(Debug, Deserialize)]
pub struct InterfacesQuery {
    pub host_id: Uuid,
}

pub async fn list_interfaces(
    Extension(st): Extension<AppState>,
    Query(q): Query<InterfacesQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<OkResponse>)> {
    match service::list_host_interfaces(&st, q.host_id).await {
        Ok(interfaces) => Ok(Json(serde_json::json!({ "interfaces": interfaces }))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(OkResponse {
                message: e.to_string(),
            }),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::networks::repo::NetworkRow;
    use chrono::Utc;

    fn make_row(type_: &str) -> NetworkRow {
        let now = Utc::now();
        NetworkRow {
            id: Uuid::new_v4(),
            name: "net-a".to_string(),
            description: Some("desc".to_string()),
            type_: type_.to_string(),
            vlan_id: Some(42),
            bridge_name: "nqbr1".to_string(),
            host_id: Some(Uuid::new_v4()),
            cidr: Some("10.0.2.0/24".to_string()),
            gateway: Some("10.0.2.1".to_string()),
            status: "active".to_string(),
            error_message: None,
            managed: true,
            dhcp_enabled: true,
            dhcp_range_start: Some("10.0.2.10".to_string()),
            dhcp_range_end: Some("10.0.2.250".to_string()),
            created_by_user_id: None,
            vni: None,
            uplink_interface: None,
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn network_to_list_item_maps_all_fields() {
        // Happy path: every NetworkRow field maps onto NetworkListItem and
        // the supplied auxiliary data (host_name, vm_count, participating_hosts)
        // is propagated verbatim.
        let row = make_row("nat");
        let host_name = Some("host-1".to_string());
        let item = network_to_list_item(&row, host_name.clone(), 7, None);

        assert_eq!(item.id, row.id);
        assert_eq!(item.name, row.name);
        assert_eq!(item.description, row.description);
        assert_eq!(item.network_type, row.type_);
        assert_eq!(item.vlan_id, row.vlan_id);
        assert_eq!(item.vni, row.vni);
        assert_eq!(item.bridge_name, row.bridge_name);
        assert_eq!(item.host_id, row.host_id);
        assert_eq!(item.host_name, host_name);
        assert_eq!(item.cidr, row.cidr);
        assert_eq!(item.gateway, row.gateway);
        assert_eq!(item.status, row.status);
        assert_eq!(item.managed, row.managed);
        assert_eq!(item.dhcp_enabled, row.dhcp_enabled);
        assert_eq!(item.dhcp_range_start, row.dhcp_range_start);
        assert_eq!(item.dhcp_range_end, row.dhcp_range_end);
        assert_eq!(item.vm_count, 7);
        assert_eq!(item.participating_hosts, None);
        assert_eq!(item.created_at, row.created_at);
        assert_eq!(item.updated_at, row.updated_at);
    }

    #[test]
    fn network_to_list_item_includes_participating_hosts_for_vxlan() {
        // VXLAN networks track participating hosts; non-VXLAN networks set None.
        // Here we just verify the helper passes the value through unchanged.
        let row = make_row("vxlan");
        let item = network_to_list_item(&row, None, 0, Some(3));
        assert_eq!(item.participating_hosts, Some(3));
        assert_eq!(item.network_type, "vxlan");
        assert!(item.host_name.is_none());
    }
}
