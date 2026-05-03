//! SPDK-lvol-backed `ReplicaStoreImpl` for the Raft block prototype.
//!
//! Closes B-II Exit Criteria item 4 ("Move committed block bytes from
//! the JSON prototype store to SPDK lvol/NBD-backed replicas") on the
//! code side. Validation requires real SPDK on the host.
//!
//! ## Why a separate impl
//!
//! The prototype `FileReplicaStore::new(path)` writes JSON to a single
//! file on the agent's filesystem. That works for unit tests and for
//! single-host smoke runs but isn't the real production data path:
//!  - the bytes live on whatever disk the agent's process owns,
//!  - there's no separation of metadata (term, log, applied index) from
//!    bulk data (the block bytes),
//!  - there's no SPDK acceleration / vhost-user-blk path.
//!
//! `SpdkLvolReplicaStore` keeps the same load/save contract as
//! `FileReplicaStore` but writes compact metadata plus committed block
//! bytes through an SPDK NBD bdev. It does not rewrite the whole
//! capacity-sized byte vector on every Raft apply.
//!
//! ## On-disk layout
//!
//! Within the lvol:
//!
//! ```text
//! offset 0                    1 MiB                    1 MiB + capacity_bytes
//! ┌────────────────────────┬─────────────────────────────────────────┐
//! │ replica metadata       │ block data region                       │
//! │ (length-prefixed JSON) │ committed guest bytes                   │
//! └────────────────────────┴─────────────────────────────────────────┘
//! ```
//!
//! The metadata region is fixed at 1 MiB. Log history is compacted on
//! save by treating all applied entries as included in the stored block
//! image; on load the state resumes at `compacted_through + 1`.
//!
//! ## What this file ships
//!
//! - The struct + constructor (operator builds it from a configured NBD
//!   device path).
//! - The `ReplicaStoreImpl` trait impl with `load`/`save` that
//!   length-prefix compact metadata and writes changed block ranges
//!   through the NBD block device.
//! - Unit tests that exercise the load/save round-trip against a
//!   tempfile (NBD devices are file-shaped from the perspective of the
//!   read/write syscalls, so tempfile is a sound substitute for the
//!   on-disk format test).
//!
//! ## What needs operator validation
//!
//! - The NBD device must already be attached to the lvol via SPDK
//!   `nbd_start_disk` (the existing B-I bootstrap script handles this).
//! - The agent's `RaftBlockState::create_group` consumes a runtime
//!   config flag to pick `FileReplicaStore::new(path)` vs
//!   `FileReplicaStore::external(Arc::new(SpdkLvolReplicaStore::new(...)))`.
//!   That flag is wired in this commit; the operator selects per-group.

use nexus_raft_block::{
    BlockOp, LogIndex, PersistentReplicaState, RaftBlockError, ReplicaStoreImpl,
};
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::sync::Mutex;

/// Bytes reserved at the start of the lvol for compact metadata.
pub const METADATA_REGION_BYTES: u64 = 1024 * 1024;

/// Length-prefix size for the metadata payload. The prefix is 8 little-
/// endian bytes representing the JSON byte count.
const LENGTH_PREFIX_BYTES: usize = 8;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SpdkReplicaMeta {
    version: u32,
    node_id: u64,
    capacity_bytes: u64,
    block_size: u64,
    highest_term_seen: u64,
    applied_indexes: Vec<LogIndex>,
    compacted_through: LogIndex,
}

impl SpdkReplicaMeta {
    fn from_state(state: &PersistentReplicaState) -> Self {
        let compacted_through = state
            .applied_indexes
            .iter()
            .copied()
            .max()
            .unwrap_or(state.compacted_through);
        Self {
            version: 1,
            node_id: state.node_id,
            capacity_bytes: state.capacity_bytes,
            block_size: state.block_size,
            highest_term_seen: state.highest_term_seen,
            applied_indexes: state.applied_indexes.clone(),
            compacted_through,
        }
    }
}

/// SPDK-lvol-backed replica state storage.
///
/// The store opens the configured NBD device on each load/save; this
/// avoids holding a long-lived file handle across the Raft state
/// machine's lifetime, which simplifies failure recovery (a partial
/// write fails the save immediately rather than leaving a dangling fd).
#[derive(Debug)]
pub struct SpdkLvolReplicaStore {
    nbd_path: PathBuf,
    /// Serializes concurrent saves on the same device. The Raft pipeline
    /// is single-threaded per-group so contention is rare; this is a
    /// safety net for the rare case of operator-triggered manual saves.
    write_lock: Mutex<()>,
}

impl SpdkLvolReplicaStore {
    /// Construct a store backed by the NBD device at `nbd_path`. The
    /// device must already be bound to an SPDK lvol via
    /// `nbd_start_disk`; this constructor does NOT perform the SPDK RPC
    /// call (that is the agent's responsibility, set up at
    /// `RaftSpdkHostBackend::attach`).
    pub fn new(nbd_path: impl Into<PathBuf>) -> Self {
        Self {
            nbd_path: nbd_path.into(),
            write_lock: Mutex::new(()),
        }
    }
}

impl ReplicaStoreImpl for SpdkLvolReplicaStore {
    fn load(&self) -> Result<Option<PersistentReplicaState>, RaftBlockError> {
        let mut file = match OpenOptions::new().read(true).open(&self.nbd_path) {
            Ok(f) => f,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(err) => {
                return Err(RaftBlockError::Store(format!(
                    "open {:?}: {err}",
                    self.nbd_path
                )))
            }
        };
        file.seek(SeekFrom::Start(0))
            .map_err(|e| RaftBlockError::Store(format!("seek {:?}: {e}", self.nbd_path)))?;
        let mut prefix = [0u8; LENGTH_PREFIX_BYTES];
        match file.read_exact(&mut prefix) {
            Ok(()) => {}
            Err(err) if err.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
            Err(err) => {
                return Err(RaftBlockError::Store(format!(
                    "read prefix {:?}: {err}",
                    self.nbd_path
                )))
            }
        }
        let len = u64::from_le_bytes(prefix);
        if len == 0 {
            return Ok(None);
        }
        if len > METADATA_REGION_BYTES - LENGTH_PREFIX_BYTES as u64 {
            return Err(RaftBlockError::Store(format!(
                "metadata length {len} exceeds reserved region {METADATA_REGION_BYTES}"
            )));
        }
        let mut buf = vec![0u8; len as usize];
        file.read_exact(&mut buf)
            .map_err(|e| RaftBlockError::Store(format!("read body {:?}: {e}", self.nbd_path)))?;
        let meta: SpdkReplicaMeta = serde_json::from_slice(&buf)
            .map_err(|e| RaftBlockError::Store(format!("decode {:?}: {e}", self.nbd_path)))?;
        if meta.version != 1 {
            return Err(RaftBlockError::Store(format!(
                "unsupported SPDK replica store version {}",
                meta.version
            )));
        }
        let mut bytes = vec![0u8; meta.capacity_bytes as usize];
        file.seek(SeekFrom::Start(METADATA_REGION_BYTES))
            .map_err(|e| RaftBlockError::Store(format!("seek {:?}: {e}", self.nbd_path)))?;
        file.read_exact(&mut bytes)
            .map_err(|e| RaftBlockError::Store(format!("read blocks {:?}: {e}", self.nbd_path)))?;
        Ok(Some(PersistentReplicaState {
            node_id: meta.node_id,
            capacity_bytes: meta.capacity_bytes,
            block_size: meta.block_size,
            highest_term_seen: meta.highest_term_seen,
            applied_indexes: meta.applied_indexes,
            bytes,
            log: Vec::new(),
            compacted_through: meta.compacted_through,
        }))
    }

    fn save(&self, state: &PersistentReplicaState) -> Result<(), RaftBlockError> {
        let _guard = self
            .write_lock
            .lock()
            .map_err(|_| RaftBlockError::Store("write_lock poisoned".into()))?;
        let meta = SpdkReplicaMeta::from_state(state);
        let encoded = serde_json::to_vec(&meta)
            .map_err(|e| RaftBlockError::Store(format!("encode {:?}: {e}", self.nbd_path)))?;
        let total_with_prefix = encoded.len() as u64 + LENGTH_PREFIX_BYTES as u64;
        if total_with_prefix > METADATA_REGION_BYTES {
            return Err(RaftBlockError::Store(format!(
                "encoded metadata ({} bytes) exceeds metadata region ({} bytes)",
                encoded.len(),
                METADATA_REGION_BYTES
            )));
        }
        let mut file = OpenOptions::new()
            .write(true)
            .read(true)
            .open(&self.nbd_path)
            .map_err(|e| RaftBlockError::Store(format!("open {:?}: {e}", self.nbd_path)))?;
        ensure_device_len(&file, METADATA_REGION_BYTES + state.capacity_bytes)?;
        let previous_meta = read_meta_from_open_file(&mut file, &self.nbd_path)?;
        if let Some(previous) = previous_meta {
            let old_applied: std::collections::BTreeSet<LogIndex> =
                previous.applied_indexes.iter().copied().collect();
            write_new_blocks(&mut file, &self.nbd_path, state, &old_applied)?;
        } else {
            write_full_blocks(&mut file, &self.nbd_path, &state.bytes)?;
        }
        write_meta_to_open_file(&mut file, &self.nbd_path, &encoded)?;
        file.sync_all()
            .map_err(|e| RaftBlockError::Store(format!("sync {:?}: {e}", self.nbd_path)))?;
        Ok(())
    }
}

fn ensure_device_len(file: &std::fs::File, required_len: u64) -> Result<(), RaftBlockError> {
    let len = file
        .metadata()
        .map_err(|e| RaftBlockError::Store(format!("stat NBD device: {e}")))?
        .len();
    if len < required_len {
        return Err(RaftBlockError::Store(format!(
            "NBD device length {len} is smaller than required raft_spdk layout {required_len}"
        )));
    }
    Ok(())
}

fn read_meta_from_open_file(
    file: &mut std::fs::File,
    path: &PathBuf,
) -> Result<Option<SpdkReplicaMeta>, RaftBlockError> {
    file.seek(SeekFrom::Start(0))
        .map_err(|e| RaftBlockError::Store(format!("seek {path:?}: {e}")))?;
    let mut prefix = [0u8; LENGTH_PREFIX_BYTES];
    match file.read_exact(&mut prefix) {
        Ok(()) => {}
        Err(err) if err.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(err) => {
            return Err(RaftBlockError::Store(format!(
                "read prefix {path:?}: {err}"
            )))
        }
    }
    let len = u64::from_le_bytes(prefix);
    if len == 0 {
        return Ok(None);
    }
    if len > METADATA_REGION_BYTES - LENGTH_PREFIX_BYTES as u64 {
        return Err(RaftBlockError::Store(format!(
            "metadata length {len} exceeds reserved region {METADATA_REGION_BYTES}"
        )));
    }
    let mut buf = vec![0u8; len as usize];
    file.read_exact(&mut buf)
        .map_err(|e| RaftBlockError::Store(format!("read body {path:?}: {e}")))?;
    let meta: SpdkReplicaMeta = serde_json::from_slice(&buf)
        .map_err(|e| RaftBlockError::Store(format!("decode {path:?}: {e}")))?;
    if meta.version != 1 {
        return Err(RaftBlockError::Store(format!(
            "unsupported SPDK replica store version {}",
            meta.version
        )));
    }
    Ok(Some(meta))
}

fn write_meta_to_open_file(
    file: &mut std::fs::File,
    path: &PathBuf,
    encoded: &[u8],
) -> Result<(), RaftBlockError> {
    file.seek(SeekFrom::Start(0))
        .map_err(|e| RaftBlockError::Store(format!("seek {path:?}: {e}")))?;
    file.write_all(&(encoded.len() as u64).to_le_bytes())
        .map_err(|e| RaftBlockError::Store(format!("write prefix {path:?}: {e}")))?;
    file.write_all(encoded)
        .map_err(|e| RaftBlockError::Store(format!("write body {path:?}: {e}")))
}

fn write_full_blocks(
    file: &mut std::fs::File,
    path: &PathBuf,
    bytes: &[u8],
) -> Result<(), RaftBlockError> {
    file.seek(SeekFrom::Start(METADATA_REGION_BYTES))
        .map_err(|e| RaftBlockError::Store(format!("seek blocks {path:?}: {e}")))?;
    file.write_all(bytes)
        .map_err(|e| RaftBlockError::Store(format!("write blocks {path:?}: {e}")))
}

fn write_new_blocks(
    file: &mut std::fs::File,
    path: &PathBuf,
    state: &PersistentReplicaState,
    old_applied: &std::collections::BTreeSet<LogIndex>,
) -> Result<(), RaftBlockError> {
    let new_applied: std::collections::BTreeSet<LogIndex> =
        state.applied_indexes.iter().copied().collect();
    for entry in &state.log {
        if old_applied.contains(&entry.index) || !new_applied.contains(&entry.index) {
            continue;
        }
        if let BlockOp::Write { offset, bytes, .. } = &entry.op {
            file.seek(SeekFrom::Start(METADATA_REGION_BYTES + *offset))
                .map_err(|e| RaftBlockError::Store(format!("seek blocks {path:?}: {e}")))?;
            file.write_all(bytes)
                .map_err(|e| RaftBlockError::Store(format!("write blocks {path:?}: {e}")))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexus_raft_block::{
        BlockCommand, FileReplicaStore, LogIndex, PersistentReplica, PersistentReplicaState,
        Replica,
    };
    use std::sync::Arc;

    /// The on-disk format round-trips: save followed by load yields the
    /// same state. Uses a tempfile in lieu of a real NBD device — the
    /// load/save logic is identical from the perspective of File
    /// read/seek/write operations.
    #[test]
    fn save_load_round_trips_persistent_state() {
        let dir = tempfile::tempdir().unwrap();
        let device = dir.path().join("fake-nbd");
        // Pre-allocate to METADATA_REGION_BYTES so the file is at least
        // as large as the metadata region (NBD-backed lvols are always
        // pre-sized).
        std::fs::File::create(&device)
            .unwrap()
            .set_len(METADATA_REGION_BYTES + 4096)
            .unwrap();

        let store = SpdkLvolReplicaStore::new(&device);

        // Round-trip Empty → None initially (file is zero-filled)
        assert!(store.load().unwrap().is_none(), "fresh device returns None");

        let replica = Replica::new(2, 4096, 512).unwrap();
        let state = PersistentReplicaState::from_replica(&replica, vec![], 0);
        store.save(&state).unwrap();

        let loaded = store.load().unwrap().expect("state present after save");
        // The Replica round-trip is the truthiest assertion: rebuild the
        // replica from the loaded state and verify it matches what we
        // saved.
        assert_eq!(loaded.log, Vec::new());
        assert_eq!(loaded.compacted_through, 0);
        let (loaded_replica, _log, _compacted): (Replica, _, LogIndex) =
            loaded.into_replica().unwrap();
        assert_eq!(loaded_replica.id(), replica.id());
        assert_eq!(loaded_replica.read_all().len(), replica.read_all().len());
    }

    /// A fresh device (no save yet) returns Ok(None), not an error.
    #[test]
    fn missing_device_yields_none() {
        let store = SpdkLvolReplicaStore::new("/nonexistent/path/to/nbd");
        assert!(store.load().unwrap().is_none());
    }

    /// Saving to a device that is not large enough for metadata + blocks
    /// returns a clear error rather than silently truncating.
    #[test]
    fn undersized_device_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let device = dir.path().join("fake-nbd");
        std::fs::File::create(&device)
            .unwrap()
            .set_len(METADATA_REGION_BYTES + 4096)
            .unwrap();
        let store = SpdkLvolReplicaStore::new(&device);

        let replica = Replica::new(1, 8192, 4096).unwrap();
        let state = PersistentReplicaState::from_replica(&replica, vec![], 0);
        let err = store.save(&state).unwrap_err();
        match err {
            RaftBlockError::Store(msg) => {
                assert!(
                    msg.contains("smaller than required"),
                    "unexpected error: {msg}"
                );
            }
            other => panic!("expected Store error, got {other:?}"),
        }
    }

    #[test]
    fn persistent_replica_reopens_from_compacted_spdk_store() {
        let dir = tempfile::tempdir().unwrap();
        let device = dir.path().join("fake-nbd");
        std::fs::File::create(&device)
            .unwrap()
            .set_len(METADATA_REGION_BYTES + 4096)
            .unwrap();
        let external = Arc::new(SpdkLvolReplicaStore::new(&device));
        let store = FileReplicaStore::external(external);
        let mut replica = PersistentReplica::create(store.clone(), 7, 4096, 512).unwrap();
        replica
            .append_command(
                1,
                BlockCommand::Write {
                    offset: 512,
                    bytes: vec![0xAB; 512],
                },
            )
            .unwrap();
        drop(replica);

        let reopened = PersistentReplica::open(store).unwrap().unwrap();
        assert_eq!(reopened.compacted_through(), 1);
        assert!(reopened.log().is_empty());
        assert_eq!(reopened.read_range(512, 512).unwrap(), vec![0xAB; 512]);

        let mut raw = std::fs::File::open(&device).unwrap();
        raw.seek(SeekFrom::Start(METADATA_REGION_BYTES + 512))
            .unwrap();
        let mut block = vec![0; 512];
        raw.read_exact(&mut block).unwrap();
        assert_eq!(block, vec![0xAB; 512]);
    }

    /// The store implements the `ReplicaStoreImpl` trait shape so it can
    /// be wrapped via `FileReplicaStore::external(Arc::new(...))`.
    #[test]
    fn implements_replica_store_impl_via_dyn_dispatch() {
        let dir = tempfile::tempdir().unwrap();
        let device = dir.path().join("fake-nbd");
        std::fs::File::create(&device)
            .unwrap()
            .set_len(8192)
            .unwrap();
        let store = SpdkLvolReplicaStore::new(&device);
        let _trait_obj: std::sync::Arc<dyn ReplicaStoreImpl> = std::sync::Arc::new(store);
    }
}
