//! `BlockBackend` trait and the `RaftBlockBackend` HTTP implementation.
//!
//! The trait is the seam between the daemon's virtio-blk request loop
//! (in the binary) and "where the bytes live" (here). The only shipped
//! impl talks to a local agent over HTTP and lets the agent's
//! `RaftBlockState` apply writes through `runtime_client_write` (real
//! Raft) or `append_command` (legacy storage path, gated by config).
//!
//! Test impls live alongside their consumers; this crate provides the
//! `InMemoryBlockBackend` for the request-loop tests.

use crate::request::{
    format_serial_id, BlockRequest, BlockRequestKind, BlockResponse, VirtioBlkStatus,
};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum BlockBackendError {
    #[error("backend transport: {0}")]
    Transport(String),
    #[error("backend rejected request: {0}")]
    Rejected(String),
    #[error("backend returned malformed response: {0}")]
    MalformedResponse(String),
    #[error("backend not configured: {0}")]
    NotConfigured(String),
}

#[async_trait::async_trait]
pub trait BlockBackend: Send + Sync + 'static {
    /// Group-level identifier the backend was constructed for. Surfaced in
    /// virtio-blk GET_ID responses.
    fn group_id(&self) -> Uuid;

    /// Block size enforced by the backend. Daemon parses requests with
    /// this alignment.
    fn block_size(&self) -> u64;

    /// Total capacity in bytes. Reported to the guest as virtio-blk
    /// configspace (`capacity` in 512-byte sectors).
    fn capacity_bytes(&self) -> u64;

    /// Apply one virtio-blk request and produce its response. Errors that
    /// are recoverable (alignment, bounds) become `VirtioBlkStatus::IoErr`;
    /// errors that are operational (transport down, no quorum) bubble out
    /// to the daemon which logs and replies IoErr with the specific cause.
    async fn dispatch(&self, request: BlockRequest) -> Result<BlockResponse, BlockBackendError>;
}

/// Configuration for the production HTTP-backed backend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaftBlockBackendConfig {
    /// `http://<agent>:<port>/v1/raft_block` base URL. The backend appends
    /// route suffixes (`/<group_id>/openraft/...`) to this.
    pub agent_base_url: String,
    /// The group's UUID (one Raft group per guest disk).
    pub group_id: Uuid,
    /// Backend-side block alignment. Must match the group's `block_size`.
    pub block_size: u64,
    /// Backend-side capacity. Must match the group's `capacity_bytes`.
    pub capacity_bytes: u64,
}

/// Production backend. Sends Read requests to the agent's `/read` route
/// (no Raft round-trip needed for follower-style reads from local replica)
/// and Write/Flush requests through the agent's `runtime_client_write` so
/// the leader replicates and quorum-commits before returning.
///
/// Reads bypass Raft because the local agent's replica is already a
/// committed copy after the prior write returns. Stale reads under partition
/// are theoretically possible (the local replica may lag if this daemon
/// runs co-located with a follower, not the leader). For B-II this matches
/// the spec's "no follower reads" non-goal: in production the daemon runs
/// on the leader's host and the local replica is always current.
#[derive(Debug, Clone)]
pub struct RaftBlockBackend {
    config: RaftBlockBackendConfig,
    client: reqwest::Client,
}

impl RaftBlockBackend {
    pub fn new(config: RaftBlockBackendConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }

    pub fn with_client(config: RaftBlockBackendConfig, client: reqwest::Client) -> Self {
        Self { config, client }
    }

    fn url(&self, suffix: &str) -> String {
        format!(
            "{}/{}",
            self.config.agent_base_url.trim_end_matches('/'),
            suffix.trim_start_matches('/')
        )
    }
}

#[async_trait::async_trait]
impl BlockBackend for RaftBlockBackend {
    fn group_id(&self) -> Uuid {
        self.config.group_id
    }

    fn block_size(&self) -> u64 {
        self.config.block_size
    }

    fn capacity_bytes(&self) -> u64 {
        self.config.capacity_bytes
    }

    async fn dispatch(&self, request: BlockRequest) -> Result<BlockResponse, BlockBackendError> {
        match request.kind {
            BlockRequestKind::Read { offset, len } => {
                let body = serde_json::json!({
                    "group_id": self.config.group_id,
                    "offset": offset,
                    "len": len,
                });
                let resp = self
                    .client
                    .post(self.url("read"))
                    .json(&body)
                    .send()
                    .await
                    .map_err(|e| BlockBackendError::Transport(e.to_string()))?;
                if !resp.status().is_success() {
                    return Ok(BlockResponse {
                        status: VirtioBlkStatus::IoErr,
                        data: vec![0; len as usize],
                    });
                }
                let body: serde_json::Value = resp
                    .json()
                    .await
                    .map_err(|e| BlockBackendError::MalformedResponse(e.to_string()))?;
                let bytes = body
                    .get("bytes")
                    .and_then(|v| v.as_array())
                    .ok_or_else(|| {
                        BlockBackendError::MalformedResponse("missing bytes array".into())
                    })?
                    .iter()
                    .map(|n| n.as_u64().unwrap_or(0) as u8)
                    .collect();
                Ok(BlockResponse {
                    status: VirtioBlkStatus::Ok,
                    data: bytes,
                })
            }
            BlockRequestKind::Write { offset, data } => {
                // Drive writes through the Raft runtime's client_write
                // which only returns once quorum-committed and applied.
                // The daemon dispatches via a synthetic `runtime_write`
                // route that wraps `state.runtime_client_write`.
                let body = serde_json::json!({
                    "group_id": self.config.group_id,
                    "command": {
                        "Write": {
                            "offset": offset,
                            "bytes": data,
                        }
                    },
                });
                let resp = self
                    .client
                    .post(self.url("runtime_write"))
                    .json(&body)
                    .send()
                    .await
                    .map_err(|e| BlockBackendError::Transport(e.to_string()))?;
                if !resp.status().is_success() {
                    let body = resp.text().await.unwrap_or_default();
                    return Err(BlockBackendError::Rejected(body));
                }
                Ok(BlockResponse {
                    status: VirtioBlkStatus::Ok,
                    data: vec![],
                })
            }
            BlockRequestKind::Flush => {
                // Raft's client_write is synchronous-on-commit, so by the
                // time any prior write returned, it's already durable on a
                // quorum of replicas. Flush has nothing to do.
                Ok(BlockResponse {
                    status: VirtioBlkStatus::Ok,
                    data: vec![],
                })
            }
            BlockRequestKind::GetId => Ok(BlockResponse {
                status: VirtioBlkStatus::Ok,
                data: format_serial_id(self.config.group_id),
            }),
        }
    }
}

/// One recorded `(offset, bytes)` pair from `InMemoryBlockBackend.write_log()`.
pub type RecordedWrite = (u64, Vec<u8>);

/// Test-only in-memory backend. Tracks all writes so tests can assert what
/// the daemon issued. Behaves like a perfectly-replicated zero-latency
/// Raft group: reads return whatever was written last, flushes are no-ops.
#[derive(Debug, Clone)]
pub struct InMemoryBlockBackend {
    group_id: Uuid,
    block_size: u64,
    capacity_bytes: u64,
    storage: Arc<Mutex<Vec<u8>>>,
    write_log: Arc<Mutex<Vec<RecordedWrite>>>,
}

impl InMemoryBlockBackend {
    pub fn new(group_id: Uuid, block_size: u64, capacity_bytes: u64) -> Self {
        Self {
            group_id,
            block_size,
            capacity_bytes,
            storage: Arc::new(Mutex::new(vec![0u8; capacity_bytes as usize])),
            write_log: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn write_log(&self) -> Vec<RecordedWrite> {
        self.write_log.lock().unwrap().clone()
    }
}

#[async_trait::async_trait]
impl BlockBackend for InMemoryBlockBackend {
    fn group_id(&self) -> Uuid {
        self.group_id
    }
    fn block_size(&self) -> u64 {
        self.block_size
    }
    fn capacity_bytes(&self) -> u64 {
        self.capacity_bytes
    }

    async fn dispatch(&self, request: BlockRequest) -> Result<BlockResponse, BlockBackendError> {
        match request.kind {
            BlockRequestKind::Read { offset, len } => {
                let storage = self.storage.lock().unwrap();
                let end = (offset + len as u64) as usize;
                if end > storage.len() {
                    return Ok(BlockResponse {
                        status: VirtioBlkStatus::IoErr,
                        data: vec![0; len as usize],
                    });
                }
                Ok(BlockResponse {
                    status: VirtioBlkStatus::Ok,
                    data: storage[offset as usize..end].to_vec(),
                })
            }
            BlockRequestKind::Write { offset, data } => {
                let mut storage = self.storage.lock().unwrap();
                let end = (offset as usize) + data.len();
                if end > storage.len() {
                    return Ok(BlockResponse {
                        status: VirtioBlkStatus::IoErr,
                        data: vec![],
                    });
                }
                storage[offset as usize..end].copy_from_slice(&data);
                self.write_log.lock().unwrap().push((offset, data));
                Ok(BlockResponse {
                    status: VirtioBlkStatus::Ok,
                    data: vec![],
                })
            }
            BlockRequestKind::Flush => Ok(BlockResponse {
                status: VirtioBlkStatus::Ok,
                data: vec![],
            }),
            BlockRequestKind::GetId => Ok(BlockResponse {
                status: VirtioBlkStatus::Ok,
                data: format_serial_id(self.group_id),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::request::parse_request;
    use crate::request::{
        VIRTIO_BLK_T_FLUSH, VIRTIO_BLK_T_GET_ID, VIRTIO_BLK_T_IN, VIRTIO_BLK_T_OUT,
    };

    #[tokio::test]
    async fn in_memory_backend_round_trips_write_then_read() {
        let backend = InMemoryBlockBackend::new(Uuid::new_v4(), 512, 8192);

        // Write 512 bytes at sector 2 (offset 1024)
        let write_req = parse_request(VIRTIO_BLK_T_OUT, 2, 512, 0, &[0xab; 512]).unwrap();
        let resp = backend.dispatch(write_req).await.unwrap();
        assert_eq!(resp.status, VirtioBlkStatus::Ok);

        // Read back at the same offset
        let read_req = parse_request(VIRTIO_BLK_T_IN, 2, 512, 512, &[]).unwrap();
        let resp = backend.dispatch(read_req).await.unwrap();
        assert_eq!(resp.status, VirtioBlkStatus::Ok);
        assert_eq!(resp.data.len(), 512);
        assert!(resp.data.iter().all(|&b| b == 0xab));

        // Write log records the operation
        let log = backend.write_log();
        assert_eq!(log.len(), 1);
        assert_eq!(log[0].0, 1024);
    }

    #[tokio::test]
    async fn in_memory_backend_flush_is_noop() {
        let backend = InMemoryBlockBackend::new(Uuid::new_v4(), 512, 4096);
        let flush_req = parse_request(VIRTIO_BLK_T_FLUSH, 0, 512, 0, &[]).unwrap();
        let resp = backend.dispatch(flush_req).await.unwrap();
        assert_eq!(resp.status, VirtioBlkStatus::Ok);
        assert!(resp.data.is_empty());
    }

    #[tokio::test]
    async fn in_memory_backend_get_id_returns_serial_with_uuid_prefix() {
        let group_id = Uuid::new_v4();
        let backend = InMemoryBlockBackend::new(group_id, 512, 4096);
        let req = parse_request(VIRTIO_BLK_T_GET_ID, 0, 512, 0, &[]).unwrap();
        let resp = backend.dispatch(req).await.unwrap();
        assert_eq!(resp.status, VirtioBlkStatus::Ok);
        assert_eq!(resp.data.len(), 20);
        assert_eq!(&resp.data[..16], group_id.as_bytes());
    }

    #[tokio::test]
    async fn in_memory_backend_returns_ioerr_for_out_of_bounds_read() {
        let backend = InMemoryBlockBackend::new(Uuid::new_v4(), 512, 1024);
        // Read at sector 4 (offset 2048) with 1024-byte device — out of bounds
        let req = parse_request(VIRTIO_BLK_T_IN, 4, 512, 512, &[]).unwrap();
        let resp = backend.dispatch(req).await.unwrap();
        assert_eq!(resp.status, VirtioBlkStatus::IoErr);
    }
}
