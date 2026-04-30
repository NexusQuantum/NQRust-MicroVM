//! Agent-side raft_spdk scaffold.
//!
//! The real B-II data path must run through raftblk, not directly through an
//! SPDK vhost controller. This backend starts the local durable raft-block group
//! before returning the future raftblk socket path.

use crate::features::raft_block::RaftBlockState;
use nexus_raft_block::BlockCommand;
use nexus_storage::{
    raftblk_socket_path, AttachedPath, BackendKind, HostBackend, RaftSpdkLocator, StorageError,
    VolumeHandle, VolumeSnapshotHandle,
};
use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Clone)]
pub struct RaftSpdkHostBackend {
    socket_dir: PathBuf,
    local_node_id: u64,
    raft_block: Arc<RaftBlockState>,
    active_groups: Arc<Mutex<HashMap<PathBuf, RaftSpdkLocator>>>,
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
            active_groups: Arc::new(Mutex::new(HashMap::new())),
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
        let socket_path = self.socket_path_for_locator(&locator);
        self.active_groups
            .lock()
            .await
            .insert(socket_path.clone(), locator);
        Ok(AttachedPath::VhostUserSock(socket_path))
    }

    async fn detach(
        &self,
        volume: &VolumeHandle,
        _attached: AttachedPath,
    ) -> Result<(), StorageError> {
        let locator = RaftSpdkLocator::from_locator_str(&volume.locator)?;
        self.raft_block.stop_group(locator.group_id).await;
        self.active_groups.lock().await.remove(_attached.path());
        Ok(())
    }

    async fn populate_streaming(
        &self,
        attached: &AttachedPath,
        source: &Path,
        target_size_bytes: u64,
    ) -> Result<(), StorageError> {
        let locator = self
            .active_groups
            .lock()
            .await
            .get(attached.path())
            .cloned()
            .ok_or_else(|| {
                StorageError::InvalidLocator(format!(
                    "raft_spdk attached path {} is not active",
                    attached.path().display()
                ))
            })?;
        if target_size_bytes > locator.size_bytes {
            return Err(StorageError::InvalidLocator(format!(
                "target size {} exceeds raft_spdk volume size {}",
                target_size_bytes, locator.size_bytes
            )));
        }
        let mut file = std::fs::File::open(source)?;
        let block_size = locator.block_size as usize;
        let mut offset = 0_u64;
        let mut remaining = target_size_bytes;
        while remaining > 0 {
            let chunk_len = block_size.min(remaining as usize);
            let mut block = vec![0; block_size];
            let mut filled = 0;
            while filled < chunk_len {
                let n = file.read(&mut block[filled..chunk_len])?;
                if n == 0 {
                    break;
                }
                filled += n;
            }
            self.raft_block
                .append_command(
                    locator.group_id,
                    1,
                    Some(self.local_node_id),
                    BlockCommand::Write {
                        offset,
                        bytes: block,
                    },
                )
                .await
                .map_err(|e| StorageError::InvalidLocator(e.to_string()))?;
            offset += block_size as u64;
            remaining = remaining.saturating_sub(block_size as u64);
        }
        Ok(())
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
        assert!(matches!(err, StorageError::InvalidLocator(_)));
    }

    #[tokio::test]
    async fn populate_streaming_writes_through_raft_block() {
        use axum::response::IntoResponse;
        use tokio::io::AsyncReadExt;

        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source.img");
        std::fs::write(&source, vec![9; 700]).unwrap();
        let state = Arc::new(RaftBlockState::new(dir.path()));
        let backend = RaftSpdkHostBackend::new("/run/nqrust/raftblk", 1, state);
        let volume = VolumeHandle {
            volume_id: Uuid::new_v4(),
            backend_id: BackendInstanceId(Uuid::new_v4()),
            backend_kind: BackendKind::RaftSpdk,
            locator: locator().to_locator_string().unwrap(),
            size_bytes: 4096,
        };
        let attached = backend.attach(&volume).await.unwrap();
        backend
            .populate_streaming(&attached, &source, 1024)
            .await
            .unwrap();

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
        assert_eq!(&bytes[0..700], &[9; 700]);
        assert_eq!(&bytes[700..1024], &[0; 324]);

        let response = crate::features::raft_block::status(
            axum::extract::State(backend.raft_block.clone()),
            axum::extract::Path(locator().group_id),
        )
        .await
        .into_response();
        assert!(response.status().is_success());
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
