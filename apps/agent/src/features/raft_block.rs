use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use nexus_raft_block::{
    openraft_entry, BlockCommand, BlockResponse, BlockSnapshot, FileReplicaStore,
    InMemoryOpenraftBlockStore, RaftBlockError, VoteOutcome,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct RaftBlockState {
    base_dir: PathBuf,
    groups: Arc<Mutex<HashMap<Uuid, InMemoryOpenraftBlockStore>>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RaftBlockStatus {
    pub group_id: Uuid,
    pub state: &'static str,
    pub data_path: &'static str,
    pub transport: &'static str,
    pub node_id: Option<u64>,
    pub capacity_bytes: Option<u64>,
    pub block_size: Option<u64>,
    pub last_applied_index: Option<u64>,
    pub compacted_through: Option<u64>,
    pub retained_log_entries: u64,
}

impl RaftBlockState {
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            base_dir: base_dir.into(),
            groups: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn store_for(&self, group_id: Uuid, node_id: u64) -> FileReplicaStore {
        FileReplicaStore::new(
            self.base_dir
                .join("raft-block")
                .join(group_id.to_string())
                .join(format!("node-{node_id}.json")),
        )
    }

    pub async fn ensure_group(
        &self,
        group_id: Uuid,
        node_id: u64,
        capacity_bytes: u64,
        block_size: u64,
    ) -> Result<(), RaftBlockError> {
        self.create_group(CreateGroupReq {
            group_id,
            node_id,
            capacity_bytes,
            block_size,
        })
        .await
    }

    pub async fn stop_group(&self, group_id: Uuid) -> bool {
        self.groups.lock().await.remove(&group_id).is_some()
    }

    pub async fn load_existing_groups(&self) -> Result<usize, RaftBlockError> {
        let root = self.base_dir.join("raft-block");
        if !root.exists() {
            return Ok(0);
        }
        let mut loaded = 0;
        let mut groups = self.groups.lock().await;
        let dirs = std::fs::read_dir(&root)
            .map_err(|e| RaftBlockError::Store(format!("read {root:?}: {e}")))?;
        for dir in dirs {
            let dir = dir.map_err(|e| RaftBlockError::Store(format!("read {root:?}: {e}")))?;
            if !dir
                .file_type()
                .map_err(|e| RaftBlockError::Store(format!("stat {:?}: {e}", dir.path())))?
                .is_dir()
            {
                continue;
            }
            let Some(group_id) = dir
                .file_name()
                .to_str()
                .and_then(|raw| Uuid::parse_str(raw).ok())
            else {
                continue;
            };
            if groups.contains_key(&group_id) {
                continue;
            }
            let files = std::fs::read_dir(dir.path())
                .map_err(|e| RaftBlockError::Store(format!("read {:?}: {e}", dir.path())))?;
            for file in files {
                let file =
                    file.map_err(|e| RaftBlockError::Store(format!("read {:?}: {e}", dir.path())))?;
                if !file
                    .file_type()
                    .map_err(|e| RaftBlockError::Store(format!("stat {:?}: {e}", file.path())))?
                    .is_file()
                {
                    continue;
                }
                if !file.file_name().to_string_lossy().starts_with("node-") {
                    continue;
                }
                let Some(store) =
                    InMemoryOpenraftBlockStore::open_existing(FileReplicaStore::new(file.path()))?
                else {
                    continue;
                };
                groups.insert(group_id, store);
                loaded += 1;
                break;
            }
        }
        Ok(loaded)
    }

    async fn create_group(&self, req: CreateGroupReq) -> Result<(), RaftBlockError> {
        let mut groups = self.groups.lock().await;
        if let Some(existing) = groups.get(&req.group_id) {
            validate_existing_group(existing, &req)?;
            return Ok(());
        }
        let store = self.store_for(req.group_id, req.node_id);
        let replica = InMemoryOpenraftBlockStore::open_or_create(
            store,
            req.node_id,
            req.capacity_bytes,
            req.block_size,
        )?;
        groups.insert(req.group_id, replica);
        Ok(())
    }

    async fn append(&self, req: AppendReq) -> Result<BlockResponse, RaftBlockError> {
        let mut groups = self.groups.lock().await;
        let replica = groups
            .get_mut(&req.group_id)
            .ok_or_else(|| RaftBlockError::Store(format!("group {} not started", req.group_id)))?;
        replica.append_command(
            req.term,
            req.leader_id.unwrap_or(replica.node_id()?),
            req.command,
        )
    }

    pub async fn append_command(
        &self,
        group_id: Uuid,
        term: u64,
        leader_id: Option<u64>,
        command: BlockCommand,
    ) -> Result<BlockResponse, RaftBlockError> {
        self.append(AppendReq {
            group_id,
            term,
            leader_id,
            command,
        })
        .await
    }

    async fn append_entries(
        &self,
        req: AppendEntriesReq,
    ) -> Result<Vec<BlockResponse>, RaftBlockError> {
        let groups = self.groups.lock().await;
        let replica = groups
            .get(&req.group_id)
            .ok_or_else(|| RaftBlockError::Store(format!("group {} not started", req.group_id)))?;
        let entries = req
            .entries
            .into_iter()
            .map(|entry| openraft_entry(req.term, req.leader_id, entry.index, entry.command));
        replica.append_openraft_entries(entries)
    }

    async fn snapshot(&self, group_id: Uuid) -> Result<BlockSnapshot, RaftBlockError> {
        let groups = self.groups.lock().await;
        let replica = groups
            .get(&group_id)
            .ok_or_else(|| RaftBlockError::Store(format!("group {group_id} not started")))?;
        replica.block_snapshot()
    }

    pub async fn snapshot_bytes(&self, group_id: Uuid) -> Result<Vec<u8>, RaftBlockError> {
        self.snapshot(group_id).await.map(|snapshot| snapshot.bytes)
    }

    async fn read(&self, req: ReadReq) -> Result<ReadResp, RaftBlockError> {
        let groups = self.groups.lock().await;
        let replica = groups
            .get(&req.group_id)
            .ok_or_else(|| RaftBlockError::Store(format!("group {} not started", req.group_id)))?;
        let bytes = replica.read_range(req.offset, req.len)?;
        Ok(ReadResp { bytes })
    }

    async fn install_snapshot(&self, req: InstallSnapshotReq) -> Result<(), RaftBlockError> {
        let mut groups = self.groups.lock().await;
        let replica = groups
            .get_mut(&req.group_id)
            .ok_or_else(|| RaftBlockError::Store(format!("group {} not started", req.group_id)))?;
        replica.install_block_snapshot(&req.snapshot)
    }

    async fn vote(&self, req: VoteReq) -> Result<VoteOutcome, RaftBlockError> {
        let groups = self.groups.lock().await;
        let replica = groups
            .get(&req.group_id)
            .ok_or_else(|| RaftBlockError::Store(format!("group {} not started", req.group_id)))?;
        replica.request_vote(req.term, req.candidate_id)
    }

    pub async fn status(&self, group_id: Uuid) -> RaftBlockStatus {
        let groups = self.groups.lock().await;
        if let Some(replica) = groups.get(&group_id) {
            RaftBlockStatus {
                group_id,
                state: "started",
                data_path: "persistent_local_replica",
                transport: "openraft_entry_local",
                node_id: replica.node_id().ok(),
                capacity_bytes: replica.capacity_bytes().ok(),
                block_size: replica.block_size().ok(),
                last_applied_index: replica.last_applied_index().ok(),
                compacted_through: replica.compacted_through().ok(),
                retained_log_entries: replica.retained_log_entries().unwrap_or(0),
            }
        } else {
            RaftBlockStatus {
                group_id,
                state: "not_started",
                data_path: "raftblk_pending",
                transport: "not_started",
                node_id: None,
                capacity_bytes: None,
                block_size: None,
                last_applied_index: None,
                compacted_through: None,
                retained_log_entries: 0,
            }
        }
    }
}

fn validate_existing_group(
    existing: &InMemoryOpenraftBlockStore,
    req: &CreateGroupReq,
) -> Result<(), RaftBlockError> {
    if existing.node_id()? != req.node_id
        || existing.capacity_bytes()? != req.capacity_bytes
        || existing.block_size()? != req.block_size
    {
        return Err(RaftBlockError::Store(format!(
            "group {} already exists with node_id={}, capacity_bytes={}, block_size={}; requested node_id={}, capacity_bytes={}, block_size={}",
            req.group_id,
            existing.node_id()?,
            existing.capacity_bytes()?,
            existing.block_size()?,
            req.node_id,
            req.capacity_bytes,
            req.block_size
        )));
    }
    Ok(())
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateGroupReq {
    pub group_id: Uuid,
    pub node_id: u64,
    pub capacity_bytes: u64,
    pub block_size: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AppendReq {
    pub group_id: Uuid,
    pub term: u64,
    #[serde(default)]
    pub leader_id: Option<u64>,
    pub command: BlockCommand,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AppendEntriesReq {
    pub group_id: Uuid,
    pub term: u64,
    pub leader_id: u64,
    pub entries: Vec<AppendEntryReq>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AppendEntryReq {
    pub index: u64,
    pub command: BlockCommand,
}

#[derive(Debug, Clone, Deserialize)]
pub struct InstallSnapshotReq {
    pub group_id: Uuid,
    pub snapshot: BlockSnapshot,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StopGroupReq {
    pub group_id: Uuid,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HeartbeatReq {
    pub group_id: Uuid,
    pub term: u64,
    pub leader_id: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VoteReq {
    pub group_id: Uuid,
    pub term: u64,
    pub candidate_id: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReadReq {
    pub group_id: Uuid,
    pub offset: u64,
    pub len: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReadResp {
    pub bytes: Vec<u8>,
}

pub async fn create(
    State(state): State<Arc<RaftBlockState>>,
    Json(req): Json<CreateGroupReq>,
) -> impl IntoResponse {
    match state.create_group(req).await {
        Ok(()) => (StatusCode::OK, Json(serde_json::json!({}))).into_response(),
        Err(err) => error_response(StatusCode::BAD_REQUEST, err),
    }
}

pub async fn status(
    State(state): State<Arc<RaftBlockState>>,
    Path(group_id): Path<Uuid>,
) -> impl IntoResponse {
    (StatusCode::OK, Json(state.status(group_id).await))
}

pub async fn append(
    State(state): State<Arc<RaftBlockState>>,
    Json(req): Json<AppendReq>,
) -> impl IntoResponse {
    match state.append(req).await {
        Ok(response) => (StatusCode::OK, Json(response)).into_response(),
        Err(err) => error_response(StatusCode::BAD_REQUEST, err),
    }
}

pub async fn append_entries(
    State(state): State<Arc<RaftBlockState>>,
    Json(req): Json<AppendEntriesReq>,
) -> impl IntoResponse {
    match state.append_entries(req).await {
        Ok(response) => (StatusCode::OK, Json(response)).into_response(),
        Err(err) => error_response(StatusCode::BAD_REQUEST, err),
    }
}

pub async fn stop(
    State(state): State<Arc<RaftBlockState>>,
    Json(req): Json<StopGroupReq>,
) -> impl IntoResponse {
    let stopped = state.stop_group(req.group_id).await;
    (
        StatusCode::OK,
        Json(serde_json::json!({ "stopped": stopped })),
    )
        .into_response()
}

pub async fn snapshot(
    State(state): State<Arc<RaftBlockState>>,
    Path(group_id): Path<Uuid>,
) -> impl IntoResponse {
    match state.snapshot(group_id).await {
        Ok(snapshot) => (StatusCode::OK, Json(snapshot)).into_response(),
        Err(err) => error_response(StatusCode::BAD_REQUEST, err),
    }
}

pub async fn read(
    State(state): State<Arc<RaftBlockState>>,
    Json(req): Json<ReadReq>,
) -> impl IntoResponse {
    match state.read(req).await {
        Ok(response) => (StatusCode::OK, Json(response)).into_response(),
        Err(err) => error_response(StatusCode::BAD_REQUEST, err),
    }
}

pub async fn vote(
    State(state): State<Arc<RaftBlockState>>,
    Json(req): Json<VoteReq>,
) -> impl IntoResponse {
    match state.vote(req).await {
        Ok(response) => (StatusCode::OK, Json(response)).into_response(),
        Err(err) => error_response(StatusCode::BAD_REQUEST, err),
    }
}

pub async fn install_snapshot(
    State(state): State<Arc<RaftBlockState>>,
    Json(req): Json<InstallSnapshotReq>,
) -> impl IntoResponse {
    match state.install_snapshot(req).await {
        Ok(()) => (StatusCode::OK, Json(serde_json::json!({}))).into_response(),
        Err(err) => error_response(StatusCode::BAD_REQUEST, err),
    }
}

pub async fn heartbeat(
    State(state): State<Arc<RaftBlockState>>,
    Json(req): Json<HeartbeatReq>,
) -> impl IntoResponse {
    let status = state.status(req.group_id).await;
    if status.state != "started" {
        return error_response(
            StatusCode::BAD_REQUEST,
            RaftBlockError::Store(format!("group {} not started", req.group_id)),
        );
    }
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "group_id": req.group_id,
            "term": req.term,
            "leader_id": req.leader_id,
            "status": status
        })),
    )
        .into_response()
}

fn error_response(status: StatusCode, err: RaftBlockError) -> axum::response::Response {
    (
        status,
        Json(serde_json::json!({
            "error": err.to_string()
        })),
    )
        .into_response()
}

pub fn router(state: Arc<RaftBlockState>) -> Router {
    Router::new()
        .route("/:group_id/status", get(status))
        .route("/:group_id/snapshot", get(snapshot))
        .route("/create", post(create))
        .route("/append", post(append))
        .route("/append_entries", post(append_entries))
        .route("/read", post(read))
        .route("/stop", post(stop))
        .route("/vote", post(vote))
        .route("/install_snapshot", post(install_snapshot))
        .route("/heartbeat", post(heartbeat))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;

    #[tokio::test]
    async fn status_reports_pending_data_path() {
        let group_id = Uuid::new_v4();
        let state = Arc::new(RaftBlockState::new(tempfile::tempdir().unwrap().path()));
        let response = status(State(state), Path(group_id)).await.into_response();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn append_is_rejected_before_group_start() {
        let state = Arc::new(RaftBlockState::new(tempfile::tempdir().unwrap().path()));
        let response = append(
            State(state),
            Json(AppendReq {
                group_id: Uuid::new_v4(),
                term: 1,
                leader_id: None,
                command: BlockCommand::Flush,
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn create_append_and_reopen_group_state() {
        let dir = tempfile::tempdir().unwrap();
        let group_id = Uuid::new_v4();
        let state = Arc::new(RaftBlockState::new(dir.path()));
        let response = create(
            State(state.clone()),
            Json(CreateGroupReq {
                group_id,
                node_id: 1,
                capacity_bytes: 4096,
                block_size: 512,
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);

        let response = append(
            State(state),
            Json(AppendReq {
                group_id,
                term: 1,
                leader_id: None,
                command: BlockCommand::Write {
                    offset: 0,
                    bytes: vec![5; 512],
                },
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);

        let restarted = Arc::new(RaftBlockState::new(dir.path()));
        let response = create(
            State(restarted.clone()),
            Json(CreateGroupReq {
                group_id,
                node_id: 1,
                capacity_bytes: 4096,
                block_size: 512,
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);
        let response = status(State(restarted), Path(group_id))
            .await
            .into_response();
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let status: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(status["state"], "started");
        assert_eq!(status["retained_log_entries"], 1);
        assert_eq!(status["last_applied_index"], 1);
        assert_eq!(status["node_id"], 1);
    }

    #[tokio::test]
    async fn startup_loads_existing_group_state() {
        let dir = tempfile::tempdir().unwrap();
        let group_id = Uuid::new_v4();
        let state = Arc::new(RaftBlockState::new(dir.path()));
        let response = create(
            State(state.clone()),
            Json(CreateGroupReq {
                group_id,
                node_id: 1,
                capacity_bytes: 4096,
                block_size: 512,
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);
        let response = append(
            State(state),
            Json(AppendReq {
                group_id,
                term: 1,
                leader_id: None,
                command: BlockCommand::Write {
                    offset: 0,
                    bytes: vec![5; 512],
                },
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);

        let restarted = Arc::new(RaftBlockState::new(dir.path()));
        assert_eq!(restarted.load_existing_groups().await.unwrap(), 1);
        let status = restarted.status(group_id).await;
        assert_eq!(status.state, "started");
        assert_eq!(status.retained_log_entries, 1);
        assert_eq!(status.last_applied_index, Some(1));
        let response = read(
            State(restarted),
            Json(ReadReq {
                group_id,
                offset: 0,
                len: 512,
            }),
        )
        .await
        .into_response();
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let response: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(response["bytes"].as_array().unwrap()[0], 5);
    }

    #[tokio::test]
    async fn create_rejects_mismatched_existing_group_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let group_id = Uuid::new_v4();
        let state = Arc::new(RaftBlockState::new(dir.path()));
        let response = create(
            State(state.clone()),
            Json(CreateGroupReq {
                group_id,
                node_id: 1,
                capacity_bytes: 4096,
                block_size: 512,
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);

        let response = create(
            State(state.clone()),
            Json(CreateGroupReq {
                group_id,
                node_id: 1,
                capacity_bytes: 8192,
                block_size: 512,
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let restarted = Arc::new(RaftBlockState::new(dir.path()));
        let response = create(
            State(restarted),
            Json(CreateGroupReq {
                group_id,
                node_id: 1,
                capacity_bytes: 8192,
                block_size: 512,
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn snapshot_and_install_snapshot_are_durable() {
        let dir = tempfile::tempdir().unwrap();
        let source_group = Uuid::new_v4();
        let target_group = Uuid::new_v4();
        let state = Arc::new(RaftBlockState::new(dir.path()));

        let response = create(
            State(state.clone()),
            Json(CreateGroupReq {
                group_id: source_group,
                node_id: 1,
                capacity_bytes: 4096,
                block_size: 512,
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);
        let response = append(
            State(state.clone()),
            Json(AppendReq {
                group_id: source_group,
                term: 1,
                leader_id: None,
                command: BlockCommand::Write {
                    offset: 0,
                    bytes: vec![7; 512],
                },
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);

        let response = snapshot(State(state.clone()), Path(source_group))
            .await
            .into_response();
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let source_snapshot: BlockSnapshot = serde_json::from_slice(&body).unwrap();

        let response = create(
            State(state.clone()),
            Json(CreateGroupReq {
                group_id: target_group,
                node_id: 2,
                capacity_bytes: 4096,
                block_size: 512,
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);
        let response = install_snapshot(
            State(state.clone()),
            Json(InstallSnapshotReq {
                group_id: target_group,
                snapshot: source_snapshot,
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);
        drop(state);

        let restarted = Arc::new(RaftBlockState::new(dir.path()));
        let response = create(
            State(restarted.clone()),
            Json(CreateGroupReq {
                group_id: target_group,
                node_id: 2,
                capacity_bytes: 4096,
                block_size: 512,
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);
        let response = snapshot(State(restarted), Path(target_group))
            .await
            .into_response();
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let snapshot: BlockSnapshot = serde_json::from_slice(&body).unwrap();
        assert_eq!(&snapshot.bytes[0..512], &[7; 512]);
    }

    #[tokio::test]
    async fn read_returns_persisted_range_and_rejects_bounds() {
        let dir = tempfile::tempdir().unwrap();
        let group_id = Uuid::new_v4();
        let state = Arc::new(RaftBlockState::new(dir.path()));
        let response = create(
            State(state.clone()),
            Json(CreateGroupReq {
                group_id,
                node_id: 1,
                capacity_bytes: 4096,
                block_size: 512,
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);
        let response = append(
            State(state.clone()),
            Json(AppendReq {
                group_id,
                term: 1,
                leader_id: None,
                command: BlockCommand::Write {
                    offset: 0,
                    bytes: vec![3; 512],
                },
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);

        let response = read(
            State(state.clone()),
            Json(ReadReq {
                group_id,
                offset: 0,
                len: 512,
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let response: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(response["bytes"].as_array().unwrap().len(), 512);

        let response = read(
            State(state),
            Json(ReadReq {
                group_id,
                offset: 4096,
                len: 1,
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn append_entries_applies_openraft_shaped_batch() {
        let dir = tempfile::tempdir().unwrap();
        let group_id = Uuid::new_v4();
        let state = Arc::new(RaftBlockState::new(dir.path()));
        let response = create(
            State(state.clone()),
            Json(CreateGroupReq {
                group_id,
                node_id: 1,
                capacity_bytes: 4096,
                block_size: 512,
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);

        let response = append_entries(
            State(state.clone()),
            Json(AppendEntriesReq {
                group_id,
                term: 2,
                leader_id: 1,
                entries: vec![
                    AppendEntryReq {
                        index: 1,
                        command: BlockCommand::Write {
                            offset: 0,
                            bytes: vec![2; 512],
                        },
                    },
                    AppendEntryReq {
                        index: 2,
                        command: BlockCommand::Flush,
                    },
                ],
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);

        let response = read(
            State(state),
            Json(ReadReq {
                group_id,
                offset: 0,
                len: 512,
            }),
        )
        .await
        .into_response();
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let response: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(response["bytes"].as_array().unwrap()[0], 2);
    }

    #[tokio::test]
    async fn stop_unloads_group_but_preserves_durable_state() {
        let dir = tempfile::tempdir().unwrap();
        let group_id = Uuid::new_v4();
        let state = Arc::new(RaftBlockState::new(dir.path()));
        let response = create(
            State(state.clone()),
            Json(CreateGroupReq {
                group_id,
                node_id: 1,
                capacity_bytes: 4096,
                block_size: 512,
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);
        let response = append(
            State(state.clone()),
            Json(AppendReq {
                group_id,
                term: 1,
                leader_id: None,
                command: BlockCommand::Write {
                    offset: 0,
                    bytes: vec![4; 512],
                },
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);

        let response = stop(State(state.clone()), Json(StopGroupReq { group_id }))
            .await
            .into_response();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(state.status(group_id).await.state, "not_started");

        let response = create(
            State(state.clone()),
            Json(CreateGroupReq {
                group_id,
                node_id: 1,
                capacity_bytes: 4096,
                block_size: 512,
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);
        let response = read(
            State(state),
            Json(ReadReq {
                group_id,
                offset: 0,
                len: 512,
            }),
        )
        .await
        .into_response();
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let response: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(response["bytes"].as_array().unwrap().len(), 512);
    }

    #[tokio::test]
    async fn heartbeat_reports_started_group_status() {
        let dir = tempfile::tempdir().unwrap();
        let group_id = Uuid::new_v4();
        let state = Arc::new(RaftBlockState::new(dir.path()));
        let response = create(
            State(state.clone()),
            Json(CreateGroupReq {
                group_id,
                node_id: 1,
                capacity_bytes: 4096,
                block_size: 512,
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);

        let response = heartbeat(
            State(state.clone()),
            Json(HeartbeatReq {
                group_id,
                term: 3,
                leader_id: 1,
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let response: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(response["term"], 3);
        assert_eq!(response["leader_id"], 1);
        assert_eq!(response["status"]["state"], "started");
        assert_eq!(response["status"]["transport"], "openraft_entry_local");
    }

    #[tokio::test]
    async fn heartbeat_rejects_unstarted_group() {
        let state = Arc::new(RaftBlockState::new(tempfile::tempdir().unwrap().path()));
        let response = heartbeat(
            State(state),
            Json(HeartbeatReq {
                group_id: Uuid::new_v4(),
                term: 1,
                leader_id: 1,
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn vote_grants_once_and_rejects_conflicting_same_term_candidate() {
        let dir = tempfile::tempdir().unwrap();
        let group_id = Uuid::new_v4();
        let state = Arc::new(RaftBlockState::new(dir.path()));
        let response = create(
            State(state.clone()),
            Json(CreateGroupReq {
                group_id,
                node_id: 1,
                capacity_bytes: 4096,
                block_size: 512,
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);

        let response = vote(
            State(state.clone()),
            Json(VoteReq {
                group_id,
                term: 2,
                candidate_id: 2,
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let response: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(response["granted"], true);
        assert_eq!(response["term"], 2);
        assert_eq!(response["voted_for"], 2);

        let response = vote(
            State(state),
            Json(VoteReq {
                group_id,
                term: 2,
                candidate_id: 3,
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let response: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(response["granted"], false);
        assert_eq!(response["voted_for"], 2);
    }
}
