use axum::{
    extract::Path,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct RaftBlockStatus {
    pub group_id: Uuid,
    pub state: &'static str,
    pub data_path: &'static str,
}

#[derive(Debug, Deserialize)]
pub struct RaftBlockRpcEnvelope {
    pub group_id: Uuid,
}

pub async fn status(Path(group_id): Path<Uuid>) -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(RaftBlockStatus {
            group_id,
            state: "not_started",
            data_path: "raftblk_pending",
        }),
    )
}

pub async fn append(Json(req): Json<RaftBlockRpcEnvelope>) -> impl IntoResponse {
    not_implemented(req.group_id, "append_entries")
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

pub fn router() -> Router {
    Router::new()
        .route("/:group_id/status", get(status))
        .route("/append", post(append))
        .route("/vote", post(vote))
        .route("/install_snapshot", post(install_snapshot))
        .route("/heartbeat", post(heartbeat))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn status_reports_pending_data_path() {
        let group_id = Uuid::new_v4();
        let response = status(Path(group_id)).await.into_response();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn append_is_explicitly_not_implemented() {
        let response = append(Json(RaftBlockRpcEnvelope {
            group_id: Uuid::new_v4(),
        }))
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::NOT_IMPLEMENTED);
    }
}
