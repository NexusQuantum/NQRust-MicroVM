use crate::AppState;
use axum::{extract::Path, http::StatusCode, Extension, Json};
use nexus_types::{
    CreateSnapshotRequest, CreateSnapshotResponse, GetSnapshotResponse, InstantiateSnapshotReq,
    InstantiateSnapshotResp, ListSnapshotsResponse, OkResponse, Snapshot, SnapshotPathParams,
    VmPathParams,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use super::repo::{NewSnapshotRow, SnapshotRepository};

/// Build the on-disk path where QEMU snapshot state is written. Lives under
/// the VM's storage dir so cleanup follows the same path FC snapshots use.
fn qemu_snapshot_state_path(vm_id: Uuid, snapshot_id: Uuid) -> std::path::PathBuf {
    std::path::PathBuf::from(format!(
        "/srv/fc/vms/{vm_id}/snapshots/{snapshot_id}/state.qmp"
    ))
}

/// Create a QEMU snapshot via the pluggable VMM route. Mirrors the FC
/// flow's audit log and DB shape so the snapshots index UI works uniformly.
async fn create_qemu_snapshot(
    st: &AppState,
    vm: &crate::features::vms::repo::VmRow,
    payload: Option<CreateSnapshotRequest>,
) -> Result<Json<CreateSnapshotResponse>, StatusCode> {
    let snapshot_id = Uuid::new_v4();
    let snapshot_name = resolve_snapshot_name(
        payload.as_ref().and_then(|p| p.name.as_deref()),
        snapshot_id,
    );

    // Ensure the snapshot dir exists. Manager runs co-located with the
    // agent in dev; in prod the agent will create the dir before writing.
    let state_path = qemu_snapshot_state_path(vm.id, snapshot_id);
    if let Some(parent) = state_path.parent() {
        let _ = tokio::fs::create_dir_all(parent).await;
    }

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/agent/v1/vmm/{}/snapshot", vm.host_addr, vm.id))
        .json(&json!({
            "vmm_kind": "qemu",
            "state_path": state_path,
            "kind": "full",
        }))
        .send()
        .await
        .map_err(|err| {
            tracing::error!(vm_id=%vm.id, error=?err, "agent qemu snapshot request failed");
            StatusCode::BAD_GATEWAY
        })?;
    let status = resp.status();
    if !status.is_success() {
        let text = resp.text().await.unwrap_or_default();
        tracing::error!(vm_id=%vm.id, status=%status, body=%text, "agent rejected qemu snapshot");
        return Err(StatusCode::BAD_GATEWAY);
    }
    let meta: nexus_vmm::SnapshotMeta = resp.json().await.map_err(|err| {
        tracing::error!(vm_id=%vm.id, error=?err, "decode agent snapshot meta");
        StatusCode::BAD_GATEWAY
    })?;

    let new_row = NewSnapshotRow {
        id: snapshot_id,
        vm_id: vm.id,
        snapshot_path: state_path.display().to_string(),
        mem_path: String::new(),
        size_bytes: meta.state_size_bytes as i64,
        state: "ready".to_string(),
        snapshot_type: "Full".to_string(),
        parent_id: None,
        track_dirty_pages: false,
        name: Some(snapshot_name.clone()),
    };
    let row = st.snapshots.insert(&new_row).await.map_err(|err| {
        tracing::error!(vm_id=%vm.id, error=?err, "insert qemu snapshot row");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Update snapshot.vmm_kind so the gate in instantiate can verify it.
    let _ = sqlx::query(r#"UPDATE snapshot SET vmm_kind = 'qemu' WHERE id = $1"#)
        .bind(snapshot_id)
        .execute(&st.db)
        .await;

    Ok(Json(CreateSnapshotResponse {
        id: row.id,
        name: Some(snapshot_name),
    }))
}

/// Look up the source VM's vmm_kind. Default to 'firecracker' on any error
/// so legacy code paths (pre-0.5.0 VMs) keep working.
async fn vm_vmm_kind(db: &sqlx::PgPool, vm_id: Uuid) -> String {
    sqlx::query_scalar::<_, String>(r#"SELECT vmm_kind FROM vm WHERE id = $1"#)
        .bind(vm_id)
        .fetch_one(db)
        .await
        .unwrap_or_else(|_| "firecracker".to_string())
}

/// Look up a snapshot row's vmm_kind. Default to 'firecracker' for legacy
/// rows that pre-date the 0040 migration's backfill.
async fn snapshot_vmm_kind(db: &sqlx::PgPool, snapshot_id: Uuid) -> String {
    sqlx::query_scalar::<_, String>(r#"SELECT vmm_kind FROM snapshot WHERE id = $1"#)
        .bind(snapshot_id)
        .fetch_one(db)
        .await
        .unwrap_or_else(|_| "firecracker".to_string())
}

/// Group of derived agent URLs used during snapshot creation.
///
/// Pure-logic helper extracted from `create` so the URL construction can be
/// covered by unit tests without spinning up a VM or agent.
#[derive(Debug, Clone, PartialEq, Eq)]
struct AgentSnapshotUrls {
    vm_url: String,
    snapshot_url: String,
    prepare_url: String,
    machine_config_url: String,
}

/// Build the URL set used to drive an agent through the snapshot pipeline.
///
/// `api_sock` is URL-encoded so callers can pass arbitrary on-disk socket
/// paths.
fn build_agent_snapshot_urls(host_addr: &str, vm_id: Uuid, api_sock: &str) -> AgentSnapshotUrls {
    let base = format!("{host_addr}/agent/v1/vms/{vm_id}");
    let qs = format!("?sock={}", urlencoding::encode(api_sock));
    AgentSnapshotUrls {
        vm_url: format!("{base}/proxy/vm{qs}"),
        snapshot_url: format!("{base}/proxy/snapshot/create{qs}"),
        prepare_url: format!("{base}/snapshots/prepare"),
        machine_config_url: format!("{base}/proxy/machine-config{qs}"),
    }
}

/// Resolve the human-readable snapshot name, defaulting to a deterministic
/// `snapshot-{uuid}` string only when the caller did not supply one.
///
/// Matches the original inline behavior: `Some("")` is taken as-is.
fn resolve_snapshot_name(override_name: Option<&str>, snapshot_id: Uuid) -> String {
    override_name
        .map(str::to_string)
        .unwrap_or_else(|| format!("snapshot-{snapshot_id}"))
}

/// Resolve the snapshot type, defaulting to `Full` only when no value was
/// provided. Mirrors the original `unwrap_or_else` behavior, so an explicit
/// empty-string value flows through unchanged.
fn resolve_snapshot_type(override_type: Option<&str>) -> String {
    override_type
        .map(str::to_string)
        .unwrap_or_else(|| "Full".to_string())
}

/// Build the JSON payload sent to the agent's snapshot/create endpoint.
///
/// Diff snapshots intentionally omit `mem_file_path` (Firecracker writes only
/// the diff bitmap and a sidecar). Full snapshots always include the field —
/// when the agent did not provide a path we forward JSON `null`, matching the
/// original behavior of the inline `json!` macro.
fn build_create_snapshot_payload(
    snapshot_type: &str,
    snapshot_path: &str,
    mem_path: Option<&str>,
) -> Value {
    if snapshot_type == "Diff" {
        json!({
            "snapshot_type": "Diff",
            "snapshot_path": snapshot_path,
        })
    } else {
        json!({
            "snapshot_type": "Full",
            "snapshot_path": snapshot_path,
            "mem_file_path": mem_path,
        })
    }
}

/// Compute the combined on-disk size of a snapshot (snapshot + memory image),
/// clamped into the `i64` column we persist.
fn combined_snapshot_size_i64(
    snapshot_size_bytes: Option<u64>,
    mem_size_bytes: Option<u64>,
) -> i64 {
    let combined = snapshot_size_bytes
        .unwrap_or(0)
        .saturating_add(mem_size_bytes.unwrap_or(0));
    combined.try_into().unwrap_or(i64::MAX)
}

/// Resolve the `mem_path` value persisted on the snapshot row. Diff snapshots
/// do not own a memory image, so we record an empty string.
fn resolve_storage_mem_path(snapshot_type: &str, mem_path: Option<&str>) -> String {
    if snapshot_type == "Diff" {
        String::new()
    } else {
        mem_path.unwrap_or("").to_string()
    }
}

/// Resolve the VM name used when instantiating a VM from a snapshot.
///
/// Mirrors the original inline behavior: any `Some` override is taken
/// verbatim (even an empty string), otherwise the snapshot's own name is
/// used, and finally we fall back to the deterministic `snapshot-{uuid}`.
fn resolve_instantiate_name(
    override_name: Option<String>,
    snapshot_name: Option<&str>,
    snapshot_id: Uuid,
) -> String {
    override_name.unwrap_or_else(|| {
        snapshot_name
            .map(str::to_string)
            .unwrap_or_else(|| format!("snapshot-{snapshot_id}"))
    })
}

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

    // Snapshots are per-backend. QEMU goes through the pluggable VMM route
    // which speaks QMP; Firecracker keeps using the legacy REST path below.
    let vmm_kind = vm_vmm_kind(&st.db, vm.id).await;
    if vmm_kind == "qemu" {
        return create_qemu_snapshot(&st, &vm, payload).await;
    }

    let snapshot_id = Uuid::new_v4();
    let snapshot_name = resolve_snapshot_name(
        payload.as_ref().and_then(|p| p.name.as_deref()),
        snapshot_id,
    );
    let client = reqwest::Client::new();
    let urls = build_agent_snapshot_urls(&vm.host_addr, vm.id, &vm.api_sock);

    let snapshot_type =
        resolve_snapshot_type(payload.as_ref().and_then(|p| p.snapshot_type.as_deref()));
    let parent_id = payload.as_ref().and_then(|p| p.parent_id);
    let track_dirty_pages = payload
        .as_ref()
        .and_then(|p| p.track_dirty_pages)
        .unwrap_or(false);

    client
        .patch(&urls.vm_url)
        .json(&json!({"state": "Paused"}))
        .send()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?
        .error_for_status()
        .map_err(|_| StatusCode::BAD_GATEWAY)?;

    if track_dirty_pages {
        // ensure Firecracker tracking enabled before diff snapshot
        let _ = client
            .patch(&urls.machine_config_url)
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
        .post(&urls.prepare_url)
        .json(&prepare_req)
        .send()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?
        .error_for_status()
        .map_err(|_| StatusCode::BAD_GATEWAY)?
        .json()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;

    let create_payload = build_create_snapshot_payload(
        &snapshot_type,
        &prepare_resp.snapshot_path,
        prepare_resp.mem_path.as_deref(),
    );

    let snapshot_result = client
        .put(&urls.snapshot_url)
        .json(&create_payload)
        .send()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?
        .error_for_status();

    let resume_result = client
        .patch(&urls.vm_url)
        .json(&json!({"state": "Resumed"}))
        .send()
        .await;

    if let Err(err) = resume_result.and_then(|resp| resp.error_for_status()) {
        tracing::warn!(vm_id = %vm.id, error = %err, "failed to resume vm after snapshot");
    }

    snapshot_result.map_err(|_| StatusCode::BAD_GATEWAY)?;

    let sizes_resp: AgentPrepareSnapshotResponse = client
        .post(&urls.prepare_url)
        .json(&prepare_req)
        .send()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?
        .error_for_status()
        .map_err(|_| StatusCode::BAD_GATEWAY)?
        .json()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;

    let total_size =
        combined_snapshot_size_i64(sizes_resp.snapshot_size_bytes, sizes_resp.mem_size_bytes);

    let repo: SnapshotRepository = st.snapshots.clone();
    let row = repo
        .insert(&NewSnapshotRow {
            id: snapshot_id,
            vm_id,
            snapshot_path: sizes_resp.snapshot_path,
            mem_path: resolve_storage_mem_path(&snapshot_type, sizes_resp.mem_path.as_deref()),
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

    let src_kind = vm_vmm_kind(&st.db, snapshot.vm_id).await;
    let snap_kind = snapshot_vmm_kind(&st.db, snapshot.id).await;

    // QEMU snapshot restore: clone the disk captured at snapshot time into a new
    // VM and cold-boot it from that disk state (a consistent revert-to-snapshot;
    // the migrate-to-file RAM state is not replayed). Reuses create_and_start_qemu.
    if snap_kind == "qemu" && src_kind == "qemu" {
        let new_id = Uuid::new_v4();
        let name = resolve_instantiate_name(payload.name, snapshot.name.as_deref(), snapshot.id);

        // The agent captured the disk next to the snapshot's state file.
        let snap_disk = qemu_snapshot_state_path(snapshot.vm_id, snapshot.id)
            .parent()
            .map(|p| p.join("disk.qcow2"))
            .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
        if tokio::fs::metadata(&snap_disk).await.is_err() {
            tracing::error!(snapshot_id=%snapshot.id, path=%snap_disk.display(),
                "qemu snapshot has no captured disk (taken before the disk-capture fix?)");
            return Err(StatusCode::BAD_REQUEST);
        }
        // Copy it into the new VM's storage as its root disk (the master stays
        // intact so the snapshot can be instantiated again).
        let dst_dir = std::path::PathBuf::from(
            std::env::var("MANAGER_STORAGE_ROOT").unwrap_or_else(|_| "/srv/fc/vms".into()),
        )
        .join(new_id.to_string())
        .join("storage");
        if let Err(e) = tokio::fs::create_dir_all(&dst_dir).await {
            tracing::error!(error=?e, "create storage dir for snapshot restore");
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
        let new_disk = dst_dir.join("disk.qcow2");
        if let Err(e) = tokio::fs::copy(&snap_disk, &new_disk).await {
            tracing::error!(error=?e, "copy snapshot disk for restore");
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }

        // Source VM's persisted boot mode (OVMF/UEFI etc.).
        let boot_mode_json: Option<serde_json::Value> =
            sqlx::query_scalar(r#"SELECT boot_mode FROM vm WHERE id = $1"#)
                .bind(snapshot.vm_id)
                .fetch_optional(&st.db)
                .await
                .ok()
                .flatten()
                .flatten();
        let boot_mode: Option<nexus_vmm::BootMode> =
            boot_mode_json.and_then(|j| serde_json::from_value(j).ok());

        let req = nexus_types::CreateVmReq {
            name: name.clone(),
            vcpu: source_vm.vcpu.max(1) as u8,
            mem_mib: source_vm.mem_mib.max(1) as u32,
            vmm_kind: Some(nexus_vmm::VmmKind::Qemu),
            boot_mode,
            guest_os: source_vm
                .guest_os
                .as_deref()
                .and_then(nexus_vmm::GuestOs::parse),
            rootfs_path: Some(new_disk.to_string_lossy().into_owned()),
            enable_vnc: source_vm.console_kind.as_deref() == Some("vnc"),
            ..Default::default()
        };

        crate::features::vms::qemu_service::create_and_start_qemu(
            &st,
            new_id,
            req,
            None,
            None,
            "snapshot-restore",
        )
        .await
        .map_err(|err| {
            tracing::error!(snapshot_id=%id, error=?err, "failed to instantiate qemu snapshot");
            StatusCode::BAD_GATEWAY
        })?;

        return Ok(Json(InstantiateSnapshotResp { id: new_id, name }));
    }

    // Cross-backend restore is not supported: the FC instantiate path can't
    // materialise a QEMU VM and vice versa. Only FC→FC reaches here now.
    if snap_kind != "firecracker" || src_kind != "firecracker" {
        tracing::warn!(
            snapshot_id=%snapshot.id,
            snapshot_vmm_kind=%snap_kind,
            source_vmm_kind=%src_kind,
            "snapshot instantiate (this route) is FC-only; QEMU instantiate lands in a follow-up",
        );
        return Err(StatusCode::BAD_REQUEST);
    }

    let vm_id = Uuid::new_v4();
    let name = resolve_instantiate_name(payload.name, snapshot.name.as_deref(), snapshot.id);

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

#[cfg(test)]
mod tests {
    use super::super::repo::SnapshotRow;
    use super::*;

    fn fixed_uuid() -> Uuid {
        // Deterministic so tests can assert exact strings.
        Uuid::parse_str("11111111-2222-3333-4444-555555555555").unwrap()
    }

    #[test]
    fn build_agent_snapshot_urls_encodes_socket_and_assembles_paths() {
        let vm_id = fixed_uuid();
        let urls =
            build_agent_snapshot_urls("http://10.0.0.5:9090", vm_id, "/srv/fc/vms/x/sock/fc.sock");

        let expected_base = format!("http://10.0.0.5:9090/agent/v1/vms/{vm_id}");
        let expected_qs = "?sock=%2Fsrv%2Ffc%2Fvms%2Fx%2Fsock%2Ffc.sock";

        assert_eq!(
            urls.vm_url,
            format!("{expected_base}/proxy/vm{expected_qs}")
        );
        assert_eq!(
            urls.snapshot_url,
            format!("{expected_base}/proxy/snapshot/create{expected_qs}")
        );
        assert_eq!(
            urls.prepare_url,
            format!("{expected_base}/snapshots/prepare")
        );
        assert_eq!(
            urls.machine_config_url,
            format!("{expected_base}/proxy/machine-config{expected_qs}")
        );
    }

    #[test]
    fn build_agent_snapshot_urls_handles_socket_with_special_chars() {
        // Spaces and ampersands in the socket path must be percent-encoded so
        // the agent does not see them as additional query parameters.
        let urls = build_agent_snapshot_urls("http://h", Uuid::nil(), "/tmp/with space&amp.sock");

        assert!(
            urls.vm_url
                .contains("sock=%2Ftmp%2Fwith%20space%26amp.sock"),
            "encoded sock missing in {}",
            urls.vm_url
        );
        assert!(
            !urls.vm_url.contains(" "),
            "raw space leaked into URL: {}",
            urls.vm_url
        );
    }

    #[test]
    fn resolve_snapshot_name_uses_override_when_present() {
        assert_eq!(
            resolve_snapshot_name(Some("nightly-backup"), fixed_uuid()),
            "nightly-backup"
        );
        // Mirrors original behavior: an explicit empty string is honored,
        // not replaced with the auto-generated default.
        assert_eq!(resolve_snapshot_name(Some(""), fixed_uuid()), "");
    }

    #[test]
    fn resolve_snapshot_name_falls_back_to_uuid_when_absent() {
        let id = fixed_uuid();
        assert_eq!(resolve_snapshot_name(None, id), format!("snapshot-{id}"));
    }

    #[test]
    fn resolve_snapshot_type_defaults_to_full() {
        assert_eq!(resolve_snapshot_type(None), "Full");
        assert_eq!(resolve_snapshot_type(Some("Diff")), "Diff");
        assert_eq!(resolve_snapshot_type(Some("Full")), "Full");
    }

    #[test]
    fn build_create_snapshot_payload_full_includes_mem_path() {
        let payload = build_create_snapshot_payload(
            "Full",
            "/var/lib/fc/snap.bin",
            Some("/var/lib/fc/mem.bin"),
        );
        assert_eq!(payload["snapshot_type"], "Full");
        assert_eq!(payload["snapshot_path"], "/var/lib/fc/snap.bin");
        assert_eq!(payload["mem_file_path"], "/var/lib/fc/mem.bin");
    }

    #[test]
    fn build_create_snapshot_payload_full_with_missing_mem_emits_null() {
        // When the agent did not surface a mem path, the original code
        // serialized `Option::None` as JSON `null`. Lock that in so the
        // upcoming refactor cannot silently drop the field.
        let payload = build_create_snapshot_payload("Full", "/snap.bin", None);
        assert!(payload["mem_file_path"].is_null(), "{payload}");
    }

    #[test]
    fn build_create_snapshot_payload_diff_omits_mem_field() {
        let payload =
            build_create_snapshot_payload("Diff", "/snap/diff.bin", Some("/should/be/ignored"));
        assert_eq!(payload["snapshot_type"], "Diff");
        assert_eq!(payload["snapshot_path"], "/snap/diff.bin");
        assert!(
            payload.get("mem_file_path").is_none(),
            "diff payload must not include mem_file_path: {payload}"
        );
    }

    #[test]
    fn combined_snapshot_size_i64_sums_and_handles_missing() {
        assert_eq!(combined_snapshot_size_i64(Some(100), Some(250)), 350);
        assert_eq!(combined_snapshot_size_i64(None, None), 0);
        assert_eq!(combined_snapshot_size_i64(Some(42), None), 42);
        assert_eq!(combined_snapshot_size_i64(None, Some(7)), 7);
    }

    #[test]
    fn combined_snapshot_size_i64_clamps_to_i64_max() {
        // Snapshot + mem larger than i64::MAX must not panic — the value is
        // saturated so the i64 column we persist stays valid.
        let huge = u64::MAX;
        assert_eq!(combined_snapshot_size_i64(Some(huge), Some(huge)), i64::MAX);
        assert_eq!(combined_snapshot_size_i64(Some(huge), None), i64::MAX);
    }

    #[test]
    fn resolve_storage_mem_path_zeroes_mem_for_diff() {
        assert_eq!(
            resolve_storage_mem_path("Diff", Some("/should/be/dropped")),
            ""
        );
        assert_eq!(resolve_storage_mem_path("Diff", None), "");
    }

    #[test]
    fn resolve_storage_mem_path_keeps_mem_for_full() {
        assert_eq!(
            resolve_storage_mem_path("Full", Some("/srv/fc/mem.bin")),
            "/srv/fc/mem.bin"
        );
        // Missing mem path on a Full snapshot becomes the empty string (the
        // value persisted to the snapshot row).
        assert_eq!(resolve_storage_mem_path("Full", None), "");
    }

    #[test]
    fn resolve_instantiate_name_prefers_override() {
        let snap_id = fixed_uuid();
        assert_eq!(
            resolve_instantiate_name(Some("clone-1".into()), Some("backup"), snap_id),
            "clone-1"
        );
        // An explicit empty override is preserved verbatim — same as the
        // original inline behavior.
        assert_eq!(
            resolve_instantiate_name(Some(String::new()), Some("backup"), snap_id),
            ""
        );
    }

    #[test]
    fn resolve_instantiate_name_falls_back_to_snapshot_then_uuid() {
        let snap_id = fixed_uuid();
        assert_eq!(
            resolve_instantiate_name(None, Some("nightly"), snap_id),
            "nightly"
        );
        assert_eq!(
            resolve_instantiate_name(None, None, snap_id),
            format!("snapshot-{snap_id}")
        );
    }

    #[test]
    fn snapshot_row_into_snapshot_preserves_fields() {
        let snap_id = Uuid::new_v4();
        let vm_id = Uuid::new_v4();
        let parent_id = Uuid::new_v4();
        let now = chrono::Utc::now();

        let row = SnapshotRow {
            id: snap_id,
            vm_id,
            snapshot_path: "/srv/snap.bin".into(),
            mem_path: "/srv/mem.bin".into(),
            size_bytes: 1234,
            state: "available".into(),
            snapshot_type: "Diff".into(),
            parent_id: Some(parent_id),
            track_dirty_pages: true,
            name: Some("nightly".into()),
            created_at: now,
            updated_at: now,
        };

        let snap: Snapshot = row.into();

        assert_eq!(snap.id, snap_id);
        assert_eq!(snap.vm_id, vm_id);
        assert_eq!(snap.snapshot_path, "/srv/snap.bin");
        assert_eq!(snap.mem_path, "/srv/mem.bin");
        assert_eq!(snap.size_bytes, 1234);
        assert_eq!(snap.state, "available");
        assert_eq!(snap.name.as_deref(), Some("nightly"));
        // The DB stores snapshot_type as a non-null string but the wire type
        // wraps it in `Option`. Capture the current always-Some behavior.
        assert_eq!(snap.snapshot_type.as_deref(), Some("Diff"));
        assert_eq!(snap.parent_id, Some(parent_id));
        assert!(snap.track_dirty_pages);
        assert_eq!(snap.created_at, now);
        assert_eq!(snap.updated_at, now);
    }
}
