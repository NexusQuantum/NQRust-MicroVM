use crate::AppState;
use axum::{extract::Path, http::StatusCode, Extension, Json};
use nexus_types::{
    CreateSnapshotRequest, CreateSnapshotResponse, GetSnapshotResponse, InstantiateSnapshotReq,
    InstantiateSnapshotResp, ListSnapshotsResponse, OkResponse, Snapshot, SnapshotPathParams,
    VmPathParams,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use super::repo::{NewSnapshotRow, SnapshotRepository};

#[utoipa::path(
    post,
    path = "/v1/vms/{id}/snapshots",
    params(VmPathParams),
    request_body(
        content = CreateSnapshotRequest,
        content_type = "application/json",
        description = "Optional agent snapshot configuration"
    ),
    responses(
        (status = 200, description = "Snapshot created", body = CreateSnapshotResponse),
        (status = 404, description = "VM not found"),
        (status = 500, description = "Failed to record snapshot"),
        (status = 502, description = "Agent interaction failed"),
    ),
    tag = "Snapshots"
)]
pub async fn create(
    Extension(st): Extension<AppState>,
    Path(VmPathParams { id: vm_id }): Path<VmPathParams>,
    _body: Option<Json<CreateSnapshotRequest>>,
) -> Result<Json<CreateSnapshotResponse>, StatusCode> {
    let payload = _body.map(|Json(req)| req);
    let vm = crate::features::vms::repo::get(&st.db, vm_id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    let snapshot_id = Uuid::new_v4();
    let snapshot_name = payload
        .as_ref()
        .and_then(|p| p.name.clone())
        .unwrap_or_else(|| format!("snapshot-{snapshot_id}"));
    let client = reqwest::Client::new();
    let base = format!("{}/agent/v1/vms/{}", vm.host_addr, vm.id);
    let qs = format!("?sock={}", urlencoding::encode(&vm.api_sock));
    let vm_url = format!("{base}/proxy/vm{qs}");
    let snapshot_url = format!("{base}/proxy/snapshot/create{qs}");
    let prepare_url = format!("{base}/snapshots/prepare");

    let snapshot_type = payload
        .as_ref()
        .and_then(|p| p.snapshot_type.clone())
        .unwrap_or_else(|| "Full".to_string());
    let parent_id = payload.as_ref().and_then(|p| p.parent_id);
    let track_dirty_pages = payload
        .as_ref()
        .and_then(|p| p.track_dirty_pages)
        .unwrap_or(false);

    client
        .patch(&vm_url)
        .json(&json!({"state": "Paused"}))
        .send()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?
        .error_for_status()
        .map_err(|_| StatusCode::BAD_GATEWAY)?;

    if track_dirty_pages {
        // ensure Firecracker tracking enabled before diff snapshot
        let _ = client
            .patch(format!("{base}/proxy/machine-config{qs}"))
            .json(&json!({ "track_dirty_pages": true }))
            .send()
            .await
            .and_then(|resp| resp.error_for_status())
            .map_err(|err| {
                tracing::warn!(vm_id = %vm.id, error = %err, "failed to enable dirty page tracking");
                StatusCode::BAD_GATEWAY
            });
    }

    let prepare_req = AgentPrepareSnapshotRequest {
        snapshot_id,
        snapshot_type: Some(snapshot_type.clone()),
    };
    let prepare_resp: AgentPrepareSnapshotResponse = client
        .post(&prepare_url)
        .json(&prepare_req)
        .send()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?
        .error_for_status()
        .map_err(|_| StatusCode::BAD_GATEWAY)?
        .json()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;

    let create_payload = if snapshot_type == "Diff" {
        json!({
            "snapshot_type": "Diff",
            "snapshot_path": prepare_resp.snapshot_path,
        })
    } else {
        json!({
            "snapshot_type": "Full",
            "snapshot_path": prepare_resp.snapshot_path,
            "mem_file_path": prepare_resp.mem_path,
        })
    };

    let snapshot_result = client
        .put(&snapshot_url)
        .json(&create_payload)
        .send()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?
        .error_for_status();

    let resume_result = client
        .patch(&vm_url)
        .json(&json!({"state": "Resumed"}))
        .send()
        .await;

    if let Err(err) = resume_result.and_then(|resp| resp.error_for_status()) {
        tracing::warn!(vm_id = %vm.id, error = %err, "failed to resume vm after snapshot");
    }

    snapshot_result.map_err(|_| StatusCode::BAD_GATEWAY)?;

    let sizes_resp: AgentPrepareSnapshotResponse = client
        .post(&prepare_url)
        .json(&prepare_req)
        .send()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?
        .error_for_status()
        .map_err(|_| StatusCode::BAD_GATEWAY)?
        .json()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;

    let combined_size = sizes_resp
        .snapshot_size_bytes
        .unwrap_or(0)
        .saturating_add(sizes_resp.mem_size_bytes.unwrap_or(0));
    let total_size: i64 = combined_size.try_into().unwrap_or(i64::MAX);

    let repo: SnapshotRepository = st.snapshots.clone();
    let row = repo
        .insert(&NewSnapshotRow {
            id: snapshot_id,
            vm_id,
            snapshot_path: sizes_resp.snapshot_path,
            mem_path: if snapshot_type == "Diff" {
                String::new()
            } else {
                sizes_resp.mem_path.clone().unwrap_or_default()
            },
            size_bytes: total_size,
            state: "available".into(),
            snapshot_type,
            parent_id,
            track_dirty_pages,
            name: Some(snapshot_name.clone()),
        })
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(CreateSnapshotResponse {
        id: row.id,
        name: row.name.clone(),
    }))
}

#[utoipa::path(
    get,
    path = "/v1/vms/{id}/snapshots",
    params(VmPathParams),
    responses(
        (status = 200, description = "Snapshots listed", body = ListSnapshotsResponse),
        (status = 500, description = "Failed to list snapshots"),
    ),
    tag = "Snapshots"
)]
pub async fn list_for_vm(
    Extension(st): Extension<AppState>,
    Path(VmPathParams { id: vm_id }): Path<VmPathParams>,
) -> Result<Json<ListSnapshotsResponse>, StatusCode> {
    let repo = st.snapshots.clone();
    let items = repo
        .list_for_vm(vm_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .into_iter()
        .map(Snapshot::from)
        .collect();
    Ok(Json(ListSnapshotsResponse { items }))
}

#[utoipa::path(
    get,
    path = "/v1/snapshots/{id}",
    params(SnapshotPathParams),
    responses(
        (status = 200, description = "Snapshot fetched", body = GetSnapshotResponse),
        (status = 404, description = "Snapshot not found"),
    ),
    tag = "Snapshots"
)]
pub async fn get(
    Extension(st): Extension<AppState>,
    Path(SnapshotPathParams { id }): Path<SnapshotPathParams>,
) -> Result<Json<GetSnapshotResponse>, StatusCode> {
    let repo = st.snapshots.clone();
    let item = repo
        .get(id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?
        .into();
    Ok(Json(GetSnapshotResponse { item }))
}

#[utoipa::path(
    delete,
    path = "/v1/snapshots/{id}",
    params(SnapshotPathParams),
    responses(
        (status = 200, description = "Snapshot deleted", body = OkResponse),
        (status = 404, description = "Snapshot not found"),
        (status = 500, description = "Failed to delete snapshot"),
    ),
    tag = "Snapshots"
)]
pub async fn delete(
    Extension(st): Extension<AppState>,
    Path(SnapshotPathParams { id }): Path<SnapshotPathParams>,
) -> Result<Json<OkResponse>, StatusCode> {
    let repo = st.snapshots.clone();
    repo.delete(id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(OkResponse::default()))
}

#[utoipa::path(
    post,
    path = "/v1/snapshots/{id}/instantiate",
    params(SnapshotPathParams),
    request_body(
        content = InstantiateSnapshotReq,
        content_type = "application/json",
        description = "Optional overrides when instantiating a snapshot"
    ),
    responses(
        (status = 200, description = "Snapshot instantiated", body = InstantiateSnapshotResp),
        (status = 404, description = "Snapshot not found"),
        (status = 502, description = "Failed to instantiate snapshot"),
    ),
    tag = "Snapshots"
)]
pub async fn instantiate(
    Extension(st): Extension<AppState>,
    Path(SnapshotPathParams { id }): Path<SnapshotPathParams>,
    body: Option<Json<InstantiateSnapshotReq>>,
) -> Result<Json<InstantiateSnapshotResp>, StatusCode> {
    let payload = body.map(|Json(req)| req).unwrap_or_default();
    let repo = st.snapshots.clone();
    let snapshot = repo.get(id).await.map_err(|_| StatusCode::NOT_FOUND)?;
    let source_vm = crate::features::vms::repo::get(&st.db, snapshot.vm_id)
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;

    let vm_id = Uuid::new_v4();
    let name = payload.name.unwrap_or_else(|| {
        snapshot
            .name
            .clone()
            .unwrap_or_else(|| format!("snapshot-{}", snapshot.id))
    });

    crate::features::vms::service::create_from_snapshot(
        &st,
        vm_id,
        name.clone(),
        None,
        snapshot.clone(),
        Some(source_vm),
    )
    .await
    .map_err(|err| {
        tracing::error!(snapshot_id = %id, error = ?err, "failed to instantiate snapshot");
        StatusCode::BAD_GATEWAY
    })?;

    Ok(Json(InstantiateSnapshotResp { id: vm_id, name }))
}

#[derive(Serialize)]
struct AgentPrepareSnapshotRequest {
    snapshot_id: Uuid,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    snapshot_type: Option<String>,
}

#[derive(Deserialize)]
struct AgentPrepareSnapshotResponse {
    snapshot_path: String,
    #[serde(default)]
    mem_path: Option<String>,
    #[serde(default)]
    snapshot_size_bytes: Option<u64>,
    #[serde(default)]
    mem_size_bytes: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[allow(dead_code)]
    diff_dir: Option<String>,
}

impl From<super::repo::SnapshotRow> for Snapshot {
    fn from(row: super::repo::SnapshotRow) -> Self {
        Snapshot {
            id: row.id,
            vm_id: row.vm_id,
            snapshot_path: row.snapshot_path,
            mem_path: row.mem_path,
            size_bytes: row.size_bytes,
            state: row.state,
            name: row.name.clone(),
            created_at: row.created_at,
            updated_at: row.updated_at,
            snapshot_type: Some(row.snapshot_type.clone()),
            parent_id: row.parent_id,
            track_dirty_pages: row.track_dirty_pages,
        }
    }
}
