//! Raft-replicated SPDK control-plane scaffold.
//!
//! B-II must not claim a production data path before raftblk/Openraft is wired.
//! This backend validates static placement and exposes the future capability
//! shape while returning NotSupported for mutating lifecycle calls.

use nexus_storage::{
    BackendInstanceId, BackendKind, Capabilities, ControlPlaneBackend, CreateOpts, StorageError,
    VolumeHandle, VolumeSnapshotHandle, RAFT_SPDK_DEFAULT_BLOCK_SIZE,
    RAFT_SPDK_STATIC_REPLICA_COUNT,
};
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct RaftSpdkConfig {
    #[serde(default = "default_block_size")]
    pub block_size: u64,
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
}

impl RaftSpdkControlPlaneBackend {
    pub fn new(id: BackendInstanceId, config: RaftSpdkConfig) -> Result<Self, StorageError> {
        validate_config(&config)?;
        Ok(Self { id, config })
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

    async fn provision(&self, _opts: CreateOpts) -> Result<VolumeHandle, StorageError> {
        Err(StorageError::NotSupported(format!(
            "raft_spdk backend {} with {} replicas awaits raftblk/Openraft group bootstrap",
            self.id.0,
            self.config.replicas.len()
        )))
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
}
