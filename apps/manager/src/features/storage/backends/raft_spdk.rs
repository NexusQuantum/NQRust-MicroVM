//! Raft-replicated SPDK control-plane scaffold.
//!
//! B-II must not claim a production data path before raftblk/Openraft is wired.
//! This backend validates static placement and exposes the future capability
//! shape while returning NotSupported for mutating lifecycle calls.

use nexus_storage::{
    BackendInstanceId, BackendKind, Capabilities, ControlPlaneBackend, CreateOpts, RaftSpdkLocator,
    RaftSpdkReplicaLocator, StorageError, VolumeHandle, VolumeSnapshotHandle,
    RAFT_SPDK_DEFAULT_BLOCK_SIZE, RAFT_SPDK_STATIC_REPLICA_COUNT,
};
use serde::{Deserialize, Serialize};
use std::path::Path;
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize)]
pub struct RaftSpdkConfig {
    #[serde(default = "default_block_size")]
    pub block_size: u64,
    #[serde(default)]
    pub prototype_provisioning_enabled: bool,
    pub replicas: Vec<RaftSpdkReplicaConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RaftSpdkReplicaConfig {
    pub node_id: u64,
    pub agent_base_url: String,
    pub spdk_backend_id: uuid::Uuid,
}

fn default_block_size() -> u64 {
    RAFT_SPDK_DEFAULT_BLOCK_SIZE
}

pub struct RaftSpdkControlPlaneBackend {
    pub id: BackendInstanceId,
    pub config: RaftSpdkConfig,
    http: reqwest::Client,
}

impl RaftSpdkControlPlaneBackend {
    pub fn new(id: BackendInstanceId, config: RaftSpdkConfig) -> Result<Self, StorageError> {
        validate_config(&config)?;
        Ok(Self {
            id,
            config,
            http: reqwest::Client::new(),
        })
    }

    fn raft_block_url(replica: &RaftSpdkReplicaConfig, path: &str) -> String {
        format!(
            "{}/v1/raft_block/{}",
            replica.agent_base_url.trim_end_matches('/'),
            path.trim_start_matches('/')
        )
    }

    async fn create_remote_group(
        &self,
        replica: &RaftSpdkReplicaConfig,
        group_id: Uuid,
        size_bytes: u64,
    ) -> Result<(), StorageError> {
        let req = CreateRaftBlockGroupReq {
            group_id,
            node_id: replica.node_id,
            capacity_bytes: size_bytes,
            block_size: self.config.block_size,
        };
        let response = self
            .http
            .post(Self::raft_block_url(replica, "create"))
            .json(&req)
            .send()
            .await
            .map_err(StorageError::backend)?;
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(StorageError::backend(std::io::Error::other(format!(
                "raft_spdk create group on node {} failed with {status}: {body}",
                replica.node_id
            ))));
        }
        Ok(())
    }

    async fn stop_remote_group(&self, replica: &RaftSpdkReplicaConfig, group_id: Uuid) {
        let _ = self
            .http
            .post(Self::raft_block_url(replica, "stop"))
            .json(&StopRaftBlockGroupReq { group_id })
            .send()
            .await;
    }
}

#[async_trait::async_trait]
impl ControlPlaneBackend for RaftSpdkControlPlaneBackend {
    fn kind(&self) -> BackendKind {
        BackendKind::RaftSpdk
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            supports_native_snapshots: true,
            supports_concurrent_attach: false,
            supports_live_migration: false,
            supports_clone_from_image: false,
        }
    }

    async fn provision(&self, opts: CreateOpts) -> Result<VolumeHandle, StorageError> {
        if !self.config.prototype_provisioning_enabled {
            return Err(StorageError::NotSupported(format!(
                "raft_spdk backend {} with {} replicas awaits production raftblk/Openraft group bootstrap; set prototype_provisioning_enabled only for B-II harness testing",
                self.id.0,
                self.config.replicas.len()
            )));
        }
        if opts.size_bytes == 0 || !opts.size_bytes.is_multiple_of(self.config.block_size) {
            return Err(StorageError::InvalidLocator(format!(
                "raft_spdk volume size must be a nonzero multiple of block_size {}",
                self.config.block_size
            )));
        }

        let group_id = Uuid::new_v4();
        let mut created: Vec<&RaftSpdkReplicaConfig> = Vec::new();
        for replica in &self.config.replicas {
            if let Err(err) = self
                .create_remote_group(replica, group_id, opts.size_bytes)
                .await
            {
                for created_replica in &created {
                    self.stop_remote_group(created_replica, group_id).await;
                }
                return Err(err);
            }
            created.push(replica);
        }

        let locator = RaftSpdkLocator::new(
            group_id,
            opts.size_bytes,
            self.config.block_size,
            self.config
                .replicas
                .iter()
                .map(|replica| RaftSpdkReplicaLocator {
                    node_id: replica.node_id,
                    agent_base_url: replica.agent_base_url.clone(),
                    spdk_lvol_locator: serde_json::json!({
                        "spdk_backend_id": replica.spdk_backend_id,
                        "prototype_replica": true
                    })
                    .to_string(),
                })
                .collect(),
            self.config.replicas.first().map(|replica| replica.node_id),
        )?;

        Ok(VolumeHandle {
            volume_id: Uuid::new_v4(),
            backend_id: self.id,
            backend_kind: BackendKind::RaftSpdk,
            locator: locator.to_locator_string()?,
            size_bytes: opts.size_bytes,
        })
    }

    async fn destroy(&self, _handle: VolumeHandle) -> Result<(), StorageError> {
        Err(StorageError::NotSupported(
            "raft_spdk destroy awaits raftblk/Openraft group teardown".into(),
        ))
    }

    async fn clone_from_image(
        &self,
        _source_image: &Path,
        _opts: CreateOpts,
    ) -> Result<VolumeHandle, StorageError> {
        Err(StorageError::NotSupported(
            "raft_spdk clone_from_image must write through Raft".into(),
        ))
    }

    async fn snapshot(
        &self,
        _volume: &VolumeHandle,
        _name: &str,
    ) -> Result<VolumeSnapshotHandle, StorageError> {
        Err(StorageError::NotSupported(
            "raft_spdk snapshot awaits consistent Raft snapshot export".into(),
        ))
    }

    async fn clone_from_snapshot(
        &self,
        _snap: &VolumeSnapshotHandle,
    ) -> Result<VolumeHandle, StorageError> {
        Err(StorageError::NotSupported(
            "raft_spdk clone_from_snapshot awaits Raft snapshot import".into(),
        ))
    }

    async fn delete_snapshot(&self, _snap: VolumeSnapshotHandle) -> Result<(), StorageError> {
        Err(StorageError::NotSupported(
            "raft_spdk delete_snapshot awaits Raft snapshot metadata".into(),
        ))
    }
}

#[derive(Debug, Serialize)]
struct CreateRaftBlockGroupReq {
    group_id: Uuid,
    node_id: u64,
    capacity_bytes: u64,
    block_size: u64,
}

#[derive(Debug, Serialize)]
struct StopRaftBlockGroupReq {
    group_id: Uuid,
}

pub fn validate_config(config: &RaftSpdkConfig) -> Result<(), StorageError> {
    if config.block_size == 0 {
        return Err(StorageError::InvalidLocator(
            "raft_spdk config.block_size must be nonzero".into(),
        ));
    }
    if config.replicas.len() != RAFT_SPDK_STATIC_REPLICA_COUNT {
        return Err(StorageError::InvalidLocator(format!(
            "raft_spdk requires exactly {RAFT_SPDK_STATIC_REPLICA_COUNT} static replicas"
        )));
    }
    let mut node_ids = std::collections::BTreeSet::new();
    for replica in &config.replicas {
        if replica.node_id == 0 {
            return Err(StorageError::InvalidLocator(
                "raft_spdk replica node_id must be nonzero".into(),
            ));
        }
        if !node_ids.insert(replica.node_id) {
            return Err(StorageError::InvalidLocator(format!(
                "raft_spdk duplicate replica node_id {}",
                replica.node_id
            )));
        }
        if replica.agent_base_url.trim().is_empty() {
            return Err(StorageError::InvalidLocator(
                "raft_spdk replica agent_base_url must not be empty".into(),
            ));
        }
        if replica.spdk_backend_id.is_nil() {
            return Err(StorageError::InvalidLocator(
                "raft_spdk replica spdk_backend_id must not be nil".into(),
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> RaftSpdkConfig {
        RaftSpdkConfig {
            block_size: 512,
            prototype_provisioning_enabled: false,
            replicas: vec![
                RaftSpdkReplicaConfig {
                    node_id: 1,
                    agent_base_url: "http://agent-1:19090".into(),
                    spdk_backend_id: uuid::Uuid::new_v4(),
                },
                RaftSpdkReplicaConfig {
                    node_id: 2,
                    agent_base_url: "http://agent-2:19090".into(),
                    spdk_backend_id: uuid::Uuid::new_v4(),
                },
                RaftSpdkReplicaConfig {
                    node_id: 3,
                    agent_base_url: "http://agent-3:19090".into(),
                    spdk_backend_id: uuid::Uuid::new_v4(),
                },
            ],
        }
    }

    #[test]
    fn validates_three_static_replicas() {
        validate_config(&cfg()).unwrap();
    }

    #[test]
    fn rejects_duplicate_replica_node_ids() {
        let mut cfg = cfg();
        cfg.replicas[2].node_id = 2;
        let err = validate_config(&cfg).unwrap_err();
        assert!(err.to_string().contains("duplicate"));
    }

    #[tokio::test]
    async fn provision_is_guarded_until_data_path_exists() {
        let backend =
            RaftSpdkControlPlaneBackend::new(BackendInstanceId(uuid::Uuid::new_v4()), cfg())
                .unwrap();
        let err = backend
            .provision(CreateOpts {
                name: "vol".into(),
                size_bytes: 4096,
                description: None,
            })
            .await
            .unwrap_err();
        assert!(matches!(err, StorageError::NotSupported(_)));
    }

    #[tokio::test]
    async fn prototype_provisioning_creates_static_agent_groups_and_locator() {
        async fn record(
            axum::extract::State(calls): axum::extract::State<
                std::sync::Arc<tokio::sync::Mutex<Vec<serde_json::Value>>>,
            >,
            axum::Json(body): axum::Json<serde_json::Value>,
        ) -> axum::Json<serde_json::Value> {
            calls.lock().await.push(body);
            axum::Json(serde_json::json!({}))
        }

        async fn spawn_agent() -> (
            String,
            std::sync::Arc<tokio::sync::Mutex<Vec<serde_json::Value>>>,
            tokio::task::JoinHandle<()>,
        ) {
            let calls = std::sync::Arc::new(tokio::sync::Mutex::new(Vec::new()));
            let app = axum::Router::new()
                .route("/v1/raft_block/create", axum::routing::post(record))
                .route("/v1/raft_block/stop", axum::routing::post(record))
                .with_state(calls.clone());
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let addr = listener.local_addr().unwrap();
            let handle = tokio::spawn(async move {
                axum::serve(listener, app).await.unwrap();
            });
            (format!("http://{addr}"), calls, handle)
        }

        let (url1, calls1, server1) = spawn_agent().await;
        let (url2, calls2, server2) = spawn_agent().await;
        let (url3, calls3, server3) = spawn_agent().await;
        let mut cfg = cfg();
        cfg.prototype_provisioning_enabled = true;
        cfg.replicas[0].agent_base_url = url1;
        cfg.replicas[1].agent_base_url = url2;
        cfg.replicas[2].agent_base_url = url3;
        let backend =
            RaftSpdkControlPlaneBackend::new(BackendInstanceId(uuid::Uuid::new_v4()), cfg).unwrap();

        let handle = backend
            .provision(CreateOpts {
                name: "vol".into(),
                size_bytes: 4096,
                description: None,
            })
            .await
            .unwrap();

        assert_eq!(handle.backend_kind, BackendKind::RaftSpdk);
        let locator = RaftSpdkLocator::from_locator_str(&handle.locator).unwrap();
        assert_eq!(locator.replicas.len(), RAFT_SPDK_STATIC_REPLICA_COUNT);
        assert_eq!(locator.leader_hint, Some(1));
        assert_eq!(calls1.lock().await[0]["node_id"], 1);
        assert_eq!(calls2.lock().await[0]["node_id"], 2);
        assert_eq!(calls3.lock().await[0]["node_id"], 3);

        server1.abort();
        server2.abort();
        server3.abort();
    }
}
