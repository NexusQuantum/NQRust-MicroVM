//! Agent-side raft_spdk scaffold.
//!
//! The real B-II data path must run through raftblk, not directly through an
//! SPDK vhost controller. This backend starts the local durable raft-block group
//! before returning the future raftblk socket path.

use crate::features::raft_block::RaftBlockState;
use nexus_storage::{
    raftblk_socket_path, AttachedPath, BackendKind, HostBackend, RaftSpdkLocator, StorageError,
    VolumeHandle, VolumeSnapshotHandle,
};
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct RaftSpdkHostBackend {
    socket_dir: PathBuf,
    local_node_id: u64,
    raft_block: Arc<RaftBlockState>,
}

impl RaftSpdkHostBackend {
    pub fn new(
        socket_dir: impl Into<PathBuf>,
        local_node_id: u64,
        raft_block: Arc<RaftBlockState>,
    ) -> Self {
        Self {
            socket_dir: socket_dir.into(),
            local_node_id,
            raft_block,
        }
    }

    fn socket_path_for_locator(&self, locator: &RaftSpdkLocator) -> PathBuf {
        raftblk_socket_path(&self.socket_dir, locator.group_id)
    }
}

#[async_trait::async_trait]
impl HostBackend for RaftSpdkHostBackend {
    fn kind(&self) -> BackendKind {
        BackendKind::RaftSpdk
    }

    async fn attach(&self, volume: &VolumeHandle) -> Result<AttachedPath, StorageError> {
        let locator = RaftSpdkLocator::from_locator_str(&volume.locator)?;
        if !locator
            .replicas
            .iter()
            .any(|replica| replica.node_id == self.local_node_id)
        {
            return Err(StorageError::InvalidLocator(format!(
                "raft_spdk local node {} is not a replica for group {}",
                self.local_node_id, locator.group_id
            )));
        }
        if locator
            .leader_hint
            .is_some_and(|leader| leader != self.local_node_id)
        {
            return Err(StorageError::NotSupported(format!(
                "raft_spdk leader-only attach refused on node {}; leader hint is {:?}",
                self.local_node_id, locator.leader_hint
            )));
        }
        self.raft_block
            .ensure_group(
                locator.group_id,
                self.local_node_id,
                locator.size_bytes,
                locator.block_size,
            )
            .await
            .map_err(|e| StorageError::InvalidLocator(e.to_string()))?;
        Ok(AttachedPath::VhostUserSock(
            self.socket_path_for_locator(&locator),
        ))
    }

    async fn detach(
        &self,
        volume: &VolumeHandle,
        _attached: AttachedPath,
    ) -> Result<(), StorageError> {
        let locator = RaftSpdkLocator::from_locator_str(&volume.locator)?;
        self.raft_block.stop_group(locator.group_id).await;
        Ok(())
    }

    async fn populate_streaming(
        &self,
        _attached: &AttachedPath,
        _source: &Path,
        _target_size_bytes: u64,
    ) -> Result<(), StorageError> {
        Err(StorageError::NotSupported(
            "raft_spdk populate_streaming must write through raftblk proposals".into(),
        ))
    }

    async fn resize2fs(&self, _attached: &AttachedPath) -> Result<(), StorageError> {
        Err(StorageError::NotSupported(
            "raft_spdk resize2fs awaits raftblk/NBD export support".into(),
        ))
    }

    async fn read_snapshot(
        &self,
        snap: &VolumeSnapshotHandle,
    ) -> Result<Box<dyn tokio::io::AsyncRead + Send + Unpin>, StorageError> {
        let locator = RaftSpdkLocator::from_locator_str(&snap.locator)?;
        let bytes = self
            .raft_block
            .snapshot_bytes(locator.group_id)
            .await
            .map_err(|e| StorageError::InvalidLocator(e.to_string()))?;
        Ok(Box::new(std::io::Cursor::new(bytes)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexus_storage::{BackendInstanceId, RaftSpdkReplicaLocator};
    use uuid::Uuid;

    fn locator() -> RaftSpdkLocator {
        RaftSpdkLocator::new(
            Uuid::parse_str("018f64ba-97aa-70d9-a7d2-6459256fd111").unwrap(),
            4096,
            512,
            vec![
                RaftSpdkReplicaLocator {
                    node_id: 1,
                    agent_base_url: "http://agent-1:19090".into(),
                    spdk_lvol_locator: "{}".into(),
                },
                RaftSpdkReplicaLocator {
                    node_id: 2,
                    agent_base_url: "http://agent-2:19090".into(),
                    spdk_lvol_locator: "{}".into(),
                },
                RaftSpdkReplicaLocator {
                    node_id: 3,
                    agent_base_url: "http://agent-3:19090".into(),
                    spdk_lvol_locator: "{}".into(),
                },
            ],
            Some(1),
        )
        .unwrap()
    }

    #[tokio::test]
    async fn attach_returns_raftblk_vhost_socket() {
        let state = Arc::new(RaftBlockState::new(tempfile::tempdir().unwrap().path()));
        let backend = RaftSpdkHostBackend::new("/run/nqrust/raftblk", 1, state.clone());
        let group_id = locator().group_id;
        let volume = VolumeHandle {
            volume_id: Uuid::new_v4(),
            backend_id: BackendInstanceId(Uuid::new_v4()),
            backend_kind: BackendKind::RaftSpdk,
            locator: locator().to_locator_string().unwrap(),
            size_bytes: 4096,
        };

        let attached = backend.attach(&volume).await.unwrap();
        let AttachedPath::VhostUserSock(path) = attached else {
            panic!("expected raftblk vhost-user socket");
        };
        assert_eq!(path, raftblk_socket_path("/run/nqrust/raftblk", group_id));
        assert_eq!(state.status(group_id).await.state, "started");
    }

    #[tokio::test]
    async fn attach_rejects_non_member_node() {
        let state = Arc::new(RaftBlockState::new(tempfile::tempdir().unwrap().path()));
        let backend = RaftSpdkHostBackend::new("/run/nqrust/raftblk", 9, state);
        let volume = VolumeHandle {
            volume_id: Uuid::new_v4(),
            backend_id: BackendInstanceId(Uuid::new_v4()),
            backend_kind: BackendKind::RaftSpdk,
            locator: locator().to_locator_string().unwrap(),
            size_bytes: 4096,
        };

        let err = backend.attach(&volume).await.unwrap_err();
        assert!(err.to_string().contains("not a replica"), "got: {err}");
    }

    #[tokio::test]
    async fn attach_rejects_follower_when_leader_hint_points_elsewhere() {
        let state = Arc::new(RaftBlockState::new(tempfile::tempdir().unwrap().path()));
        let backend = RaftSpdkHostBackend::new("/run/nqrust/raftblk", 2, state);
        let volume = VolumeHandle {
            volume_id: Uuid::new_v4(),
            backend_id: BackendInstanceId(Uuid::new_v4()),
            backend_kind: BackendKind::RaftSpdk,
            locator: locator().to_locator_string().unwrap(),
            size_bytes: 4096,
        };

        let err = backend.attach(&volume).await.unwrap_err();
        assert!(err.to_string().contains("leader-only"), "got: {err}");
    }

    #[tokio::test]
    async fn detach_stops_group_without_destroying_state() {
        let state = Arc::new(RaftBlockState::new(tempfile::tempdir().unwrap().path()));
        let backend = RaftSpdkHostBackend::new("/run/nqrust/raftblk", 1, state.clone());
        let group_id = locator().group_id;
        let volume = VolumeHandle {
            volume_id: Uuid::new_v4(),
            backend_id: BackendInstanceId(Uuid::new_v4()),
            backend_kind: BackendKind::RaftSpdk,
            locator: locator().to_locator_string().unwrap(),
            size_bytes: 4096,
        };

        let attached = backend.attach(&volume).await.unwrap();
        assert_eq!(state.status(group_id).await.state, "started");
        backend.detach(&volume, attached).await.unwrap();
        assert_eq!(state.status(group_id).await.state, "not_started");
        backend.attach(&volume).await.unwrap();
        assert_eq!(state.status(group_id).await.state, "started");
    }

    #[tokio::test]
    async fn populate_is_guarded_until_raftblk_exists() {
        let state = Arc::new(RaftBlockState::new(tempfile::tempdir().unwrap().path()));
        let backend = RaftSpdkHostBackend::new("/run/nqrust/raftblk", 1, state);
        let err = backend
            .populate_streaming(
                &AttachedPath::VhostUserSock("/tmp/raft.sock".into()),
                Path::new("/dev/null"),
                4096,
            )
            .await
            .unwrap_err();
        assert!(matches!(err, StorageError::NotSupported(_)));
    }

    #[tokio::test]
    async fn read_snapshot_streams_consistent_raft_bytes() {
        use axum::response::IntoResponse;
        use tokio::io::AsyncReadExt;

        let state = Arc::new(RaftBlockState::new(tempfile::tempdir().unwrap().path()));
        let backend = RaftSpdkHostBackend::new("/run/nqrust/raftblk", 1, state.clone());
        let group_id = locator().group_id;
        let volume = VolumeHandle {
            volume_id: Uuid::new_v4(),
            backend_id: BackendInstanceId(Uuid::new_v4()),
            backend_kind: BackendKind::RaftSpdk,
            locator: locator().to_locator_string().unwrap(),
            size_bytes: 4096,
        };
        backend.attach(&volume).await.unwrap();
        let response = crate::features::raft_block::append(
            axum::extract::State(state),
            axum::Json(crate::features::raft_block::AppendReq {
                group_id,
                term: 1,
                leader_id: None,
                command: nexus_raft_block::BlockCommand::Write {
                    offset: 0,
                    bytes: vec![7; 512],
                },
            }),
        )
        .await
        .into_response();
        assert!(response.status().is_success());

        let snap = VolumeSnapshotHandle {
            snapshot_id: Uuid::new_v4(),
            source_volume_id: volume.volume_id,
            backend_id: volume.backend_id,
            backend_kind: BackendKind::RaftSpdk,
            locator: locator().to_locator_string().unwrap(),
        };
        let mut reader = backend.read_snapshot(&snap).await.unwrap();
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await.unwrap();
        assert_eq!(&bytes[0..512], &[7; 512]);
        assert_eq!(bytes.len(), 4096);
    }
}
