//! Agent-side raft_spdk scaffold.
//!
//! The real B-II data path must run through raftblk, not directly through an
//! SPDK vhost controller. This backend exposes the future attach shape while
//! guarding all byte-mutating operations until the Openraft/raftblk service is
//! implemented.

use nexus_storage::{
    raftblk_socket_path, AttachedPath, BackendKind, HostBackend, RaftSpdkLocator, StorageError,
    VolumeHandle, VolumeSnapshotHandle,
};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct RaftSpdkHostBackend {
    socket_dir: PathBuf,
}

impl RaftSpdkHostBackend {
    pub fn new(socket_dir: impl Into<PathBuf>) -> Self {
        Self {
            socket_dir: socket_dir.into(),
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
        Ok(AttachedPath::VhostUserSock(
            self.socket_path_for_locator(&locator),
        ))
    }

    async fn detach(
        &self,
        _volume: &VolumeHandle,
        _attached: AttachedPath,
    ) -> Result<(), StorageError> {
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
        _snap: &VolumeSnapshotHandle,
    ) -> Result<Box<dyn tokio::io::AsyncRead + Send + Unpin>, StorageError> {
        Err(StorageError::NotSupported(
            "raft_spdk read_snapshot awaits consistent Raft snapshot export".into(),
        ))
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
        let backend = RaftSpdkHostBackend::new("/run/nqrust/raftblk");
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
    }

    #[tokio::test]
    async fn populate_is_guarded_until_raftblk_exists() {
        let backend = RaftSpdkHostBackend::new("/run/nqrust/raftblk");
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
}
