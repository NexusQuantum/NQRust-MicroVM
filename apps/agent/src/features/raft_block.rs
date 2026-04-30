use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use nexus_raft_block::{
    openraft_entry, BlockCommand, BlockRaftTypeConfig, BlockResponse, BlockSnapshot,
    FileReplicaStore, InMemoryOpenraftBlockStore, RaftBlockError, VoteOutcome,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RaftBlockStatus {
    pub group_id: Uuid,
    pub state: String,
    pub data_path: String,
    pub transport: String,
    pub node_id: Option<u64>,
    pub capacity_bytes: Option<u64>,
    pub block_size: Option<u64>,
    pub last_applied_index: Option<u64>,
    pub compacted_through: Option<u64>,
    pub retained_log_entries: u64,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct RaftBlockHttpClient {
    client: reqwest::Client,
    base_url: String,
}

#[allow(dead_code)]
impl RaftBlockHttpClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: normalize_base_url(base_url.into()),
        }
    }

    pub fn with_client(client: reqwest::Client, base_url: impl Into<String>) -> Self {
        Self {
            client,
            base_url: normalize_base_url(base_url.into()),
        }
    }

    pub async fn create_group(&self, req: &CreateGroupReq) -> Result<(), RaftBlockTransportError> {
        self.post_empty("create", req).await
    }

    pub async fn append_entries(
        &self,
        req: &AppendEntriesReq,
    ) -> Result<Vec<BlockResponse>, RaftBlockTransportError> {
        self.post_json("append_entries", req).await
    }

    pub async fn openraft_append_entries(
        &self,
        group_id: Uuid,
        req: &openraft::raft::AppendEntriesRequest<BlockRaftTypeConfig>,
    ) -> Result<openraft::raft::AppendEntriesResponse<u64>, RaftBlockTransportError> {
        self.post_json(&format!("{group_id}/openraft/append_entries"), req)
            .await
    }

    pub async fn vote(&self, req: &VoteReq) -> Result<VoteOutcome, RaftBlockTransportError> {
        self.post_json("vote", req).await
    }

    pub async fn openraft_vote(
        &self,
        group_id: Uuid,
        req: &openraft::raft::VoteRequest<u64>,
    ) -> Result<openraft::raft::VoteResponse<u64>, RaftBlockTransportError> {
        self.post_json(&format!("{group_id}/openraft/vote"), req)
            .await
    }

    pub async fn install_snapshot(
        &self,
        req: &InstallSnapshotReq,
    ) -> Result<(), RaftBlockTransportError> {
        self.post_empty("install_snapshot", req).await
    }

    pub async fn openraft_install_snapshot(
        &self,
        group_id: Uuid,
        req: &openraft::raft::InstallSnapshotRequest<BlockRaftTypeConfig>,
    ) -> Result<openraft::raft::InstallSnapshotResponse<u64>, RaftBlockTransportError> {
        self.post_json(&format!("{group_id}/openraft/install_snapshot"), req)
            .await
    }

    pub async fn snapshot(&self, group_id: Uuid) -> Result<BlockSnapshot, RaftBlockTransportError> {
        let url = self.url(&format!("{group_id}/snapshot"));
        self.decode_response(self.client.get(url).send().await?)
            .await
    }

    pub async fn heartbeat(
        &self,
        req: &HeartbeatReq,
    ) -> Result<serde_json::Value, RaftBlockTransportError> {
        self.post_json("heartbeat", req).await
    }

    pub async fn status(&self, group_id: Uuid) -> Result<RaftBlockStatus, RaftBlockTransportError> {
        let url = self.url(&format!("{group_id}/status"));
        self.decode_response(self.client.get(url).send().await?)
            .await
    }

    pub async fn read(&self, req: &ReadReq) -> Result<ReadResp, RaftBlockTransportError> {
        self.post_json("read", req).await
    }

    fn url(&self, path: &str) -> String {
        format!("{}/{}", self.base_url, path.trim_start_matches('/'))
    }

    async fn post_empty<T: Serialize + ?Sized>(
        &self,
        path: &str,
        body: &T,
    ) -> Result<(), RaftBlockTransportError> {
        let _: serde_json::Value = self.post_json(path, body).await?;
        Ok(())
    }

    async fn post_json<T, R>(&self, path: &str, body: &T) -> Result<R, RaftBlockTransportError>
    where
        T: Serialize + ?Sized,
        R: for<'de> Deserialize<'de>,
    {
        let url = self.url(path);
        let response = self.client.post(url).json(body).send().await?;
        self.decode_response(response).await
    }

    async fn decode_response<R>(
        &self,
        response: reqwest::Response,
    ) -> Result<R, RaftBlockTransportError>
    where
        R: for<'de> Deserialize<'de>,
    {
        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(RaftBlockTransportError::Remote { status, body });
        }
        Ok(response.json().await?)
    }
}

#[derive(Debug, thiserror::Error)]
#[allow(dead_code)]
pub enum RaftBlockTransportError {
    #[error("raft block transport request failed: {0}")]
    Request(#[from] reqwest::Error),
    #[error("raft block remote returned {status}: {body}")]
    Remote {
        status: reqwest::StatusCode,
        body: String,
    },
}

#[allow(dead_code)]
fn normalize_base_url(mut base_url: String) -> String {
    while base_url.ends_with('/') {
        base_url.pop();
    }
    base_url
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

    async fn openraft_append_entries(
        &self,
        group_id: Uuid,
        req: openraft::raft::AppendEntriesRequest<BlockRaftTypeConfig>,
    ) -> Result<openraft::raft::AppendEntriesResponse<u64>, RaftBlockError> {
        let groups = self.groups.lock().await;
        let replica = groups
            .get(&group_id)
            .ok_or_else(|| RaftBlockError::Store(format!("group {group_id} not started")))?;
        if !req.entries.is_empty() {
            replica.append_openraft_entries(req.entries)?;
        }
        Ok(openraft::raft::AppendEntriesResponse::Success)
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

    async fn openraft_install_snapshot(
        &self,
        group_id: Uuid,
        req: openraft::raft::InstallSnapshotRequest<BlockRaftTypeConfig>,
    ) -> Result<openraft::raft::InstallSnapshotResponse<u64>, RaftBlockError> {
        let groups = self.groups.lock().await;
        let replica = groups
            .get(&group_id)
            .ok_or_else(|| RaftBlockError::Store(format!("group {group_id} not started")))?;
        if !req.done || req.offset != 0 {
            return Err(RaftBlockError::Store(
                "raft block prototype accepts only single-chunk Openraft snapshots".into(),
            ));
        }
        let snapshot: BlockSnapshot =
            serde_json::from_slice(&req.data).map_err(|e| RaftBlockError::Store(e.to_string()))?;
        replica.install_openraft_snapshot(&req.meta, &snapshot)?;
        Ok(openraft::raft::InstallSnapshotResponse { vote: req.vote })
    }

    async fn vote(&self, req: VoteReq) -> Result<VoteOutcome, RaftBlockError> {
        let groups = self.groups.lock().await;
        let replica = groups
            .get(&req.group_id)
            .ok_or_else(|| RaftBlockError::Store(format!("group {} not started", req.group_id)))?;
        replica.request_vote(req.term, req.candidate_id)
    }

    async fn openraft_vote(
        &self,
        group_id: Uuid,
        req: openraft::raft::VoteRequest<u64>,
    ) -> Result<openraft::raft::VoteResponse<u64>, RaftBlockError> {
        let groups = self.groups.lock().await;
        let replica = groups
            .get(&group_id)
            .ok_or_else(|| RaftBlockError::Store(format!("group {group_id} not started")))?;
        let candidate_id = req
            .vote
            .leader_id
            .voted_for()
            .ok_or_else(|| RaftBlockError::Store("Openraft vote has no candidate".into()))?;
        let outcome = replica.request_vote(req.vote.leader_id.term, candidate_id)?;
        let vote = openraft::Vote {
            leader_id: outcome
                .voted_for
                .map(|node_id| openraft::LeaderId::new(outcome.term, node_id))
                .unwrap_or_default(),
            committed: outcome.committed,
        };
        Ok(openraft::raft::VoteResponse {
            vote,
            vote_granted: outcome.granted,
            last_log_id: None,
        })
    }

    pub async fn status(&self, group_id: Uuid) -> RaftBlockStatus {
        let groups = self.groups.lock().await;
        if let Some(replica) = groups.get(&group_id) {
            RaftBlockStatus {
                group_id,
                state: "started".into(),
                data_path: "persistent_local_replica".into(),
                transport: "openraft_entry_local".into(),
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
                state: "not_started".into(),
                data_path: "raftblk_pending".into(),
                transport: "not_started".into(),
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateGroupReq {
    pub group_id: Uuid,
    pub node_id: u64,
    pub capacity_bytes: u64,
    pub block_size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppendReq {
    pub group_id: Uuid,
    pub term: u64,
    #[serde(default)]
    pub leader_id: Option<u64>,
    pub command: BlockCommand,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppendEntriesReq {
    pub group_id: Uuid,
    pub term: u64,
    pub leader_id: u64,
    pub entries: Vec<AppendEntryReq>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppendEntryReq {
    pub index: u64,
    pub command: BlockCommand,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallSnapshotReq {
    pub group_id: Uuid,
    pub snapshot: BlockSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StopGroupReq {
    pub group_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatReq {
    pub group_id: Uuid,
    pub term: u64,
    pub leader_id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoteReq {
    pub group_id: Uuid,
    pub term: u64,
    pub candidate_id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadReq {
    pub group_id: Uuid,
    pub offset: u64,
    pub len: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

pub async fn openraft_append_entries(
    State(state): State<Arc<RaftBlockState>>,
    Path(group_id): Path<Uuid>,
    Json(req): Json<openraft::raft::AppendEntriesRequest<BlockRaftTypeConfig>>,
) -> impl IntoResponse {
    match state.openraft_append_entries(group_id, req).await {
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

pub async fn openraft_vote(
    State(state): State<Arc<RaftBlockState>>,
    Path(group_id): Path<Uuid>,
    Json(req): Json<openraft::raft::VoteRequest<u64>>,
) -> impl IntoResponse {
    match state.openraft_vote(group_id, req).await {
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

pub async fn openraft_install_snapshot(
    State(state): State<Arc<RaftBlockState>>,
    Path(group_id): Path<Uuid>,
    Json(req): Json<openraft::raft::InstallSnapshotRequest<BlockRaftTypeConfig>>,
) -> impl IntoResponse {
    match state.openraft_install_snapshot(group_id, req).await {
        Ok(response) => (StatusCode::OK, Json(response)).into_response(),
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
        .route(
            "/:group_id/openraft/append_entries",
            post(openraft_append_entries),
        )
        .route("/:group_id/openraft/vote", post(openraft_vote))
        .route(
            "/:group_id/openraft/install_snapshot",
            post(openraft_install_snapshot),
        )
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
    use nexus_raft_block::openraft_log_id;

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
    async fn openraft_native_routes_accept_rpc_shapes() {
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

        let vote = openraft::Vote {
            leader_id: openraft::LeaderId::new(2, 2),
            committed: false,
        };
        let response = openraft_vote(
            State(state.clone()),
            Path(group_id),
            Json(openraft::raft::VoteRequest {
                vote,
                last_log_id: None,
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let response: openraft::raft::VoteResponse<u64> = serde_json::from_slice(&body).unwrap();
        assert!(response.vote_granted);

        let response = openraft_append_entries(
            State(state.clone()),
            Path(group_id),
            Json(openraft::raft::AppendEntriesRequest {
                vote,
                prev_log_id: None,
                entries: vec![openraft_entry(
                    2,
                    1,
                    1,
                    BlockCommand::Write {
                        offset: 0,
                        bytes: vec![11; 512],
                    },
                )],
                leader_commit: Some(openraft_log_id(2, 1, 1)),
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
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let read: ReadResp = serde_json::from_slice(&body).unwrap();
        assert_eq!(read.bytes[0], 11);

        let snapshot = BlockSnapshot {
            replica_id: 9,
            last_included_index: 3,
            highest_term_seen: 3,
            bytes: vec![4; 4096],
        };
        let response = openraft_install_snapshot(
            State(state.clone()),
            Path(group_id),
            Json(openraft::raft::InstallSnapshotRequest {
                vote,
                meta: openraft::SnapshotMeta {
                    last_log_id: Some(openraft_log_id(3, 1, 3)),
                    last_membership: openraft::StoredMembership::default(),
                    snapshot_id: "native-test".into(),
                },
                offset: 0,
                data: serde_json::to_vec(&snapshot).unwrap(),
                done: true,
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);

        let status = state.status(group_id).await;
        assert_eq!(status.last_applied_index, Some(3));
        let read = state
            .read(ReadReq {
                group_id,
                offset: 0,
                len: 512,
            })
            .await
            .unwrap();
        assert_eq!(read.bytes[0], 4);
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

    #[tokio::test]
    async fn http_client_drives_remote_group_routes() {
        let dir = tempfile::tempdir().unwrap();
        let group_id = Uuid::new_v4();
        let state = Arc::new(RaftBlockState::new(dir.path()));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(listener, router(state)).await.unwrap();
        });
        let client =
            RaftBlockHttpClient::with_client(reqwest::Client::new(), format!("http://{addr}"));

        client
            .create_group(&CreateGroupReq {
                group_id,
                node_id: 1,
                capacity_bytes: 4096,
                block_size: 512,
            })
            .await
            .unwrap();
        let vote_outcome = client
            .vote(&VoteReq {
                group_id,
                term: 2,
                candidate_id: 2,
            })
            .await
            .unwrap();
        assert!(vote_outcome.granted);
        let native_request_vote = openraft::Vote {
            leader_id: openraft::LeaderId::new(2, 2),
            committed: false,
        };

        let response = client
            .append_entries(&AppendEntriesReq {
                group_id,
                term: 2,
                leader_id: 1,
                entries: vec![AppendEntryReq {
                    index: 1,
                    command: BlockCommand::Write {
                        offset: 0,
                        bytes: vec![9; 512],
                    },
                }],
            })
            .await
            .unwrap();
        assert_eq!(response[0].applied_index, 1);
        let native_append = client
            .openraft_append_entries(
                group_id,
                &openraft::raft::AppendEntriesRequest {
                    vote: native_request_vote,
                    prev_log_id: Some(openraft_log_id(2, 1, 1)),
                    entries: vec![openraft_entry(
                        2,
                        1,
                        2,
                        BlockCommand::Write {
                            offset: 512,
                            bytes: vec![8; 512],
                        },
                    )],
                    leader_commit: Some(openraft_log_id(2, 1, 2)),
                },
            )
            .await
            .unwrap();
        assert_eq!(
            native_append,
            openraft::raft::AppendEntriesResponse::Success
        );
        let native_vote = client
            .openraft_vote(
                group_id,
                &openraft::raft::VoteRequest {
                    vote: native_request_vote,
                    last_log_id: Some(openraft_log_id(2, 1, 2)),
                },
            )
            .await
            .unwrap();
        assert!(native_vote.vote_granted);
        let read = client
            .read(&ReadReq {
                group_id,
                offset: 0,
                len: 512,
            })
            .await
            .unwrap();
        assert_eq!(read.bytes[0], 9);

        let status = client.status(group_id).await.unwrap();
        assert_eq!(status.state, "started");
        assert_eq!(status.transport, "openraft_entry_local");

        let heartbeat = client
            .heartbeat(&HeartbeatReq {
                group_id,
                term: 2,
                leader_id: 1,
            })
            .await
            .unwrap();
        assert_eq!(heartbeat["status"]["state"], "started");

        let snapshot = client.snapshot(group_id).await.unwrap();
        let target_group = Uuid::new_v4();
        client
            .create_group(&CreateGroupReq {
                group_id: target_group,
                node_id: 2,
                capacity_bytes: 4096,
                block_size: 512,
            })
            .await
            .unwrap();
        client
            .install_snapshot(&InstallSnapshotReq {
                group_id: target_group,
                snapshot,
            })
            .await
            .unwrap();
        let native_snapshot = BlockSnapshot {
            replica_id: 2,
            last_included_index: 4,
            highest_term_seen: 4,
            bytes: vec![6; 4096],
        };
        client
            .openraft_install_snapshot(
                target_group,
                &openraft::raft::InstallSnapshotRequest {
                    vote: native_request_vote,
                    meta: openraft::SnapshotMeta {
                        last_log_id: Some(openraft_log_id(4, 1, 4)),
                        last_membership: openraft::StoredMembership::default(),
                        snapshot_id: "http-native-test".into(),
                    },
                    offset: 0,
                    data: serde_json::to_vec(&native_snapshot).unwrap(),
                    done: true,
                },
            )
            .await
            .unwrap();
        let restored = client
            .read(&ReadReq {
                group_id: target_group,
                offset: 0,
                len: 512,
            })
            .await
            .unwrap();
        assert_eq!(restored.bytes[0], 6);

        server.abort();
    }

    #[tokio::test]
    async fn http_client_surfaces_remote_errors() {
        let state = Arc::new(RaftBlockState::new(tempfile::tempdir().unwrap().path()));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(listener, router(state)).await.unwrap();
        });
        let client = RaftBlockHttpClient::new(format!("http://{addr}/"));

        let err = client
            .append_entries(&AppendEntriesReq {
                group_id: Uuid::new_v4(),
                term: 1,
                leader_id: 1,
                entries: vec![],
            })
            .await
            .unwrap_err();
        match err {
            RaftBlockTransportError::Remote { status, body } => {
                assert_eq!(status, reqwest::StatusCode::BAD_REQUEST);
                assert!(body.contains("not started"));
            }
            other => panic!("unexpected error: {other}"),
        }

        server.abort();
    }
}
