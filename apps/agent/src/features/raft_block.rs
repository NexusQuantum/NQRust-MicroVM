use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use nexus_raft_block::{
    BlockCommand, BlockResponse, FileReplicaStore, PersistentReplica, RaftBlockError,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

#[derive(Clone)]
pub struct RaftBlockState {
    base_dir: PathBuf,
    groups: Arc<Mutex<HashMap<Uuid, PersistentReplica>>>,
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

    async fn create_group(&self, req: CreateGroupReq) -> Result<(), RaftBlockError> {
        let store = self.store_for(req.group_id, req.node_id);
        let replica = if let Some(existing) = PersistentReplica::open(store.clone())? {
            existing
        } else {
            PersistentReplica::create(store, req.node_id, req.capacity_bytes, req.block_size)?
        };
        self.groups.lock().await.insert(req.group_id, replica);
        Ok(())
    }

    async fn append(&self, req: AppendReq) -> Result<BlockResponse, RaftBlockError> {
        let mut groups = self.groups.lock().await;
        let replica = groups
            .get_mut(&req.group_id)
            .ok_or_else(|| RaftBlockError::Store(format!("group {} not started", req.group_id)))?;
        replica.append_command(req.term, req.command)
    }

    async fn status(&self, group_id: Uuid) -> RaftBlockStatus {
        let groups = self.groups.lock().await;
        if let Some(replica) = groups.get(&group_id) {
            RaftBlockStatus {
                group_id,
                state: "started",
                data_path: "persistent_local_replica",
                applied_entries: replica.log().len() as u64,
            }
        } else {
            RaftBlockStatus {
                group_id,
                state: "not_started",
                data_path: "raftblk_pending",
                applied_entries: 0,
            }
        }
    }
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
    pub command: BlockCommand,
}

#[derive(Debug, Serialize)]
pub struct RaftBlockStatus {
    pub group_id: Uuid,
    pub state: &'static str,
    pub data_path: &'static str,
    pub applied_entries: u64,
}

#[derive(Debug, Deserialize)]
pub struct RaftBlockRpcEnvelope {
    pub group_id: Uuid,
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

pub async fn vote(Json(req): Json<RaftBlockRpcEnvelope>) -> impl IntoResponse {
    not_implemented(req.group_id, "vote")
}

pub async fn install_snapshot(Json(req): Json<RaftBlockRpcEnvelope>) -> impl IntoResponse {
    not_implemented(req.group_id, "install_snapshot")
}

pub async fn heartbeat(Json(req): Json<RaftBlockRpcEnvelope>) -> impl IntoResponse {
    not_implemented(req.group_id, "heartbeat")
}

fn not_implemented(group_id: Uuid, rpc: &'static str) -> axum::response::Response {
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(serde_json::json!({
            "group_id": group_id,
            "rpc": rpc,
            "error": "raft_block transport awaits Openraft adapter"
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
        .route("/create", post(create))
        .route("/append", post(append))
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
        assert_eq!(status["applied_entries"], 1);
    }

    #[tokio::test]
    async fn vote_is_explicitly_not_implemented() {
        let response = vote(Json(RaftBlockRpcEnvelope {
            group_id: Uuid::new_v4(),
        }))
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::NOT_IMPLEMENTED);
    }
}
