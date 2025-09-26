use crate::AppState;
use axum::{extract::Path, http::StatusCode, Extension, Json};
use nexus_types::{
    CreateSnapshotRequest, CreateSnapshotResponse, GetSnapshotResponse, InstantiateSnapshotReq,
    InstantiateSnapshotResp, ListSnapshotsResponse, Snapshot,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use super::repo::{NewSnapshotRow, SnapshotRepository};

pub async fn create(
    Extension(st): Extension<AppState>,
    Path(vm_id): Path<Uuid>,
    _body: Option<Json<CreateSnapshotRequest>>,
) -> Result<Json<CreateSnapshotResponse>, StatusCode> {
    let vm = crate::features::vms::repo::get(&st.db, vm_id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    let snapshot_id = Uuid::new_v4();
    let client = reqwest::Client::new();
    let base = format!("{}/agent/v1/vms/{}", vm.host_addr, vm.id);
    let qs = format!("?sock={}", urlencoding::encode(&vm.api_sock));
    let actions_url = format!("{base}/proxy/actions{qs}");
    let snapshot_url = format!("{base}/proxy/snapshot/create{qs}");
    let prepare_url = format!("{base}/snapshots/prepare");

    client
        .put(&actions_url)
        .json(&json!({"action_type": "InstancePause"}))
        .send()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?
        .error_for_status()
        .map_err(|_| StatusCode::BAD_GATEWAY)?;

    let prepare_req = AgentPrepareSnapshotRequest { snapshot_id };
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

    let snapshot_result = client
        .put(&snapshot_url)
        .json(&json!({
            "snapshot_type": "Full",
            "snapshot_path": prepare_resp.snapshot_path,
            "mem_file_path": prepare_resp.mem_path,
            "version": 1
        }))
        .send()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?
        .error_for_status();

    let resume_result = client
        .put(&actions_url)
        .json(&json!({"action_type": "InstanceResume"}))
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
    let total_size: i64 = match combined_size.try_into() {
        Ok(value) => value,
        Err(_) => i64::MAX,
    };

    let repo: SnapshotRepository = st.snapshots.clone();
    let row = repo
        .insert(&NewSnapshotRow {
            id: snapshot_id,
            vm_id,
            snapshot_path: sizes_resp.snapshot_path,
            mem_path: sizes_resp.mem_path,
            size_bytes: total_size,
            state: "available".into(),
        })
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(CreateSnapshotResponse { id: row.id }))
}

pub async fn list_for_vm(
    Extension(st): Extension<AppState>,
    Path(vm_id): Path<Uuid>,
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

pub async fn get(
    Extension(st): Extension<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<GetSnapshotResponse>, StatusCode> {
    let repo = st.snapshots.clone();
    let item = repo
        .get(id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?
        .into();
    Ok(Json(GetSnapshotResponse { item }))
}

pub async fn instantiate(
    Extension(st): Extension<AppState>,
    Path(id): Path<Uuid>,
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
        let suffix = vm_id.to_string();
        let suffix = &suffix[..8];
        format!("{}-clone-{suffix}", source_vm.name)
    });

    crate::features::vms::service::create_from_snapshot(
        &st,
        vm_id,
        name.clone(),
        None,
        snapshot,
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
}

#[derive(Deserialize)]
struct AgentPrepareSnapshotResponse {
    snapshot_path: String,
    mem_path: String,
    #[serde(default)]
    snapshot_size_bytes: Option<u64>,
    #[serde(default)]
    mem_size_bytes: Option<u64>,
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
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}
