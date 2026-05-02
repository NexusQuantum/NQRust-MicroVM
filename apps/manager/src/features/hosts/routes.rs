use crate::features::hosts::repo::HostRow;
use crate::AppState;
use axum::{extract::Path, http::StatusCode, Extension, Json};
use chrono::{DateTime, Utc};
use nexus_types::{
    HostHeartbeatRequest, HostPathParams, OkResponse, RegisterHostRequest, RegisterHostResponse,
};
use serde::{Deserialize, Serialize};
use tracing::error;
use uuid::Uuid;

/// Health status thresholds. A host is "healthy" if its last heartbeat was
/// within `HEALTHY_THRESHOLD_SECONDS`, "degraded" if within
/// `DEGRADED_THRESHOLD_SECONDS`, and "offline" otherwise.
pub(crate) const HEALTHY_THRESHOLD_SECONDS: i64 = 30;
pub(crate) const DEGRADED_THRESHOLD_SECONDS: i64 = 5 * 60;

/// Pure-logic helper: compute the health status string for a host from its
/// last-seen timestamp and a reference "now".
pub(crate) fn compute_host_status(last_seen_at: DateTime<Utc>, now: DateTime<Utc>) -> &'static str {
    if last_seen_at > now - chrono::Duration::seconds(HEALTHY_THRESHOLD_SECONDS) {
        "healthy"
    } else if last_seen_at > now - chrono::Duration::seconds(DEGRADED_THRESHOLD_SECONDS) {
        "degraded"
    } else {
        "offline"
    }
}

/// Pure-logic helper: extract the metric tuple `(cpus, total_memory_mb,
/// total_disk_gb, used_disk_gb)` from a capabilities JSON blob. Returns
/// `None` if any of the four numeric fields are missing or not an integer.
pub(crate) fn extract_host_metrics(caps: &serde_json::Value) -> Option<(i32, i64, i64, i64)> {
    let cpus = caps
        .get("cpus")
        .and_then(|v| v.as_i64())
        .map(|v| v as i32)?;
    let memory = caps.get("total_memory_mb").and_then(|v| v.as_i64())?;
    let total_disk = caps.get("total_disk_gb").and_then(|v| v.as_i64())?;
    let used_disk = caps.get("used_disk_gb").and_then(|v| v.as_i64())?;
    Some((cpus, memory, total_disk, used_disk))
}

/// Pure-logic helper: convert a `HostRow` into a `HostListItem`, given a
/// pre-computed status string and VM count.
pub(crate) fn host_row_to_list_item(row: HostRow, status: &str, vm_count: i64) -> HostListItem {
    HostListItem {
        id: row.id,
        name: row.name,
        addr: row.addr,
        status: status.to_string(),
        capabilities_json: row.capabilities_json,
        total_cpus: row.total_cpus,
        total_memory_mb: row.total_memory_mb,
        total_disk_gb: row.total_disk_gb,
        used_disk_gb: row.used_disk_gb,
        vm_count,
        last_seen_at: row.last_seen_at,
        last_metrics_at: row.last_metrics_at,
        is_hot_spare: row.is_hot_spare,
        lifecycle_state: row.lifecycle_state,
        lifecycle_changed_at: row.lifecycle_changed_at,
        spdk_backend_id: row.spdk_backend_id,
    }
}

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
        supported_backend_kinds,
    } = req;

    let row = st
        .hosts
        .register(&name, &addr, capabilities)
        .await
        .map_err(|err| {
            error!(?err, "failed to register host");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    if let Some(kinds) = supported_backend_kinds {
        if let Err(err) = st.hosts.update_supported_backend_kinds(row.id, kinds).await {
            error!(
                ?err,
                "failed to update host supported_backend_kinds on register"
            );
        }
    }

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
        if let Some((cpus, memory, total_disk, used_disk)) = extract_host_metrics(&caps) {
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

    if let Some(kinds) = req.supported_backend_kinds {
        if let Err(err) = st.hosts.update_supported_backend_kinds(id, kinds).await {
            error!(
                ?err,
                "failed to update host supported_backend_kinds on heartbeat"
            );
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
    /// B-III Task 5: hot-spare reserved for failure recovery.
    pub is_hot_spare: bool,
    /// B-III Task 6: `active`, `draining`, `decommissioned`.
    pub lifecycle_state: String,
    pub lifecycle_changed_at: Option<chrono::DateTime<chrono::Utc>>,
    /// B-III follow-up: SPDK lvol bdev id used for raft_spdk replicas.
    /// `None` means the host is not a raft_spdk placement target.
    pub spdk_backend_id: Option<Uuid>,
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
        let status = compute_host_status(host.last_seen_at, chrono::Utc::now());

        // Get VM count for this host
        let vm_count = st.hosts.get_vm_count(host.id).await.unwrap_or(0);

        items.push(host_row_to_list_item(host, status, vm_count));
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
    let status = compute_host_status(host.last_seen_at, chrono::Utc::now());

    // Get VM count
    let vm_count = st.hosts.get_vm_count(id).await.unwrap_or(0);

    Ok(Json(HostDetailResponse {
        item: host_row_to_list_item(host, status, vm_count),
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

#[derive(Debug, Clone, Deserialize)]
pub struct SetHotSpareRequest {
    pub is_hot_spare: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SetSpdkBackendIdRequest {
    /// `None` clears the host's raft_spdk placement eligibility.
    pub spdk_backend_id: Option<Uuid>,
}

/// B-III follow-up: set the host's SPDK lvol bdev id. Operators run this
/// once per host that should host raft_spdk replicas; the planner reads
/// the column when emitting `add_replica` plans so the operator no
/// longer has to thread `--spdk-backend-id` through every CLI call.
#[utoipa::path(
    post,
    path = "/v1/hosts/{id}/spdk_backend_id",
    params(("id" = uuid::Uuid, Path, description = "Host id")),
    request_body = SetSpdkBackendIdRequest,
    responses(
        (status = 200, description = "Updated host", body = HostDetailResponse),
        (status = 404, description = "Host not found"),
    ),
    tag = "Hosts"
)]
pub async fn set_spdk_backend_id(
    Extension(st): Extension<AppState>,
    Path(HostPathParams { id }): Path<HostPathParams>,
    Json(req): Json<SetSpdkBackendIdRequest>,
) -> Result<Json<HostDetailResponse>, StatusCode> {
    let row = st
        .hosts
        .set_spdk_backend_id(id, req.spdk_backend_id)
        .await
        .map_err(|err| match err {
            sqlx::Error::RowNotFound => StatusCode::NOT_FOUND,
            other => {
                error!(error = ?other, "set_spdk_backend_id failed");
                StatusCode::INTERNAL_SERVER_ERROR
            }
        })?;
    let vm_count = st.hosts.get_vm_count(id).await.unwrap_or(0);
    let status = compute_host_status(row.last_seen_at, chrono::Utc::now());
    Ok(Json(HostDetailResponse {
        item: host_row_to_list_item(row, status, vm_count),
    }))
}

/// B-III Task 5: toggle hot-spare flag.
#[utoipa::path(
    post,
    path = "/v1/hosts/{id}/hot_spare",
    params(("id" = uuid::Uuid, Path, description = "Host id")),
    request_body = SetHotSpareRequest,
    responses(
        (status = 200, description = "Updated host", body = HostDetailResponse),
        (status = 404, description = "Host not found"),
    ),
    tag = "Hosts"
)]
pub async fn set_hot_spare(
    Extension(st): Extension<AppState>,
    Path(HostPathParams { id }): Path<HostPathParams>,
    Json(req): Json<SetHotSpareRequest>,
) -> Result<Json<HostDetailResponse>, StatusCode> {
    let row = st
        .hosts
        .set_hot_spare(id, req.is_hot_spare)
        .await
        .map_err(|err| match err {
            sqlx::Error::RowNotFound => StatusCode::NOT_FOUND,
            other => {
                error!(error = ?other, "set_hot_spare failed");
                StatusCode::INTERNAL_SERVER_ERROR
            }
        })?;
    let vm_count = st.hosts.get_vm_count(id).await.unwrap_or(0);
    let status = compute_host_status(row.last_seen_at, chrono::Utc::now());
    Ok(Json(HostDetailResponse {
        item: host_row_to_list_item(row, status, vm_count),
    }))
}

/// B-III Task 6: begin host decommission. Transitions the host to
/// `draining`. The host stops accepting new placement immediately;
/// existing replicas are not yet drained — that's the decommission
/// reconciler's job (Task 7) once it lands. Refuses if the host hosts
/// raft_spdk replicas and no hot-spare is available, so an operator
/// notices the placement constraint up front.
#[utoipa::path(
    post,
    path = "/v1/hosts/{id}/decommission",
    params(("id" = uuid::Uuid, Path, description = "Host id")),
    responses(
        (status = 200, description = "Host now draining", body = HostDetailResponse),
        (status = 404, description = "Host not found"),
        (status = 409, description = "Refused: hosts raft_spdk replicas and no hot-spare available"),
    ),
    tag = "Hosts"
)]
pub async fn decommission(
    Extension(st): Extension<AppState>,
    Path(HostPathParams { id }): Path<HostPathParams>,
) -> Result<Json<HostDetailResponse>, StatusCode> {
    // Pre-flight: if this host backs any raft_spdk replicas, require at
    // least one healthy hot-spare. Without that, draining the host would
    // drop one or more groups below quorum on remove.
    let raft_replica_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM raft_spdk_replica r
          JOIN host h ON h.addr = SPLIT_PART(r.agent_base_url, '/v1/raft_block', 1)
         WHERE h.id = $1
           AND r.removed_at IS NULL
        "#,
    )
    .bind(id)
    .fetch_one(&st.db)
    .await
    .unwrap_or(0);
    if raft_replica_count > 0 {
        let spares = st.hosts.list_hot_spares().await.map_err(|err| {
            error!(error = ?err, "list_hot_spares failed");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
        if spares.is_empty() {
            return Err(StatusCode::CONFLICT);
        }
    }

    let row = st
        .hosts
        .set_lifecycle(id, "draining")
        .await
        .map_err(|err| match err {
            sqlx::Error::RowNotFound => StatusCode::NOT_FOUND,
            other => {
                error!(error = ?other, "set_lifecycle(draining) failed");
                StatusCode::INTERNAL_SERVER_ERROR
            }
        })?;
    let vm_count = st.hosts.get_vm_count(id).await.unwrap_or(0);
    let status = compute_host_status(row.last_seen_at, chrono::Utc::now());
    Ok(Json(HostDetailResponse {
        item: host_row_to_list_item(row, status, vm_count),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{extract::Path, Extension};
    use chrono::Utc;
    use serde_json::json;

    async fn test_registry(pool: &sqlx::PgPool) -> crate::features::storage::registry::Registry {
        crate::features::storage::registry::Registry::load(pool, None)
            .await
            .expect("registry")
    }

    fn sample_row(last_seen_at: chrono::DateTime<Utc>) -> HostRow {
        HostRow {
            id: Uuid::new_v4(),
            name: "agent-x".into(),
            addr: "http://10.0.0.5:9090".into(),
            capabilities_json: json!({"cpus": 4}),
            last_seen_at,
            total_cpus: Some(8),
            total_memory_mb: Some(16_384),
            total_disk_gb: Some(500),
            used_disk_gb: Some(120),
            last_metrics_at: Some(last_seen_at),
            is_hot_spare: false,
            lifecycle_state: "active".into(),
            lifecycle_changed_at: None,
            spdk_backend_id: None,
        }
    }

    // --- compute_host_status ---

    #[test]
    fn compute_host_status_recent_is_healthy() {
        let now = Utc::now();
        let last_seen = now - chrono::Duration::seconds(5);
        assert_eq!(compute_host_status(last_seen, now), "healthy");
    }

    #[test]
    fn compute_host_status_within_degraded_window_is_degraded() {
        let now = Utc::now();
        // Just past the 30s healthy threshold but well within the 5min window.
        let last_seen = now - chrono::Duration::seconds(60);
        assert_eq!(compute_host_status(last_seen, now), "degraded");
    }

    #[test]
    fn compute_host_status_old_is_offline() {
        let now = Utc::now();
        let last_seen = now - chrono::Duration::hours(1);
        assert_eq!(compute_host_status(last_seen, now), "offline");
    }

    #[test]
    fn compute_host_status_at_healthy_boundary_is_degraded() {
        // The original logic uses strict `>`: a last_seen exactly at the
        // boundary falls through to the next bucket. This pins that
        // behavior so the upcoming refactor cannot silently flip it.
        let now = Utc::now();
        let last_seen = now - chrono::Duration::seconds(HEALTHY_THRESHOLD_SECONDS);
        assert_eq!(compute_host_status(last_seen, now), "degraded");

        let last_seen_offline = now - chrono::Duration::seconds(DEGRADED_THRESHOLD_SECONDS);
        assert_eq!(compute_host_status(last_seen_offline, now), "offline");
    }

    // --- extract_host_metrics ---

    #[test]
    fn extract_host_metrics_full_payload() {
        let caps = json!({
            "cpus": 4,
            "total_memory_mb": 16384,
            "total_disk_gb": 500,
            "used_disk_gb": 120,
            "extra": "ignored",
        });
        assert_eq!(extract_host_metrics(&caps), Some((4, 16_384, 500, 120)));
    }

    #[test]
    fn extract_host_metrics_returns_none_when_field_missing() {
        // Missing `used_disk_gb` -> the whole tuple is rejected.
        let caps = json!({
            "cpus": 4,
            "total_memory_mb": 16384,
            "total_disk_gb": 500,
        });
        assert!(extract_host_metrics(&caps).is_none());
    }

    #[test]
    fn extract_host_metrics_returns_none_for_wrong_types() {
        // Strings instead of numbers must not be accepted.
        let caps = json!({
            "cpus": "4",
            "total_memory_mb": 16384,
            "total_disk_gb": 500,
            "used_disk_gb": 120,
        });
        assert!(extract_host_metrics(&caps).is_none());

        // Empty object
        assert!(extract_host_metrics(&json!({})).is_none());
    }

    // --- host_row_to_list_item ---

    #[test]
    fn host_row_to_list_item_preserves_fields_and_status() {
        let now = Utc::now();
        let row = sample_row(now);
        let id = row.id;
        let item = host_row_to_list_item(row, "healthy", 7);
        assert_eq!(item.id, id);
        assert_eq!(item.name, "agent-x");
        assert_eq!(item.addr, "http://10.0.0.5:9090");
        assert_eq!(item.status, "healthy");
        assert_eq!(item.total_cpus, Some(8));
        assert_eq!(item.total_memory_mb, Some(16_384));
        assert_eq!(item.total_disk_gb, Some(500));
        assert_eq!(item.used_disk_gb, Some(120));
        assert_eq!(item.vm_count, 7);
        assert_eq!(item.last_seen_at, now);
        assert_eq!(item.capabilities_json, json!({"cpus": 4}));
    }

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
        let registry = test_registry(&pool).await;
        let state = crate::AppState {
            db: pool.clone(),
            hosts: repo.clone(),
            images,
            snapshots,
            users,
            shell_repo,
            licensing: crate::features::licensing::repo::LicensingRepository::new(pool.clone()),
            allow_direct_image_paths: true,
            storage,
            registry,
            download_progress,
            license_state: std::sync::Arc::new(tokio::sync::RwLock::new(
                nexus_types::LicenseState::default(),
            )),
            license_config: crate::features::licensing::license_service::LicenseConfig::from_env(),
            sso_providers: crate::features::sso::repo::SsoProviderRepository::new(pool.clone()),
            user_identities: crate::features::sso::repo::UserIdentityRepository::new(pool.clone()),
            auth_states: crate::features::sso::repo::AuthStateRepository::new(pool.clone()),
            sso_base_url: "http://localhost:18080".to_string(),
            sso_frontend_url: "http://localhost:3000".to_string(),
            sso_encryption_key: crate::features::sso::crypto::derive_key("test-key"),
        };

        let req = RegisterHostRequest {
            name: "agent-1".into(),
            addr: "http://127.0.0.1:9090".into(),
            capabilities: json!({"cpus": 4}),
            supported_backend_kinds: None,
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
        let registry = test_registry(&pool).await;
        let state = crate::AppState {
            db: pool.clone(),
            hosts: repo.clone(),
            images,
            snapshots,
            users,
            shell_repo,
            licensing: crate::features::licensing::repo::LicensingRepository::new(pool.clone()),
            allow_direct_image_paths: true,
            storage,
            registry,
            download_progress,
            license_state: std::sync::Arc::new(tokio::sync::RwLock::new(
                nexus_types::LicenseState::default(),
            )),
            license_config: crate::features::licensing::license_service::LicenseConfig::from_env(),
            sso_providers: crate::features::sso::repo::SsoProviderRepository::new(pool.clone()),
            user_identities: crate::features::sso::repo::UserIdentityRepository::new(pool.clone()),
            auth_states: crate::features::sso::repo::AuthStateRepository::new(pool.clone()),
            sso_base_url: "http://localhost:18080".to_string(),
            sso_frontend_url: "http://localhost:3000".to_string(),
            sso_encryption_key: crate::features::sso::crypto::derive_key("test-key"),
        };

        let req = RegisterHostRequest {
            name: "agent-2".into(),
            addr: "http://127.0.0.1:9191".into(),
            capabilities: json!({}),
            supported_backend_kinds: None,
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
                supported_backend_kinds: None,
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
