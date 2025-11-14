use crate::features::networks::repo::NetworkRepository;
use crate::AppState;
use axum::{extract::Path, http::StatusCode, Extension, Json};
use serde::{Deserialize, Serialize};
use tracing::error;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct CreateNetworkRequest {
    pub name: String,
    pub description: Option<String>,
    #[serde(rename = "type")]
    pub network_type: String, // "bridge" or "vlan"
    pub vlan_id: Option<i32>,
    pub bridge_name: String,
    pub host_id: Uuid,
    pub cidr: Option<String>,
    pub gateway: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CreateNetworkResponse {
    pub id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct NetworkListItem {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    #[serde(rename = "type")]
    pub network_type: String,
    pub vlan_id: Option<i32>,
    pub bridge_name: String,
    pub host_id: Option<Uuid>,
    pub host_name: Option<String>,
    pub cidr: Option<String>,
    pub gateway: Option<String>,
    pub vm_count: i64,
    pub created_at: chrono::DateTime<chrono::Utc>,
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

#[utoipa::path(
    post,
    path = "/v1/networks",
    request_body = CreateNetworkRequest,
    responses(
        (status = 200, description = "Network created", body = CreateNetworkResponse),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Failed to create network"),
    ),
    tag = "Networks"
)]
pub async fn create(
    Extension(st): Extension<AppState>,
    Json(req): Json<CreateNetworkRequest>,
) -> Result<Json<CreateNetworkResponse>, StatusCode> {
    // Validate network type
    if req.network_type != "bridge" && req.network_type != "vlan" {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Validate VLAN ID if type is vlan
    if req.network_type == "vlan" {
        if req.vlan_id.is_none() {
            return Err(StatusCode::BAD_REQUEST);
        }
        if let Some(vlan_id) = req.vlan_id {
            if vlan_id < 1 || vlan_id > 4094 {
                return Err(StatusCode::BAD_REQUEST);
            }
        }
    }

    let network_repo = NetworkRepository::new(st.db.clone());
    let network = network_repo
        .create(
            &req.name,
            req.description.as_deref(),
            &req.network_type,
            req.vlan_id,
            &req.bridge_name,
            req.host_id,
            req.cidr.as_deref(),
            req.gateway.as_deref(),
        )
        .await
        .map_err(|err| {
            error!(?err, "failed to create network");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(CreateNetworkResponse { id: network.id }))
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
    for network in networks {
        let vm_count = network_repo.get_vm_count(network.id).await.unwrap_or(0);

        // Get host name if host_id is present
        let host_name = if let Some(host_id) = network.host_id {
            st.hosts.get(host_id).await.ok().map(|h| h.name)
        } else {
            None
        };

        items.push(NetworkListItem {
            id: network.id,
            name: network.name,
            description: network.description,
            network_type: network.type_,
            vlan_id: network.vlan_id,
            bridge_name: network.bridge_name,
            host_id: network.host_id,
            host_name,
            cidr: network.cidr,
            gateway: network.gateway,
            vm_count,
            created_at: network.created_at,
        });
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

    // Get host name
    let host_name = if let Some(host_id) = network.host_id {
        st.hosts.get(host_id).await.ok().map(|h| h.name)
    } else {
        None
    };

    Ok(Json(NetworkDetailResponse {
        item: NetworkListItem {
            id: network.id,
            name: network.name,
            description: network.description,
            network_type: network.type_,
            vlan_id: network.vlan_id,
            bridge_name: network.bridge_name,
            host_id: network.host_id,
            host_name,
            cidr: network.cidr,
            gateway: network.gateway,
            vm_count,
            created_at: network.created_at,
        },
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

    Ok(Json(NetworkDetailResponse {
        item: NetworkListItem {
            id: network.id,
            name: network.name,
            description: network.description,
            network_type: network.type_,
            vlan_id: network.vlan_id,
            bridge_name: network.bridge_name,
            host_id: network.host_id,
            host_name,
            cidr: network.cidr,
            gateway: network.gateway,
            vm_count,
            created_at: network.created_at,
        },
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
) -> Result<Json<OkResponse>, StatusCode> {
    let network_repo = NetworkRepository::new(st.db.clone());

    // Check if network has any VMs attached
    let vm_count = network_repo.get_vm_count(id).await.unwrap_or(0);
    if vm_count > 0 {
        return Err(StatusCode::CONFLICT);
    }

    network_repo.delete(id).await.map_err(|err| match err {
        sqlx::Error::RowNotFound => StatusCode::NOT_FOUND,
        other => {
            error!(error = ?other, "failed to delete network");
            StatusCode::INTERNAL_SERVER_ERROR
        }
    })?;

    Ok(Json(OkResponse {
        message: "Network deleted successfully".to_string(),
    }))
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

    // Verify network exists
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
