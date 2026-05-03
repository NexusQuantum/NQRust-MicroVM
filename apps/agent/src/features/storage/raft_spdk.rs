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

/// Tracks a spawned raftblk-vhost daemon process per group so detach can
/// stop it cleanly.
#[derive(Debug)]
struct DaemonHandle {
    child: tokio::process::Child,
}

#[derive(Debug, Clone)]
pub struct RaftSpdkHostBackend {
    socket_dir: PathBuf,
    local_node_id: u64,
    raft_block: Arc<RaftBlockState>,
    active_groups: Arc<Mutex<HashMap<PathBuf, RaftSpdkLocator>>>,
    /// raftblk-vhost daemon processes spawned for each active group.
    /// Stored as `tokio::process::Child` so detach can `.kill().await`
    /// cleanly. Keyed by group_id (Uuid stringified) so reattach finds
    /// any existing process.
    daemons: Arc<Mutex<HashMap<uuid::Uuid, DaemonHandle>>>,
    /// Path to the raftblk-vhost binary. Defaults to "raftblk-vhost"
    /// (in PATH); operators can override via `AGENT_RAFTBLK_VHOST_BIN`
    /// at agent startup.
    daemon_bin: PathBuf,
    /// Local agent base URL the daemon will dial (e.g.
    /// "http://127.0.0.1:9090/v1/raft_block"). Operators set
    /// `AGENT_RAFTBLK_AGENT_URL` at agent startup.
    daemon_agent_url: String,
    /// When false, attach() does NOT spawn the raftblk-vhost daemon —
    /// it just returns the expected socket path. Used by unit tests
    /// (which don't have the daemon binary available) and by operator
    /// setups that manage the daemon out-of-band via systemd. Default
    /// true; override at agent startup with
    /// `AGENT_RAFTBLK_DISABLE_AUTOSPAWN=1`.
    autospawn_enabled: bool,
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
            daemons: Arc::new(Mutex::new(HashMap::new())),
            daemon_bin: std::env::var("AGENT_RAFTBLK_VHOST_BIN")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("raftblk-vhost")),
            daemon_agent_url: std::env::var("AGENT_RAFTBLK_AGENT_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:9090/v1/raft_block".to_string()),
            autospawn_enabled: std::env::var("AGENT_RAFTBLK_DISABLE_AUTOSPAWN").is_err(),
        }
    }

    /// Test-only constructor that disables the daemon auto-spawn so
    /// `attach()` returns the expected socket path without trying to
    /// exec the raftblk-vhost binary.
    #[cfg(test)]
    pub fn new_no_autospawn(
        socket_dir: impl Into<PathBuf>,
        local_node_id: u64,
        raft_block: Arc<RaftBlockState>,
    ) -> Self {
        let mut backend = Self::new(socket_dir, local_node_id, raft_block);
        backend.autospawn_enabled = false;
        backend
    }

    fn socket_path_for_locator(&self, locator: &RaftSpdkLocator) -> PathBuf {
        raftblk_socket_path(&self.socket_dir, locator.group_id)
    }

    /// Start a raftblk-vhost daemon for `locator` on `socket_path` if
    /// one isn't already running for the group. Waits up to 5s for the
    /// socket to bind so the caller can return AttachedPath::VhostUserSock
    /// confidently. If the daemon binary is missing, returns an error
    /// rather than silently leaving an empty socket path.
    async fn ensure_daemon(
        &self,
        locator: &RaftSpdkLocator,
        socket_path: &Path,
    ) -> Result<(), StorageError> {
        {
            let daemons = self.daemons.lock().await;
            if daemons.contains_key(&locator.group_id) {
                return Ok(());
            }
        }
        if let Some(parent) = socket_path.parent() {
            std::fs::create_dir_all(parent).map_err(StorageError::backend)?;
        }
        // If a stale socket file is left behind from a previous crash,
        // remove it so the new daemon's bind succeeds.
        let _ = std::fs::remove_file(socket_path);

        let child = tokio::process::Command::new(&self.daemon_bin)
            .arg("--socket")
            .arg(socket_path)
            .arg("--agent-base-url")
            .arg(&self.daemon_agent_url)
            .arg("--group-id")
            .arg(locator.group_id.to_string())
            .arg("--block-size")
            .arg(locator.block_size.to_string())
            .arg("--capacity-bytes")
            .arg(locator.size_bytes.to_string())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|e| {
                StorageError::backend(std::io::Error::other(format!(
                    "spawn raftblk-vhost ({:?}): {e}",
                    self.daemon_bin
                )))
            })?;

        // Wait up to 5s for the daemon to bind the socket.
        for _ in 0..50 {
            if socket_path.exists() {
                self.daemons
                    .lock()
                    .await
                    .insert(locator.group_id, DaemonHandle { child });
                return Ok(());
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
        // Timed out — kill the child to avoid orphan, return error.
        let mut killed_child = child;
        let _ = killed_child.kill().await;
        Err(StorageError::backend(std::io::Error::other(format!(
            "raftblk-vhost daemon for group {} did not bind {} within 5s",
            locator.group_id,
            socket_path.display()
        ))))
    }

    async fn stop_daemon(&self, group_id: uuid::Uuid) {
        if let Some(mut handle) = self.daemons.lock().await.remove(&group_id) {
            let _ = handle.child.kill().await;
        }
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
        // Any replica node may host a vhost-user daemon for a local
        // Firecracker VM. Writes from the daemon are routed through Raft
        // to the leader regardless of which node serves the socket, so
        // attach is no longer leader-only — the daemon must run on the
        // same host as the consuming VM.
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
        // Spawn the raftblk-vhost daemon if it isn't already running for
        // this group. Returns once the socket is bound so Firecracker can
        // immediately use the path. Skipped when autospawn_enabled is
        // false (tests, or operator setups that manage the daemon
        // out-of-band via systemd).
        if self.autospawn_enabled {
            self.ensure_daemon(&locator, &socket_path).await?;
        }
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
        self.stop_daemon(locator.group_id).await;
        self.raft_block
            .stop_group(locator.group_id)
            .await
            .map_err(|e| StorageError::InvalidLocator(e.to_string()))?;
        self.active_groups.lock().await.remove(_attached.path());
        let _ = std::fs::remove_file(_attached.path());
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
        // Populate writes the rootfs into Raft via append_command. Calling
        // it once per block_size byte is correct but pathologically slow
        // for the prototype FileReplicaStore — every call rewrites the
        // entire log JSON to disk and fsyncs, making populate O(N²) in
        // entry count. A 64 MiB rootfs at 4 KiB blocks = 16 384 writes,
        // each rewriting an ever-growing JSON file: empirically this
        // didn't finish in 4 minutes.
        //
        // Coalescing into 1 MiB chunks (256 entries for 64 MiB) keeps the
        // virtio_blk wire `block_size` unchanged (the daemon still reports
        // 4 KiB to the guest) while collapsing populate from O(N²) to
        // O(N²/256²). The chunk is a multiple of block_size so the
        // BlockCommand::Write is still aligned.
        const POPULATE_TARGET_CHUNK_BYTES: usize = 1024 * 1024;
        let blocks_per_chunk = (POPULATE_TARGET_CHUNK_BYTES / block_size).max(1);
        let chunk_size = blocks_per_chunk * block_size;
        let mut offset = 0_u64;
        let mut remaining = target_size_bytes;
        while remaining > 0 {
            let chunk_len = chunk_size.min(remaining as usize);
            let mut block = vec![0u8; chunk_len];
            let mut filled = 0;
            while filled < chunk_len {
                let n = file.read(&mut block[filled..chunk_len])?;
                if n == 0 {
                    break;
                }
                filled += n;
            }
            // Production raft_spdk replicates populate writes through
            // openraft so committed bytes survive a leader-loss before the
            // guest writes anything. If no runtime is registered for this
            // group (prototype tests, or the legacy single-replica path),
            // fall back to the direct in-memory append so the existing
            // unit tests keep working.
            let command = BlockCommand::Write {
                offset,
                bytes: block,
            };
            let runtime_present = self
                .raft_block
                .runtime_for(locator.group_id)
                .await
                .is_some();
            if runtime_present {
                self.raft_block
                    .runtime_client_write(locator.group_id, command)
                    .await
                    .map_err(|e| StorageError::InvalidLocator(e.to_string()))?;
            } else {
                self.raft_block
                    .append_command(locator.group_id, 1, Some(self.local_node_id), command)
                    .await
                    .map_err(|e| StorageError::InvalidLocator(e.to_string()))?;
            }
            offset += chunk_len as u64;
            remaining = remaining.saturating_sub(chunk_len as u64);
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
        let backend =
            RaftSpdkHostBackend::new_no_autospawn("/run/nqrust/raftblk", 1, state.clone());
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
        let backend = RaftSpdkHostBackend::new_no_autospawn("/run/nqrust/raftblk", 9, state);
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
    async fn attach_succeeds_on_follower_replica() {
        // Any replica node may serve the vhost-user socket — writes route
        // through Raft to the leader regardless. Confirms attach no longer
        // rejects on a non-leader replica.
        let state = Arc::new(RaftBlockState::new(tempfile::tempdir().unwrap().path()));
        let backend = RaftSpdkHostBackend::new_no_autospawn("/run/nqrust/raftblk", 2, state);
        let volume = VolumeHandle {
            volume_id: Uuid::new_v4(),
            backend_id: BackendInstanceId(Uuid::new_v4()),
            backend_kind: BackendKind::RaftSpdk,
            locator: locator().to_locator_string().unwrap(),
            size_bytes: 4096,
        };

        let attached = backend.attach(&volume).await.expect("attach on follower");
        assert!(matches!(attached, AttachedPath::VhostUserSock(_)));
    }

    #[tokio::test]
    async fn detach_stops_group_without_destroying_state() {
        let state = Arc::new(RaftBlockState::new(tempfile::tempdir().unwrap().path()));
        let backend =
            RaftSpdkHostBackend::new_no_autospawn("/run/nqrust/raftblk", 1, state.clone());
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
        let backend = RaftSpdkHostBackend::new_no_autospawn("/run/nqrust/raftblk", 1, state);
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
        let backend = RaftSpdkHostBackend::new_no_autospawn("/run/nqrust/raftblk", 1, state);
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
        let backend =
            RaftSpdkHostBackend::new_no_autospawn("/run/nqrust/raftblk", 1, state.clone());
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
