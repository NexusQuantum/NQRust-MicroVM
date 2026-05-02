use crate::features::storage_backends::repo::{StorageBackendRepository, StorageBackendRow};
use crate::AppState;
use axum::{
    extract::{Path, Query},
    http::StatusCode,
    response::IntoResponse,
    Extension, Json,
};
use chrono::{DateTime, Utc};
use nexus_storage::{RaftBlockStoreKind, RaftSpdkLocator, RaftSpdkReplicaLocator};
use nexus_types::{BackendKind, Capabilities, StorageBackend};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::{BTreeSet, HashMap};
use std::time::{Duration, Instant};
use tokio::time::sleep;
use utoipa::ToSchema;
use uuid::Uuid;

const REPAIR_CATCHUP_TIMEOUT: Duration = Duration::from_secs(300);
const REPAIR_CATCHUP_POLL_INTERVAL: Duration = Duration::from_secs(1);

fn row_to_wire(row: StorageBackendRow) -> Result<StorageBackend, StatusCode> {
    let kind: BackendKind = serde_json::from_value(serde_json::Value::String(row.kind.clone()))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let capabilities: Capabilities = match serde_json::from_value(row.capabilities_json) {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(
                "storage_backend '{}' has malformed capabilities_json; using default: {e}",
                row.name
            );
            Capabilities::default()
        }
    };
    Ok(StorageBackend {
        id: row.id,
        name: row.name,
        kind,
        capabilities,
        is_default: row.is_default,
        created_at: row.created_at,
        deleted_at: row.deleted_at,
    })
}

#[derive(serde::Serialize, ToSchema)]
pub struct StorageBackendListResponse {
    pub items: Vec<StorageBackend>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RaftSpdkGroupListItem {
    pub group_id: Uuid,
    pub volume_id: Uuid,
    pub size_bytes: u64,
    pub block_size: u64,
    pub replica_count: usize,
    pub leader_hint: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RaftSpdkGroupListResponse {
    pub items: Vec<RaftSpdkGroupListItem>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RaftSpdkQuorumState {
    LeaderSteady,
    Electing,
    QuorumLost,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaftBlockReplicaStatus {
    pub group_id: Uuid,
    pub state: String,
    pub data_path: String,
    pub transport: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raft_state: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_term: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_leader: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_log_index: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub millis_since_quorum_ack: Option<u64>,
    pub store_kind: RaftBlockStoreKind,
    pub store_path: Option<String>,
    pub node_id: Option<u64>,
    pub capacity_bytes: Option<u64>,
    pub block_size: Option<u64>,
    pub last_applied_index: Option<u64>,
    pub compacted_through: Option<u64>,
    pub retained_log_entries: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct RaftSpdkReplicaStatusItem {
    pub node_id: u64,
    pub agent_base_url: String,
    pub healthy: bool,
    pub status: Option<RaftBlockReplicaStatus>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RaftSpdkGroupStatusResponse {
    pub group_id: Uuid,
    pub size_bytes: u64,
    pub block_size: u64,
    pub leader_hint: Option<u64>,
    pub observed_leader: Option<u64>,
    pub quorum_state: RaftSpdkQuorumState,
    pub lagging_followers: Vec<u64>,
    pub replicas: Vec<RaftSpdkReplicaStatusItem>,
}

#[derive(Debug, Deserialize)]
pub struct RaftSpdkStatusQuery {
    #[serde(default = "default_lag_threshold")]
    lag_threshold: u64,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct RaftRepairQueueItem {
    pub id: Uuid,
    pub backend_id: Uuid,
    pub group_id: Uuid,
    pub op_type: String,
    pub op_args: JsonValue,
    pub state: String,
    pub attempts: i32,
    pub last_error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RaftRepairQueueResponse {
    pub items: Vec<RaftRepairQueueItem>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RaftRepairReplicaResponse {
    pub operation: RaftRepairQueueItem,
}

#[derive(Debug, Clone, Serialize)]
pub struct RaftRepairProgress {
    pub node_id: u64,
    pub last_applied_index: u64,
    pub required_applied_index: u64,
    pub caught_up: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct RaftRepairStatusResponse {
    pub operation: Option<RaftRepairQueueItem>,
    pub progress: Option<RaftRepairProgress>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub progress_error: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AddRaftSpdkReplicaReq {
    pub node_id: u64,
    pub agent_base_url: String,
    pub spdk_backend_id: Uuid,
    #[serde(default)]
    pub desired_store_kind: Option<RaftBlockStoreKind>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AddRaftSpdkReplicaResponse {
    pub operation: RaftRepairQueueItem,
    pub locator: RaftSpdkLocator,
}

fn default_lag_threshold() -> u64 {
    1024
}

#[utoipa::path(
    get,
    path = "/v1/storage_backends",
    responses(
        (status = 200, body = StorageBackendListResponse),
    ),
    tag = "StorageBackends",
)]
pub async fn list(Extension(st): Extension<AppState>) -> impl IntoResponse {
    let repo = StorageBackendRepository::new(st.db.clone());
    match repo.list_active().await {
        Ok(rows) => {
            let mut items = Vec::with_capacity(rows.len());
            for r in rows {
                match row_to_wire(r) {
                    Ok(w) => items.push(w),
                    Err(s) => {
                        return (s, Json(serde_json::json!({"error": "row deserialization"})))
                            .into_response()
                    }
                }
            }
            (StatusCode::OK, Json(StorageBackendListResponse { items })).into_response()
        }
        Err(e) => {
            tracing::error!("storage_backends list failed: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "db"})),
            )
                .into_response()
        }
    }
}

#[utoipa::path(
    get,
    path = "/v1/storage_backends/{id}",
    params(("id" = Uuid, Path, description = "Storage backend ID")),
    responses(
        (status = 200, body = StorageBackend),
        (status = 404),
    ),
    tag = "StorageBackends",
)]
pub async fn get_one(
    Extension(st): Extension<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let repo = StorageBackendRepository::new(st.db.clone());
    match repo.get(id).await {
        Ok(Some(row)) => match row_to_wire(row) {
            Ok(w) => (StatusCode::OK, Json(w)).into_response(),
            Err(s) => {
                (s, Json(serde_json::json!({"error": "row deserialization"}))).into_response()
            }
        },
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "not found"})),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("storage_backends get failed: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "db"})),
            )
                .into_response()
        }
    }
}

#[utoipa::path(
    get,
    path = "/v1/storage_backends/{id}/repair_queue",
    params(("id" = Uuid, Path, description = "Storage backend ID")),
    responses((status = 200), (status = 400), (status = 404)),
    tag = "StorageBackends",
)]
pub async fn list_repair_queue(
    Extension(st): Extension<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    if let Err((status, error)) = get_raft_spdk_backend_row(&st, id).await {
        return (status, Json(serde_json::json!({ "error": error }))).into_response();
    }

    match sqlx::query_as::<_, RaftRepairQueueItem>(
        r#"
        SELECT id,
               backend_id,
               group_id,
               op_type,
               op_args,
               state,
               attempts,
               last_error,
               created_at,
               started_at,
               finished_at,
               updated_at
          FROM raft_repair_queue
         WHERE backend_id = $1
         ORDER BY created_at DESC, id DESC
         LIMIT 200
        "#,
    )
    .bind(id)
    .fetch_all(&st.db)
    .await
    {
        Ok(items) => (StatusCode::OK, Json(RaftRepairQueueResponse { items })).into_response(),
        Err(e) => {
            tracing::error!(backend_id = %id, error = ?e, "raft repair queue list failed");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "db"})),
            )
                .into_response()
        }
    }
}

#[utoipa::path(
    post,
    path = "/v1/storage_backends/{id}/groups/{group_id}/replicas",
    params(
        ("id" = Uuid, Path, description = "Storage backend ID"),
        ("group_id" = Uuid, Path, description = "Raft block group ID")
    ),
    responses((status = 200), (status = 400), (status = 404), (status = 409), (status = 502), (status = 504)),
    tag = "StorageBackends",
)]
pub async fn add_replica(
    Extension(st): Extension<AppState>,
    Path((id, group_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<AddRaftSpdkReplicaReq>,
) -> impl IntoResponse {
    if req.node_id == 0 || req.agent_base_url.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "node_id and agent_base_url are required" })),
        )
            .into_response();
    }
    if let Err((status, error)) = get_raft_spdk_backend_row(&st, id).await {
        return (status, Json(serde_json::json!({ "error": error }))).into_response();
    }
    let groups = match load_raft_spdk_groups(&st, id).await {
        Ok(groups) => groups,
        Err((status, error)) => {
            return (status, Json(serde_json::json!({ "error": error }))).into_response();
        }
    };
    let Some((volume_id, locator)) = groups
        .into_iter()
        .find(|(_, locator)| locator.group_id == group_id)
    else {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "group not found" })),
        )
            .into_response();
    };
    if locator
        .replicas
        .iter()
        .any(|replica| replica.node_id == req.node_id)
    {
        return (
            StatusCode::CONFLICT,
            Json(serde_json::json!({ "error": "replica node_id already exists" })),
        )
            .into_response();
    }

    let agent_base_url = normalize_raft_block_base_url(&req.agent_base_url);
    let spdk_lvol_locator = serde_json::json!({
        "spdk_backend_id": req.spdk_backend_id,
        "production_replica": true
    })
    .to_string();
    let new_replica = RaftSpdkReplicaLocator {
        node_id: req.node_id,
        agent_base_url,
        spdk_lvol_locator,
    };
    let mut expanded_replicas = locator.replicas.clone();
    expanded_replicas.push(new_replica.clone());
    expanded_replicas.sort_by_key(|replica| replica.node_id);
    let expanded_locator = match RaftSpdkLocator::new(
        locator.group_id,
        locator.size_bytes,
        locator.block_size,
        expanded_replicas,
        locator.leader_hint,
    ) {
        Ok(locator) => locator,
        Err(err) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": err.to_string() })),
            )
                .into_response();
        }
    };

    let mut operation =
        match create_repair_queue_row(&st, id, group_id, req.node_id, "add_replica").await {
            Ok(row) => row,
            Err(e) => {
                tracing::error!(
                    backend_id = %id,
                    group_id = %group_id,
                    node_id = req.node_id,
                    error = ?e,
                    "failed to create raft add-replica queue row"
                );
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({"error": "db"})),
                )
                    .into_response();
            }
        };

    let desired_store_kind = req
        .desired_store_kind
        .unwrap_or(RaftBlockStoreKind::SpdkLvol);
    if let Err(error) =
        create_replica_group(&new_replica, &expanded_locator, desired_store_kind).await
    {
        let _ = finish_repair_queue_row(&st, operation.id, "failed", Some(&error)).await;
        return (
            StatusCode::BAD_GATEWAY,
            Json(serde_json::json!({ "error": error, "operation_id": operation.id })),
        )
            .into_response();
    }
    if let Err(error) = start_replica_runtime(
        new_replica.agent_base_url.as_str(),
        expanded_locator.group_id,
        replica_peer_map(&expanded_locator),
    )
    .await
    {
        let _ = finish_repair_queue_row(&st, operation.id, "failed", Some(&error)).await;
        return (
            repair_start_error_status(&error),
            Json(serde_json::json!({ "error": error, "operation_id": operation.id })),
        )
            .into_response();
    }
    if let Err(error) = wait_for_replica_catchup(
        &expanded_locator,
        req.node_id,
        REPAIR_CATCHUP_TIMEOUT,
        REPAIR_CATCHUP_POLL_INTERVAL,
    )
    .await
    {
        let _ = finish_repair_queue_row(&st, operation.id, "failed", Some(&error)).await;
        return (
            StatusCode::GATEWAY_TIMEOUT,
            Json(serde_json::json!({ "error": error, "operation_id": operation.id })),
        )
            .into_response();
    }
    if let Err(error) = change_membership_on_leader(&expanded_locator).await {
        let _ = finish_repair_queue_row(&st, operation.id, "failed", Some(&error)).await;
        return (
            StatusCode::BAD_GATEWAY,
            Json(serde_json::json!({ "error": error, "operation_id": operation.id })),
        )
            .into_response();
    }
    if let Err(e) = persist_added_replica(&st, id, volume_id, &expanded_locator, &new_replica).await
    {
        let error = format!("persist added replica: {e}");
        let _ = finish_repair_queue_row(&st, operation.id, "failed", Some(&error)).await;
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": error, "operation_id": operation.id })),
        )
            .into_response();
    }

    match finish_repair_queue_row(&st, operation.id, "succeeded", None).await {
        Ok(row) => {
            operation = row;
            (
                StatusCode::OK,
                Json(AddRaftSpdkReplicaResponse {
                    operation,
                    locator: expanded_locator,
                }),
            )
                .into_response()
        }
        Err(e) => {
            tracing::error!(
                operation_id = %operation.id,
                error = ?e,
                "failed to mark raft add-replica operation succeeded"
            );
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "db"})),
            )
                .into_response()
        }
    }
}

#[utoipa::path(
    post,
    path = "/v1/storage_backends/{id}/groups/{group_id}/replicas/{node_id}/repair",
    params(
        ("id" = Uuid, Path, description = "Storage backend ID"),
        ("group_id" = Uuid, Path, description = "Raft block group ID"),
        ("node_id" = u64, Path, description = "Replica node ID")
    ),
    responses((status = 200), (status = 400), (status = 404), (status = 412), (status = 502), (status = 504)),
    tag = "StorageBackends",
)]
pub async fn repair_replica(
    Extension(st): Extension<AppState>,
    Path((id, group_id, node_id)): Path<(Uuid, Uuid, u64)>,
) -> impl IntoResponse {
    if let Err((status, error)) = get_raft_spdk_backend_row(&st, id).await {
        return (status, Json(serde_json::json!({ "error": error }))).into_response();
    }
    let groups = match load_raft_spdk_groups(&st, id).await {
        Ok(groups) => groups,
        Err((status, error)) => {
            return (status, Json(serde_json::json!({ "error": error }))).into_response();
        }
    };
    let Some((_, locator)) = groups
        .into_iter()
        .find(|(_, locator)| locator.group_id == group_id)
    else {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "group not found" })),
        )
            .into_response();
    };
    let Some(replica) = locator
        .replicas
        .iter()
        .find(|replica| replica.node_id == node_id)
    else {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "replica not found" })),
        )
            .into_response();
    };

    let mut operation =
        match create_repair_queue_row(&st, id, group_id, node_id, "repair_replica").await {
            Ok(row) => row,
            Err(e) => {
                tracing::error!(
                    backend_id = %id,
                    group_id = %group_id,
                    node_id,
                    error = ?e,
                    "failed to create raft repair queue row"
                );
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({"error": "db"})),
                )
                    .into_response();
            }
        };

    let peers = replica_peer_map(&locator);
    match start_replica_runtime(replica.agent_base_url.as_str(), group_id, peers).await {
        Ok(()) => match wait_for_replica_catchup(
            &locator,
            node_id,
            REPAIR_CATCHUP_TIMEOUT,
            REPAIR_CATCHUP_POLL_INTERVAL,
        )
        .await
        {
            Ok(()) => match finish_repair_queue_row(&st, operation.id, "succeeded", None).await {
                Ok(row) => {
                    operation = row;
                    (
                        StatusCode::OK,
                        Json(RaftRepairReplicaResponse { operation }),
                    )
                        .into_response()
                }
                Err(e) => {
                    tracing::error!(
                        operation_id = %operation.id,
                        error = ?e,
                        "failed to mark raft repair operation succeeded"
                    );
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(serde_json::json!({"error": "db"})),
                    )
                        .into_response()
                }
            },
            Err(error) => {
                let _ = finish_repair_queue_row(&st, operation.id, "failed", Some(&error)).await;
                (
                    StatusCode::GATEWAY_TIMEOUT,
                    Json(serde_json::json!({ "error": error, "operation_id": operation.id })),
                )
                    .into_response()
            }
        },
        Err(error) => {
            let _ = finish_repair_queue_row(&st, operation.id, "failed", Some(&error)).await;
            let status = repair_start_error_status(&error);
            (
                status,
                Json(serde_json::json!({ "error": error, "operation_id": operation.id })),
            )
                .into_response()
        }
    }
}

#[utoipa::path(
    get,
    path = "/v1/storage_backends/{id}/groups/{group_id}/replicas/{node_id}/repair_status",
    params(
        ("id" = Uuid, Path, description = "Storage backend ID"),
        ("group_id" = Uuid, Path, description = "Raft block group ID"),
        ("node_id" = u64, Path, description = "Replica node ID")
    ),
    responses((status = 200), (status = 400), (status = 404)),
    tag = "StorageBackends",
)]
pub async fn repair_status(
    Extension(st): Extension<AppState>,
    Path((id, group_id, node_id)): Path<(Uuid, Uuid, u64)>,
) -> impl IntoResponse {
    if let Err((status, error)) = get_raft_spdk_backend_row(&st, id).await {
        return (status, Json(serde_json::json!({ "error": error }))).into_response();
    }
    let groups = match load_raft_spdk_groups(&st, id).await {
        Ok(groups) => groups,
        Err((status, error)) => {
            return (status, Json(serde_json::json!({ "error": error }))).into_response();
        }
    };
    let Some((_, locator)) = groups
        .into_iter()
        .find(|(_, locator)| locator.group_id == group_id)
    else {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "group not found" })),
        )
            .into_response();
    };
    if !locator
        .replicas
        .iter()
        .any(|replica| replica.node_id == node_id)
    {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "replica not found" })),
        )
            .into_response();
    }

    let operation = match latest_repair_queue_row(&st, id, group_id, node_id).await {
        Ok(row) => row,
        Err(e) => {
            tracing::error!(
                backend_id = %id,
                group_id = %group_id,
                node_id,
                error = ?e,
                "failed to load latest raft repair queue row"
            );
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "db"})),
            )
                .into_response();
        }
    };
    let (progress, progress_error) = match replica_catchup_progress(&locator, node_id).await {
        Ok((last_applied_index, required_applied_index)) => (
            Some(RaftRepairProgress {
                node_id,
                last_applied_index,
                required_applied_index,
                caught_up: last_applied_index >= required_applied_index,
            }),
            None,
        ),
        Err(error) => (None, Some(error)),
    };

    (
        StatusCode::OK,
        Json(RaftRepairStatusResponse {
            operation,
            progress,
            progress_error,
        }),
    )
        .into_response()
}

fn replica_peer_map(locator: &RaftSpdkLocator) -> HashMap<u64, String> {
    locator
        .replicas
        .iter()
        .map(|replica| (replica.node_id, replica.agent_base_url.clone()))
        .collect()
}

fn normalize_raft_block_base_url(raw: &str) -> String {
    let trimmed = raw.trim_end_matches('/');
    if trimmed.ends_with("/v1/raft_block") {
        trimmed.to_string()
    } else {
        format!("{trimmed}/v1/raft_block")
    }
}

async fn create_replica_group(
    replica: &RaftSpdkReplicaLocator,
    locator: &RaftSpdkLocator,
    desired_store_kind: RaftBlockStoreKind,
) -> Result<(), String> {
    let url = format!("{}/create", replica.agent_base_url.trim_end_matches('/'));
    let response = reqwest::Client::new()
        .post(&url)
        .json(&serde_json::json!({
            "group_id": locator.group_id,
            "node_id": replica.node_id,
            "capacity_bytes": locator.size_bytes,
            "block_size": locator.block_size,
            "desired_store_kind": desired_store_kind,
        }))
        .send()
        .await
        .map_err(|e| format!("{url}: {e}"))?;
    if response.status().is_success() {
        return Ok(());
    }
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    Err(format!("{url}: {status}: {body}"))
}

async fn start_replica_runtime(
    agent_base_url: &str,
    group_id: Uuid,
    peers: HashMap<u64, String>,
) -> Result<(), String> {
    let url = format!("{}/runtime_start", agent_base_url.trim_end_matches('/'));
    let response = reqwest::Client::new()
        .post(&url)
        .json(&serde_json::json!({
            "group_id": group_id,
            "peers": peers,
        }))
        .send()
        .await
        .map_err(|e| format!("{url}: {e}"))?;
    if response.status().is_success() {
        return Ok(());
    }
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    Err(format!("{url}: {status}: {body}"))
}

async fn change_membership_on_leader(locator: &RaftSpdkLocator) -> Result<(), String> {
    let statuses = fetch_replica_statuses(locator).await;
    let observed_leader = aggregate_raft_spdk_status(locator, statuses, 0).observed_leader;
    let leader_id = observed_leader
        .or(locator.leader_hint)
        .ok_or_else(|| "cannot change membership: no observed leader".to_string())?;
    let leader = locator
        .replicas
        .iter()
        .find(|replica| replica.node_id == leader_id)
        .ok_or_else(|| format!("cannot change membership: leader {leader_id} not in locator"))?;
    let voters: Vec<u64> = locator
        .replicas
        .iter()
        .map(|replica| replica.node_id)
        .collect();
    let url = format!(
        "{}/{}/openraft/change_membership",
        leader.agent_base_url.trim_end_matches('/'),
        locator.group_id
    );
    let response = reqwest::Client::new()
        .post(&url)
        .json(&serde_json::json!({
            "voters": voters,
            "retain": false,
        }))
        .send()
        .await
        .map_err(|e| format!("{url}: {e}"))?;
    if response.status().is_success() {
        return Ok(());
    }
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    Err(format!("{url}: {status}: {body}"))
}

fn repair_start_error_status(error: &str) -> StatusCode {
    let normalized = error.to_ascii_lowercase();
    if normalized.contains("not started")
        || normalized.contains("not found")
        || normalized.contains("missing manifest")
    {
        StatusCode::PRECONDITION_FAILED
    } else {
        StatusCode::BAD_GATEWAY
    }
}

async fn wait_for_replica_catchup(
    locator: &RaftSpdkLocator,
    node_id: u64,
    timeout: Duration,
    poll_interval: Duration,
) -> Result<(), String> {
    let started = Instant::now();
    loop {
        match replica_catchup_progress(locator, node_id).await {
            Ok((target_applied, required_applied)) if target_applied >= required_applied => {
                return Ok(());
            }
            Ok((target_applied, required_applied)) if started.elapsed() >= timeout => {
                return Err(format!(
                    "timed out waiting for replica {node_id} to catch up: applied={target_applied}, required={required_applied}"
                ));
            }
            Ok(_) => {}
            Err(error) if started.elapsed() >= timeout => return Err(error),
            Err(_) => {}
        }
        sleep(poll_interval).await;
    }
}

async fn replica_catchup_progress(
    locator: &RaftSpdkLocator,
    node_id: u64,
) -> Result<(u64, u64), String> {
    let statuses = fetch_replica_statuses(locator).await;
    catchup_progress_from_statuses(node_id, statuses)
}

fn catchup_progress_from_statuses(
    node_id: u64,
    statuses: Vec<(u64, String, Result<RaftBlockReplicaStatus, String>)>,
) -> Result<(u64, u64), String> {
    let mut target_applied = None;
    let mut required_applied = 0_u64;
    let mut errors = Vec::new();

    for (status_node_id, _, result) in statuses {
        match result {
            Ok(status) => {
                let applied = status.last_applied_index.unwrap_or(0);
                if status_node_id == node_id {
                    target_applied = Some(applied);
                } else {
                    required_applied = required_applied.max(applied);
                }
            }
            Err(error) if status_node_id == node_id => errors.push(error),
            Err(_) => {}
        }
    }

    let Some(target_applied) = target_applied else {
        return Err(errors
            .pop()
            .unwrap_or_else(|| format!("replica {node_id} status unavailable")));
    };
    Ok((target_applied, required_applied))
}

async fn create_repair_queue_row(
    st: &AppState,
    backend_id: Uuid,
    group_id: Uuid,
    node_id: u64,
    op_type: &str,
) -> sqlx::Result<RaftRepairQueueItem> {
    sqlx::query_as::<_, RaftRepairQueueItem>(
        r#"
        INSERT INTO raft_repair_queue (
            backend_id,
            group_id,
            op_type,
            op_args,
            state,
            attempts,
            started_at
        )
        VALUES ($1, $2, $3, $4, 'in_progress', 1, now())
        RETURNING id,
                  backend_id,
                  group_id,
                  op_type,
                  op_args,
                  state,
                  attempts,
                  last_error,
                  created_at,
                  started_at,
                  finished_at,
                  updated_at
        "#,
    )
    .bind(backend_id)
    .bind(group_id)
    .bind(op_type)
    .bind(serde_json::json!({ "node_id": node_id }))
    .fetch_one(&st.db)
    .await
}

async fn finish_repair_queue_row(
    st: &AppState,
    operation_id: Uuid,
    state: &str,
    error: Option<&str>,
) -> sqlx::Result<RaftRepairQueueItem> {
    sqlx::query_as::<_, RaftRepairQueueItem>(
        r#"
        UPDATE raft_repair_queue
           SET state = $2,
               last_error = $3,
               finished_at = now(),
               updated_at = now()
         WHERE id = $1
        RETURNING id,
                  backend_id,
                  group_id,
                  op_type,
                  op_args,
                  state,
                  attempts,
                  last_error,
                  created_at,
                  started_at,
                  finished_at,
                  updated_at
        "#,
    )
    .bind(operation_id)
    .bind(state)
    .bind(error)
    .fetch_one(&st.db)
    .await
}

async fn latest_repair_queue_row(
    st: &AppState,
    backend_id: Uuid,
    group_id: Uuid,
    node_id: u64,
) -> sqlx::Result<Option<RaftRepairQueueItem>> {
    sqlx::query_as::<_, RaftRepairQueueItem>(
        r#"
        SELECT id,
               backend_id,
               group_id,
               op_type,
               op_args,
               state,
               attempts,
               last_error,
               created_at,
               started_at,
               finished_at,
               updated_at
          FROM raft_repair_queue
         WHERE backend_id = $1
           AND group_id = $2
           AND op_type = 'repair_replica'
           AND op_args->>'node_id' = $3
         ORDER BY created_at DESC, id DESC
         LIMIT 1
        "#,
    )
    .bind(backend_id)
    .bind(group_id)
    .bind(node_id.to_string())
    .fetch_optional(&st.db)
    .await
}

async fn persist_added_replica(
    st: &AppState,
    backend_id: Uuid,
    volume_id: Uuid,
    locator: &RaftSpdkLocator,
    replica: &RaftSpdkReplicaLocator,
) -> sqlx::Result<()> {
    let encoded = locator
        .to_locator_string()
        .map_err(|e| sqlx::Error::Protocol(e.to_string()))?;
    let mut tx = st.db.begin().await?;
    sqlx::query(
        r#"
        UPDATE volume
           SET path = $2
         WHERE id = $1
        "#,
    )
    .bind(volume_id)
    .bind(encoded)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO raft_spdk_replica (
            backend_id,
            group_id,
            node_id,
            agent_base_url,
            spdk_lvol_locator,
            role,
            removed_at
        )
        VALUES ($1, $2, $3, $4, $5, 'voter', NULL)
        ON CONFLICT (backend_id, group_id, node_id) DO UPDATE
          SET agent_base_url = EXCLUDED.agent_base_url,
              spdk_lvol_locator = EXCLUDED.spdk_lvol_locator,
              role = 'voter',
              removed_at = NULL,
              updated_at = now()
        "#,
    )
    .bind(backend_id)
    .bind(locator.group_id)
    .bind(replica.node_id as i64)
    .bind(&replica.agent_base_url)
    .bind(&replica.spdk_lvol_locator)
    .execute(&mut *tx)
    .await?;
    tx.commit().await
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct BackendVolumeRow {
    id: Uuid,
    path: String,
    size_bytes: i64,
}

async fn get_raft_spdk_backend_row(
    st: &AppState,
    id: Uuid,
) -> Result<StorageBackendRow, (StatusCode, String)> {
    let repo = StorageBackendRepository::new(st.db.clone());
    let row = repo
        .get(id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("db: {e}")))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "not found".to_string()))?;
    if row.kind != "raft_spdk" {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("backend {} is {}, not raft_spdk", row.id, row.kind),
        ));
    }
    Ok(row)
}

async fn load_raft_spdk_groups(
    st: &AppState,
    backend_id: Uuid,
) -> Result<Vec<(Uuid, RaftSpdkLocator)>, (StatusCode, String)> {
    let rows = sqlx::query_as::<_, BackendVolumeRow>(
        r#"SELECT id, path, size_bytes FROM volume WHERE backend_id = $1 ORDER BY created_at, id"#,
    )
    .bind(backend_id)
    .fetch_all(&st.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("db: {e}")))?;

    let mut groups = Vec::new();
    let mut seen = BTreeSet::new();
    for row in rows {
        let Ok(locator) = RaftSpdkLocator::from_locator_str(&row.path) else {
            tracing::warn!(
                volume_id = %row.id,
                backend_id = %backend_id,
                size_bytes = row.size_bytes,
                "skipping raft_spdk volume row with unparsable locator"
            );
            continue;
        };
        if seen.insert(locator.group_id) {
            groups.push((row.id, locator));
        }
    }
    Ok(groups)
}

#[utoipa::path(
    get,
    path = "/v1/storage_backends/{id}/groups",
    params(("id" = Uuid, Path, description = "Storage backend ID")),
    responses((status = 200), (status = 400), (status = 404)),
    tag = "StorageBackends",
)]
pub async fn list_groups(
    Extension(st): Extension<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    if let Err((status, error)) = get_raft_spdk_backend_row(&st, id).await {
        return (status, Json(serde_json::json!({ "error": error }))).into_response();
    }
    match load_raft_spdk_groups(&st, id).await {
        Ok(groups) => {
            let items = groups
                .into_iter()
                .map(|(volume_id, locator)| RaftSpdkGroupListItem {
                    group_id: locator.group_id,
                    volume_id,
                    size_bytes: locator.size_bytes,
                    block_size: locator.block_size,
                    replica_count: locator.replicas.len(),
                    leader_hint: locator.leader_hint,
                })
                .collect();
            (StatusCode::OK, Json(RaftSpdkGroupListResponse { items })).into_response()
        }
        Err((status, error)) => {
            (status, Json(serde_json::json!({ "error": error }))).into_response()
        }
    }
}

#[utoipa::path(
    get,
    path = "/v1/storage_backends/{id}/groups/{group_id}",
    params(
        ("id" = Uuid, Path, description = "Storage backend ID"),
        ("group_id" = Uuid, Path, description = "Raft block group ID")
    ),
    responses((status = 200), (status = 400), (status = 404)),
    tag = "StorageBackends",
)]
pub async fn get_group_status(
    Extension(st): Extension<AppState>,
    Path((id, group_id)): Path<(Uuid, Uuid)>,
    Query(query): Query<RaftSpdkStatusQuery>,
) -> impl IntoResponse {
    if let Err((status, error)) = get_raft_spdk_backend_row(&st, id).await {
        return (status, Json(serde_json::json!({ "error": error }))).into_response();
    }
    let groups = match load_raft_spdk_groups(&st, id).await {
        Ok(groups) => groups,
        Err((status, error)) => {
            return (status, Json(serde_json::json!({ "error": error }))).into_response();
        }
    };
    let Some((_, locator)) = groups
        .into_iter()
        .find(|(_, locator)| locator.group_id == group_id)
    else {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "group not found" })),
        )
            .into_response();
    };

    let statuses = fetch_replica_statuses(&locator).await;
    let response = aggregate_raft_spdk_status(&locator, statuses, query.lag_threshold);
    (StatusCode::OK, Json(response)).into_response()
}

async fn fetch_replica_statuses(
    locator: &RaftSpdkLocator,
) -> Vec<(u64, String, Result<RaftBlockReplicaStatus, String>)> {
    let http = reqwest::Client::new();
    let mut out = Vec::with_capacity(locator.replicas.len());
    for replica in &locator.replicas {
        let base = replica.agent_base_url.trim_end_matches('/');
        let url = format!("{base}/{}/status", locator.group_id);
        let result = match http.get(&url).send().await {
            Ok(resp) if resp.status().is_success() => resp
                .json::<RaftBlockReplicaStatus>()
                .await
                .map_err(|e| format!("decode {url}: {e}")),
            Ok(resp) => {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                Err(format!("{url}: {status}: {body}"))
            }
            Err(e) => Err(format!("{url}: {e}")),
        };
        out.push((replica.node_id, replica.agent_base_url.clone(), result));
    }
    out
}

fn aggregate_raft_spdk_status(
    locator: &RaftSpdkLocator,
    statuses: Vec<(u64, String, Result<RaftBlockReplicaStatus, String>)>,
    lag_threshold: u64,
) -> RaftSpdkGroupStatusResponse {
    let quorum = locator.replicas.len() / 2 + 1;
    let mut healthy = 0_usize;
    let mut leaders = BTreeSet::new();
    let mut leader_applied = 0_u64;
    let mut observed_leader = None;
    let mut leader_self_reported = false;
    let mut replicas = Vec::with_capacity(statuses.len());

    for (node_id, agent_base_url, result) in statuses {
        match result {
            Ok(status) => {
                if status.state == "started" {
                    healthy += 1;
                }
                if let Some(leader) = status.current_leader {
                    leaders.insert(leader);
                }
                if status.current_leader == status.node_id {
                    observed_leader = status.current_leader;
                    leader_self_reported = true;
                    leader_applied = status.last_applied_index.unwrap_or(0);
                }
                replicas.push(RaftSpdkReplicaStatusItem {
                    node_id,
                    agent_base_url,
                    healthy: status.state == "started",
                    status: Some(status),
                    error: None,
                });
            }
            Err(error) => replicas.push(RaftSpdkReplicaStatusItem {
                node_id,
                agent_base_url,
                healthy: false,
                status: None,
                error: Some(error),
            }),
        }
    }

    if observed_leader.is_none() && leaders.len() == 1 {
        observed_leader = leaders.iter().next().copied();
        leader_applied = replicas
            .iter()
            .filter_map(|replica| replica.status.as_ref()?.last_applied_index)
            .max()
            .unwrap_or(0);
    }

    let quorum_state = if healthy < quorum {
        RaftSpdkQuorumState::QuorumLost
    } else if leader_self_reported && observed_leader.is_some() && leaders.len() <= 1 {
        RaftSpdkQuorumState::LeaderSteady
    } else {
        RaftSpdkQuorumState::Electing
    };

    let lagging_followers = replicas
        .iter()
        .filter_map(|replica| {
            let status = replica.status.as_ref()?;
            if status.current_leader == Some(replica.node_id) {
                return None;
            }
            let applied = status.last_applied_index.unwrap_or(0);
            (leader_applied.saturating_sub(applied) > lag_threshold).then_some(replica.node_id)
        })
        .collect();

    RaftSpdkGroupStatusResponse {
        group_id: locator.group_id,
        size_bytes: locator.size_bytes,
        block_size: locator.block_size,
        leader_hint: locator.leader_hint,
        observed_leader,
        quorum_state,
        lagging_followers,
        replicas,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexus_storage::RaftSpdkReplicaLocator;

    fn locator() -> RaftSpdkLocator {
        RaftSpdkLocator::new(
            Uuid::parse_str("018f64ba-97aa-70d9-a7d2-6459256fd111").unwrap(),
            4096,
            512,
            vec![
                RaftSpdkReplicaLocator {
                    node_id: 1,
                    agent_base_url: "http://agent-1/v1/raft_block".into(),
                    spdk_lvol_locator: "{}".into(),
                },
                RaftSpdkReplicaLocator {
                    node_id: 2,
                    agent_base_url: "http://agent-2/v1/raft_block".into(),
                    spdk_lvol_locator: "{}".into(),
                },
                RaftSpdkReplicaLocator {
                    node_id: 3,
                    agent_base_url: "http://agent-3/v1/raft_block".into(),
                    spdk_lvol_locator: "{}".into(),
                },
            ],
            Some(1),
        )
        .unwrap()
    }

    fn status(node_id: u64, leader: Option<u64>, applied: u64) -> RaftBlockReplicaStatus {
        RaftBlockReplicaStatus {
            group_id: locator().group_id,
            state: "started".into(),
            data_path: "persistent_local_replica".into(),
            transport: "openraft_entry_local".into(),
            raft_state: Some(if leader == Some(node_id) {
                "Leader".into()
            } else {
                "Follower".into()
            }),
            current_term: Some(3),
            current_leader: leader,
            last_log_index: Some(applied),
            millis_since_quorum_ack: None,
            store_kind: RaftBlockStoreKind::SpdkLvol,
            store_path: Some(format!("/var/lib/spdk-stub/node-{node_id}.dev")),
            node_id: Some(node_id),
            capacity_bytes: Some(4096),
            block_size: Some(512),
            last_applied_index: Some(applied),
            compacted_through: Some(applied),
            retained_log_entries: 1,
        }
    }

    #[test]
    fn status_api_marks_steady_leader_and_lagging_follower() {
        let locator = locator();
        let response = aggregate_raft_spdk_status(
            &locator,
            vec![
                (
                    1,
                    "http://agent-1/v1/raft_block".into(),
                    Ok(status(1, Some(1), 2048)),
                ),
                (
                    2,
                    "http://agent-2/v1/raft_block".into(),
                    Ok(status(2, Some(1), 2047)),
                ),
                (
                    3,
                    "http://agent-3/v1/raft_block".into(),
                    Ok(status(3, Some(1), 1)),
                ),
            ],
            1024,
        );

        assert!(matches!(
            response.quorum_state,
            RaftSpdkQuorumState::LeaderSteady
        ));
        assert_eq!(response.observed_leader, Some(1));
        assert_eq!(response.lagging_followers, vec![3]);
    }

    #[test]
    fn status_api_marks_quorum_lost_when_majority_unreachable() {
        let locator = locator();
        let response = aggregate_raft_spdk_status(
            &locator,
            vec![
                (
                    1,
                    "http://agent-1/v1/raft_block".into(),
                    Ok(status(1, Some(1), 10)),
                ),
                (
                    2,
                    "http://agent-2/v1/raft_block".into(),
                    Err("offline".into()),
                ),
                (
                    3,
                    "http://agent-3/v1/raft_block".into(),
                    Err("offline".into()),
                ),
            ],
            1024,
        );

        assert!(matches!(
            response.quorum_state,
            RaftSpdkQuorumState::QuorumLost
        ));
    }

    #[test]
    fn status_api_marks_electing_when_leader_is_not_reachable() {
        let locator = locator();
        let response = aggregate_raft_spdk_status(
            &locator,
            vec![
                (
                    1,
                    "http://agent-1/v1/raft_block".into(),
                    Err("offline".into()),
                ),
                (
                    2,
                    "http://agent-2/v1/raft_block".into(),
                    Ok(status(2, Some(1), 10)),
                ),
                (
                    3,
                    "http://agent-3/v1/raft_block".into(),
                    Ok(status(3, Some(1), 10)),
                ),
            ],
            1024,
        );

        assert!(matches!(
            response.quorum_state,
            RaftSpdkQuorumState::Electing
        ));
        assert_eq!(response.observed_leader, Some(1));
    }

    #[test]
    fn repair_endpoint_builds_peer_map_from_locator() {
        let peers = replica_peer_map(&locator());

        assert_eq!(peers.len(), 3);
        assert_eq!(
            peers.get(&1).map(String::as_str),
            Some("http://agent-1/v1/raft_block")
        );
        assert_eq!(
            peers.get(&3).map(String::as_str),
            Some("http://agent-3/v1/raft_block")
        );
    }

    #[test]
    fn repair_progress_requires_target_to_reach_peer_high_watermark() {
        let progress = catchup_progress_from_statuses(
            3,
            vec![
                (
                    1,
                    "http://agent-1/v1/raft_block".into(),
                    Ok(status(1, Some(1), 20)),
                ),
                (
                    2,
                    "http://agent-2/v1/raft_block".into(),
                    Ok(status(2, Some(1), 18)),
                ),
                (
                    3,
                    "http://agent-3/v1/raft_block".into(),
                    Ok(status(3, Some(1), 17)),
                ),
            ],
        )
        .unwrap();

        assert_eq!(progress, (17, 20));
    }

    #[test]
    fn repair_progress_errors_when_target_status_is_missing() {
        let error = catchup_progress_from_statuses(
            3,
            vec![
                (
                    1,
                    "http://agent-1/v1/raft_block".into(),
                    Ok(status(1, Some(1), 20)),
                ),
                (
                    3,
                    "http://agent-3/v1/raft_block".into(),
                    Err("offline".into()),
                ),
            ],
        )
        .unwrap_err();

        assert_eq!(error, "offline");
    }

    #[test]
    fn repair_start_errors_classify_missing_manifest_as_precondition() {
        assert_eq!(
            repair_start_error_status("runtime_start: group abc not started"),
            StatusCode::PRECONDITION_FAILED
        );
        assert_eq!(
            repair_start_error_status("connection refused"),
            StatusCode::BAD_GATEWAY
        );
    }
}
