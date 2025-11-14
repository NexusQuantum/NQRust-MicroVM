use crate::AppState;
use axum::{extract::Path, http::StatusCode, Extension, Json};
use nexus_types::{
    HostHeartbeatRequest, HostPathParams, OkResponse, RegisterHostRequest, RegisterHostResponse,
};
use serde::Serialize;
use tracing::error;
use uuid::Uuid;

#[utoipa::path(
    post,
    path = "/v1/hosts/register",
    request_body = RegisterHostRequest,
    responses(
        (status = 200, description = "Host registered", body = RegisterHostResponse),
        (status = 500, description = "Failed to register host"),
    ),
    tag = "Hosts"
)]
pub async fn register(
    Extension(st): Extension<AppState>,
    Json(req): Json<RegisterHostRequest>,
) -> Result<Json<RegisterHostResponse>, StatusCode> {
    let RegisterHostRequest {
        name,
        addr,
        capabilities,
    } = req;

    let row = st
        .hosts
        .register(&name, &addr, capabilities)
        .await
        .map_err(|err| {
            error!(?err, "failed to register host");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(RegisterHostResponse { id: row.id }))
}

#[utoipa::path(
    post,
    path = "/v1/hosts/{id}/heartbeat",
    params(HostPathParams),
    request_body = HostHeartbeatRequest,
    responses(
        (status = 200, description = "Heartbeat recorded", body = OkResponse),
        (status = 404, description = "Host not found"),
        (status = 500, description = "Failed to record heartbeat"),
    ),
    tag = "Hosts"
)]
pub async fn heartbeat(
    Extension(st): Extension<AppState>,
    Path(HostPathParams { id }): Path<HostPathParams>,
    Json(req): Json<HostHeartbeatRequest>,
) -> Result<Json<OkResponse>, StatusCode> {
    // Update heartbeat timestamp and capabilities
    st.hosts
        .heartbeat(id, req.capabilities.clone())
        .await
        .map_err(|err| match err {
            sqlx::Error::RowNotFound => StatusCode::NOT_FOUND,
            other => {
                error!(error = ?other, "failed to record host heartbeat");
                StatusCode::INTERNAL_SERVER_ERROR
            }
        })?;

    // Extract and save metrics if present in capabilities
    if let Some(caps) = req.capabilities {
        if let (Some(cpus), Some(memory), Some(total_disk), Some(used_disk)) = (
            caps.get("cpus").and_then(|v| v.as_i64()).map(|v| v as i32),
            caps.get("total_memory_mb").and_then(|v| v.as_i64()),
            caps.get("total_disk_gb").and_then(|v| v.as_i64()),
            caps.get("used_disk_gb").and_then(|v| v.as_i64()),
        ) {
            // Update metrics in database
            if let Err(err) = st
                .hosts
                .update_metrics(id, cpus, memory, total_disk, used_disk)
                .await
            {
                error!(error = ?err, "failed to update host metrics");
                // Don't fail the heartbeat if metrics update fails
            }
        }
    }

    Ok(Json(OkResponse::default()))
}

#[derive(Debug, Clone, Serialize)]
pub struct HostListItem {
    pub id: Uuid,
    pub name: String,
    pub addr: String,
    pub status: String, // "healthy", "degraded", "offline"
    pub capabilities_json: serde_json::Value,
    pub total_cpus: Option<i32>,
    pub total_memory_mb: Option<i64>,
    pub total_disk_gb: Option<i64>,
    pub used_disk_gb: Option<i64>,
    pub vm_count: i64,
    pub last_seen_at: chrono::DateTime<chrono::Utc>,
    pub last_metrics_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct HostListResponse {
    pub items: Vec<HostListItem>,
}

#[derive(Debug, Clone, Serialize)]
pub struct HostDetailResponse {
    pub item: HostListItem,
}

#[utoipa::path(
    get,
    path = "/v1/hosts",
    responses(
        (status = 200, description = "List of all hosts", body = HostListResponse),
        (status = 500, description = "Failed to list hosts"),
    ),
    tag = "Hosts"
)]
pub async fn list(
    Extension(st): Extension<AppState>,
) -> Result<Json<HostListResponse>, StatusCode> {
    let hosts = st.hosts.list_all().await.map_err(|err| {
        error!(?err, "failed to list hosts");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let mut items = Vec::new();
    for host in hosts {
        // Determine health status based on last_seen_at
        let status = if host.last_seen_at > chrono::Utc::now() - chrono::Duration::seconds(30) {
            "healthy"
        } else if host.last_seen_at > chrono::Utc::now() - chrono::Duration::minutes(5) {
            "degraded"
        } else {
            "offline"
        };

        // Get VM count for this host
        let vm_count = st.hosts.get_vm_count(host.id).await.unwrap_or(0);

        items.push(HostListItem {
            id: host.id,
            name: host.name,
            addr: host.addr,
            status: status.to_string(),
            capabilities_json: host.capabilities_json,
            total_cpus: host.total_cpus,
            total_memory_mb: host.total_memory_mb,
            total_disk_gb: host.total_disk_gb,
            used_disk_gb: host.used_disk_gb,
            vm_count,
            last_seen_at: host.last_seen_at,
            last_metrics_at: host.last_metrics_at,
        });
    }

    Ok(Json(HostListResponse { items }))
}

#[utoipa::path(
    get,
    path = "/v1/hosts/{id}",
    params(HostPathParams),
    responses(
        (status = 200, description = "Host details", body = HostDetailResponse),
        (status = 404, description = "Host not found"),
        (status = 500, description = "Failed to get host"),
    ),
    tag = "Hosts"
)]
pub async fn get(
    Extension(st): Extension<AppState>,
    Path(HostPathParams { id }): Path<HostPathParams>,
) -> Result<Json<HostDetailResponse>, StatusCode> {
    let host = st.hosts.get(id).await.map_err(|err| match err {
        sqlx::Error::RowNotFound => StatusCode::NOT_FOUND,
        other => {
            error!(error = ?other, "failed to get host");
            StatusCode::INTERNAL_SERVER_ERROR
        }
    })?;

    // Determine health status
    let status = if host.last_seen_at > chrono::Utc::now() - chrono::Duration::seconds(30) {
        "healthy"
    } else if host.last_seen_at > chrono::Utc::now() - chrono::Duration::minutes(5) {
        "degraded"
    } else {
        "offline"
    };

    // Get VM count
    let vm_count = st.hosts.get_vm_count(id).await.unwrap_or(0);

    Ok(Json(HostDetailResponse {
        item: HostListItem {
            id: host.id,
            name: host.name,
            addr: host.addr,
            status: status.to_string(),
            capabilities_json: host.capabilities_json,
            total_cpus: host.total_cpus,
            total_memory_mb: host.total_memory_mb,
            total_disk_gb: host.total_disk_gb,
            used_disk_gb: host.used_disk_gb,
            vm_count,
            last_seen_at: host.last_seen_at,
            last_metrics_at: host.last_metrics_at,
        },
    }))
}

#[utoipa::path(
    delete,
    path = "/v1/hosts/{id}",
    params(HostPathParams),
    responses(
        (status = 200, description = "Host deleted", body = OkResponse),
        (status = 400, description = "Cannot delete alive host"),
        (status = 404, description = "Host not found"),
        (status = 500, description = "Failed to delete host"),
    ),
    tag = "Hosts"
)]
pub async fn delete(
    Extension(st): Extension<AppState>,
    Path(HostPathParams { id }): Path<HostPathParams>,
) -> Result<Json<OkResponse>, StatusCode> {
    // Check if host exists
    st.hosts.get(id).await.map_err(|err| match err {
        sqlx::Error::RowNotFound => StatusCode::NOT_FOUND,
        other => {
            error!(error = ?other, "failed to get host");
            StatusCode::INTERNAL_SERVER_ERROR
        }
    })?;

    // Check if host is alive (last_seen_at within 30 seconds)
    let is_alive = st.hosts.is_alive(id).await.map_err(|err| {
        error!(error = ?err, "failed to check host status");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    if is_alive {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Delete the host (only dead hosts will be deleted due to SQL constraint)
    st.hosts.delete(id).await.map_err(|err| {
        error!(error = ?err, "failed to delete host");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(OkResponse::default()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{extract::Path, Extension};
    use serde_json::json;

    #[ignore]
    #[sqlx::test(migrations = "./migrations")]
    async fn register_creates_host(pool: sqlx::PgPool) {
        let repo = crate::features::hosts::repo::HostRepository::new(pool.clone());
        let images =
            crate::features::images::repo::ImageRepository::new(pool.clone(), "/srv/images");
        let snapshots = crate::features::snapshots::repo::SnapshotRepository::new(pool.clone());
        let storage = crate::features::storage::LocalStorage::new();
        storage.init().await.unwrap();
        let users = crate::features::users::repo::UserRepository::new(pool.clone());
        let shell_repo = crate::features::vms::shell::ShellRepository::new(pool.clone());
        let download_progress =
            std::sync::Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new()));
        let state = crate::AppState {
            db: pool.clone(),
            hosts: repo.clone(),
            images,
            snapshots,
            users,
            shell_repo,
            allow_direct_image_paths: true,
            storage,
            download_progress,
        };

        let req = RegisterHostRequest {
            name: "agent-1".into(),
            addr: "http://127.0.0.1:9090".into(),
            capabilities: json!({"cpus": 4}),
        };

        let Json(response) = super::register(Extension(state), Json(req)).await.unwrap();
        let stored = repo.get(response.id).await.unwrap();
        assert_eq!(stored.name, "agent-1");
        assert_eq!(stored.addr, "http://127.0.0.1:9090");
        assert_eq!(stored.capabilities_json, json!({"cpus": 4}));
    }

    #[ignore]
    #[sqlx::test(migrations = "./migrations")]
    async fn heartbeat_updates_last_seen_and_capabilities(pool: sqlx::PgPool) {
        let repo = crate::features::hosts::repo::HostRepository::new(pool.clone());
        let images =
            crate::features::images::repo::ImageRepository::new(pool.clone(), "/srv/images");
        let snapshots = crate::features::snapshots::repo::SnapshotRepository::new(pool.clone());
        let storage = crate::features::storage::LocalStorage::new();
        storage.init().await.unwrap();
        let users = crate::features::users::repo::UserRepository::new(pool.clone());
        let shell_repo = crate::features::vms::shell::ShellRepository::new(pool.clone());
        let download_progress =
            std::sync::Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new()));
        let state = crate::AppState {
            db: pool.clone(),
            hosts: repo.clone(),
            images,
            snapshots,
            users,
            shell_repo,
            allow_direct_image_paths: true,
            storage,
            download_progress,
        };

        let req = RegisterHostRequest {
            name: "agent-2".into(),
            addr: "http://127.0.0.1:9191".into(),
            capabilities: json!({}),
        };

        let Json(register_resp) = super::register(Extension(state.clone()), Json(req))
            .await
            .unwrap();

        sqlx::query("UPDATE host SET last_seen_at = now() - interval '1 hour' WHERE id=$1")
            .bind(register_resp.id)
            .execute(repo.pool())
            .await
            .unwrap();

        let before = repo.get(register_resp.id).await.unwrap();

        let Json(response) = super::heartbeat(
            Extension(state),
            Path(HostPathParams {
                id: register_resp.id,
            }),
            Json(HostHeartbeatRequest {
                capabilities: Some(json!({"memory": 8192})),
            }),
        )
        .await
        .unwrap();

        assert_eq!(response, OkResponse::default());

        let after = repo.get(register_resp.id).await.unwrap();
        assert!(after.last_seen_at > before.last_seen_at);
        assert_eq!(after.capabilities_json, json!({"memory": 8192}));
    }
}
