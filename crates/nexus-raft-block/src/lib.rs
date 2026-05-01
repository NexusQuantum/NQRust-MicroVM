//! Correctness prototype for B-II replicated block semantics.
//!
//! This crate intentionally does not expose a production storage backend. It is
//! a small deterministic model for log entries, quorum commit, idempotent replay,
//! and repair. The production Raft/SPDK backend should be built only after this
//! model grows enough failure coverage to catch ordering, replay, and stale
//! leader bugs.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Debug;
use std::io::Cursor;
use std::io::{Read, Seek, SeekFrom, Write};
use std::ops::{Bound, RangeBounds};
use std::path::{Path, PathBuf};
use thiserror::Error;

pub type NodeId = u64;
pub type LogIndex = u64;
pub type Term = u64;
pub const OPENRAFT_VERSION: &str = "0.9.24";

openraft::declare_raft_types!(
    pub BlockRaftTypeConfig:
        D = BlockCommand,
        R = BlockResponse,
        NodeId = NodeId,
        Node = openraft::BasicNode,
        Entry = openraft::Entry<BlockRaftTypeConfig>,
        SnapshotData = Cursor<Vec<u8>>,
        Responder = openraft::impls::OneshotResponder<BlockRaftTypeConfig>,
        AsyncRuntime = openraft::TokioRuntime,
);

pub fn default_openraft_config() -> Result<std::sync::Arc<openraft::Config>, RaftBlockError> {
    // Heartbeat / election timing.
    //
    // Why these values: the agent-side network adapter posts append_entries
    // over HTTP+JSON. In nested-KVM environments (KubeVirt) loopback request
    // RTT can spike past 100ms under load (populate streams 64MiB through
    // Raft, each chunk a separate commit). With heartbeat_interval=100 and
    // election timeout starting at 500ms, a follower whose append_entries
    // takes >500ms to round-trip flips to candidate, term climbs, and the
    // group falls into permanent election storm. Bumping heartbeat to 500ms
    // and election timeout to 2.5–5s gives ample slack for HTTP/JSON RPCs
    // under bursty populate load while keeping single-node failure detection
    // under ~5s.
    let config = openraft::Config {
        cluster_name: "nqrust-raft-block".into(),
        heartbeat_interval: 500,
        election_timeout_min: 2500,
        election_timeout_max: 5000,
        ..Default::default()
    };
    config
        .validate()
        .map(std::sync::Arc::new)
        .map_err(|e| RaftBlockError::Store(format!("invalid Openraft config: {e}")))
}

pub fn openraft_log_id(term: Term, leader_id: NodeId, index: LogIndex) -> openraft::LogId<NodeId> {
    openraft::LogId::new(openraft::CommittedLeaderId::new(term, leader_id), index)
}

pub fn openraft_entry(
    term: Term,
    leader_id: NodeId,
    index: LogIndex,
    command: BlockCommand,
) -> openraft::Entry<BlockRaftTypeConfig> {
    openraft::Entry {
        log_id: openraft_log_id(term, leader_id, index),
        payload: openraft::EntryPayload::Normal(command),
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum RaftBlockError {
    #[error("block size must be nonzero")]
    ZeroBlockSize,
    #[error("replica capacity must be a nonzero multiple of block size")]
    InvalidCapacity,
    #[error("write offset/length must align to block size")]
    UnalignedWrite,
    #[error("write is empty")]
    EmptyWrite,
    #[error("write extends past replica capacity")]
    OutOfBounds,
    #[error("replica has no remaining simulated disk capacity")]
    DiskFull,
    #[error("entry checksum mismatch")]
    ChecksumMismatch,
    #[error("entry term {entry_term} is stale; node has seen term {seen_term}")]
    StaleTerm { entry_term: Term, seen_term: Term },
    #[error("not enough acknowledgements for quorum: {acks}/{quorum}")]
    NoQuorum { acks: usize, quorum: usize },
    #[error("node {0} not found")]
    NodeNotFound(NodeId),
    #[error("node {node_id} is not the current leader {leader_id}")]
    NotLeader { node_id: NodeId, leader_id: NodeId },
    #[error("persistent store error: {0}")]
    Store(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlockOp {
    Write {
        offset: u64,
        bytes: Vec<u8>,
        checksum: [u8; 32],
    },
    Flush,
}

impl BlockOp {
    pub fn write(offset: u64, bytes: Vec<u8>) -> Result<Self, RaftBlockError> {
        if bytes.is_empty() {
            return Err(RaftBlockError::EmptyWrite);
        }
        let checksum = checksum_bytes(&bytes);
        Ok(Self::Write {
            offset,
            bytes,
            checksum,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogEntry {
    pub term: Term,
    pub index: LogIndex,
    pub op: BlockOp,
}

impl LogEntry {
    pub fn write(
        term: Term,
        index: LogIndex,
        offset: u64,
        bytes: Vec<u8>,
    ) -> Result<Self, RaftBlockError> {
        Ok(Self {
            term,
            index,
            op: BlockOp::write(offset, bytes)?,
        })
    }

    pub fn flush(term: Term, index: LogIndex) -> Self {
        Self {
            term,
            index,
            op: BlockOp::Flush,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Replica {
    id: NodeId,
    block_size: u64,
    bytes: Vec<u8>,
    highest_term_seen: Term,
    applied: BTreeSet<LogIndex>,
    fail_after_applied_entries: Option<usize>,
}

impl Replica {
    pub fn new(id: NodeId, capacity_bytes: u64, block_size: u64) -> Result<Self, RaftBlockError> {
        if block_size == 0 {
            return Err(RaftBlockError::ZeroBlockSize);
        }
        if capacity_bytes == 0 || !capacity_bytes.is_multiple_of(block_size) {
            return Err(RaftBlockError::InvalidCapacity);
        }
        Ok(Self {
            id,
            block_size,
            bytes: vec![0; capacity_bytes as usize],
            highest_term_seen: 0,
            applied: BTreeSet::new(),
            fail_after_applied_entries: None,
        })
    }

    pub fn id(&self) -> NodeId {
        self.id
    }

    pub fn observe_term(&mut self, term: Term) {
        self.highest_term_seen = self.highest_term_seen.max(term);
    }

    pub fn read_all(&self) -> &[u8] {
        &self.bytes
    }

    pub fn applied_indexes(&self) -> &BTreeSet<LogIndex> {
        &self.applied
    }

    pub fn fail_after_applied_entries(&mut self, entries: usize) {
        self.fail_after_applied_entries = Some(entries);
    }

    pub fn snapshot(&self, last_included_index: LogIndex) -> BlockSnapshot {
        BlockSnapshot {
            replica_id: self.id,
            last_included_index,
            highest_term_seen: self.highest_term_seen,
            bytes: self.bytes.clone(),
        }
    }

    pub fn install_snapshot(&mut self, snapshot: &BlockSnapshot) -> Result<(), RaftBlockError> {
        if snapshot.bytes.len() != self.bytes.len() {
            return Err(RaftBlockError::InvalidCapacity);
        }
        self.bytes.clone_from(&snapshot.bytes);
        self.observe_term(snapshot.highest_term_seen);
        self.applied = (1..=snapshot.last_included_index).collect();
        Ok(())
    }

    pub fn validate_entry(&self, entry: &LogEntry) -> Result<(), RaftBlockError> {
        if entry.term < self.highest_term_seen {
            return Err(RaftBlockError::StaleTerm {
                entry_term: entry.term,
                seen_term: self.highest_term_seen,
            });
        }

        if self.applied.contains(&entry.index) {
            return Ok(());
        }

        match &entry.op {
            BlockOp::Write {
                offset,
                bytes,
                checksum,
            } => {
                validate_write(self.block_size, self.bytes.len() as u64, *offset, bytes)?;
                if checksum_bytes(bytes) != *checksum {
                    return Err(RaftBlockError::ChecksumMismatch);
                }
            }
            BlockOp::Flush => {}
        }

        Ok(())
    }

    pub fn apply(&mut self, entry: &LogEntry) -> Result<bool, RaftBlockError> {
        self.validate_entry(entry)?;
        self.observe_term(entry.term);

        if self.applied.contains(&entry.index) {
            return Ok(false);
        }
        if self
            .fail_after_applied_entries
            .is_some_and(|limit| self.applied.len() >= limit)
        {
            return Err(RaftBlockError::DiskFull);
        }

        if let BlockOp::Write { offset, bytes, .. } = &entry.op {
            let start = *offset as usize;
            let end = start + bytes.len();
            self.bytes[start..end].copy_from_slice(bytes);
        }

        self.applied.insert(entry.index);
        Ok(true)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockSnapshot {
    pub replica_id: NodeId,
    pub last_included_index: LogIndex,
    pub highest_term_seen: Term,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlockCommand {
    Write { offset: u64, bytes: Vec<u8> },
    Flush,
}

impl BlockCommand {
    pub fn into_entry(self, term: Term, index: LogIndex) -> Result<LogEntry, RaftBlockError> {
        match self {
            BlockCommand::Write { offset, bytes } => LogEntry::write(term, index, offset, bytes),
            BlockCommand::Flush => Ok(LogEntry::flush(term, index)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockResponse {
    pub applied_index: LogIndex,
    pub bytes_written: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VoteOutcome {
    pub granted: bool,
    pub term: Term,
    pub voted_for: Option<NodeId>,
    pub committed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistentReplicaState {
    pub node_id: NodeId,
    pub capacity_bytes: u64,
    pub block_size: u64,
    pub highest_term_seen: Term,
    pub applied_indexes: Vec<LogIndex>,
    pub bytes: Vec<u8>,
    pub log: Vec<LogEntry>,
    pub compacted_through: LogIndex,
}

impl PersistentReplicaState {
    pub fn from_replica(
        replica: &Replica,
        log: Vec<LogEntry>,
        compacted_through: LogIndex,
    ) -> Self {
        Self {
            node_id: replica.id,
            capacity_bytes: replica.bytes.len() as u64,
            block_size: replica.block_size,
            highest_term_seen: replica.highest_term_seen,
            applied_indexes: replica.applied.iter().copied().collect(),
            bytes: replica.bytes.clone(),
            log,
            compacted_through,
        }
    }

    pub fn into_replica(self) -> Result<(Replica, Vec<LogEntry>, LogIndex), RaftBlockError> {
        let mut replica = Replica::new(self.node_id, self.capacity_bytes, self.block_size)?;
        if self.bytes.len() != replica.bytes.len() {
            return Err(RaftBlockError::InvalidCapacity);
        }
        replica.bytes = self.bytes;
        replica.highest_term_seen = self.highest_term_seen;
        replica.applied = self.applied_indexes.into_iter().collect();
        Ok((replica, self.log, self.compacted_through))
    }
}

/// Pluggable backend for `FileReplicaStore`. Implementors provide the
/// concrete persistence strategy (JSON-on-filesystem in this crate; SPDK
/// lvol writes via NBD in the agent crate; future Ceph RBD or NVMe-oF
/// in their own crates).
///
/// The trait is consumed only via `FileReplicaStore::external(...)`; the
/// existing constructor `FileReplicaStore::new(path)` keeps the
/// filesystem-backed behavior with no changes for callers.
pub trait ReplicaStoreImpl: Send + Sync + std::fmt::Debug {
    /// Read the persisted replica state, or `Ok(None)` if no prior state
    /// is durable yet (fresh deployment / first call before the first
    /// successful save).
    fn load(&self) -> Result<Option<PersistentReplicaState>, RaftBlockError>;

    /// Atomically persist `state` such that a subsequent load() returns
    /// it. Implementations must be crash-safe: a partial write must not
    /// corrupt a prior valid load result.
    fn save(&self, state: &PersistentReplicaState) -> Result<(), RaftBlockError>;
}

/// `Clone`-able store handle used throughout the crate. Internally it
/// dispatches to either the JSON-on-filesystem path (existing default
/// behavior, used by all current callers and tests) or an external
/// `ReplicaStoreImpl` (e.g. SPDK lvol on the agent side).
///
/// The name is preserved for backward compatibility with all callers
/// that take `FileReplicaStore` by value; new code can construct the
/// external variant via `FileReplicaStore::external(...)`.
#[derive(Debug, Clone)]
pub struct FileReplicaStore {
    inner: ReplicaStoreKind,
}

#[derive(Debug, Clone)]
enum ReplicaStoreKind {
    /// Filesystem-backed `PersistentReplicaState`. New writes use a
    /// sidecar directory with split metadata/block/log files; legacy
    /// monolithic JSON files still load.
    JsonFile(PathBuf),
    /// External implementation. Boxed because the impl may be
    /// agent-specific (e.g. holds an HTTP client to local SPDK).
    External(std::sync::Arc<dyn ReplicaStoreImpl>),
    /// No-op: never persists. `load()` always returns `None`. Used by
    /// smoke tests where crash-recovery semantics aren't needed and the
    /// O(N²) cost of full-state JSON rewrites would dominate runtime.
    NoOp,
}

impl FileReplicaStore {
    /// Construct the JSON-on-filesystem variant (backward-compatible).
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            inner: ReplicaStoreKind::JsonFile(path.into()),
        }
    }

    /// Construct an external-backend variant. The caller is responsible
    /// for the impl's correctness (atomicity, crash-safety). The `Arc`
    /// is cheap to clone and already shared across the lib's clones of
    /// the store handle.
    pub fn external(impl_: std::sync::Arc<dyn ReplicaStoreImpl>) -> Self {
        Self {
            inner: ReplicaStoreKind::External(impl_),
        }
    }

    /// In-memory store that never writes to disk. `load()` always
    /// returns `None`, `save()` is a no-op. Intended for smoke tests
    /// and ephemeral operator setups where the JSON path's per-write
    /// O(N²) full-state rewrite dominates runtime. Crash recovery is
    /// forfeited.
    pub fn in_memory() -> Self {
        Self {
            inner: ReplicaStoreKind::NoOp,
        }
    }

    /// Read the persisted state. Returns `Ok(None)` if nothing has been
    /// saved yet (the JSON file is missing, or the external store
    /// reports no state).
    pub fn load(&self) -> Result<Option<PersistentReplicaState>, RaftBlockError> {
        match &self.inner {
            ReplicaStoreKind::JsonFile(path) => load_json(path),
            ReplicaStoreKind::External(impl_) => impl_.load(),
            ReplicaStoreKind::NoOp => Ok(None),
        }
    }

    /// Persist `state`. Atomic: a partial failure must not leave a
    /// corrupt prior state visible to a subsequent load.
    pub fn save(&self, state: &PersistentReplicaState) -> Result<(), RaftBlockError> {
        match &self.inner {
            ReplicaStoreKind::JsonFile(path) => save_json(path, state),
            ReplicaStoreKind::External(impl_) => impl_.save(state),
            ReplicaStoreKind::NoOp => Ok(()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SidecarReplicaMeta {
    version: u32,
    node_id: NodeId,
    capacity_bytes: u64,
    block_size: u64,
    highest_term_seen: Term,
    applied_indexes: Vec<LogIndex>,
    compacted_through: LogIndex,
    log_len: usize,
}

impl SidecarReplicaMeta {
    fn from_state(state: &PersistentReplicaState) -> Self {
        Self {
            version: 1,
            node_id: state.node_id,
            capacity_bytes: state.capacity_bytes,
            block_size: state.block_size,
            highest_term_seen: state.highest_term_seen,
            applied_indexes: state.applied_indexes.clone(),
            compacted_through: state.compacted_through,
            log_len: state.log.len(),
        }
    }
}

#[derive(Debug, Clone)]
struct SidecarPaths {
    dir: PathBuf,
    meta: PathBuf,
    blocks: PathBuf,
    log: PathBuf,
}

fn sidecar_paths(path: &Path) -> SidecarPaths {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("replica-state");
    let dir = path.with_file_name(format!("{file_name}.d"));
    SidecarPaths {
        meta: dir.join("meta.json"),
        blocks: dir.join("blocks.bin"),
        log: dir.join("log.bin"),
        dir,
    }
}

fn load_json(path: &Path) -> Result<Option<PersistentReplicaState>, RaftBlockError> {
    if sidecar_paths(path).meta.exists() {
        return load_sidecar(path);
    }
    if !path.exists() {
        return Ok(None);
    }
    let mut file = std::fs::File::open(path)
        .map_err(|e| RaftBlockError::Store(format!("open {path:?}: {e}")))?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)
        .map_err(|e| RaftBlockError::Store(format!("read {path:?}: {e}")))?;
    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(|e| RaftBlockError::Store(format!("decode {path:?}: {e}")))
}

fn save_json(path: &Path, state: &PersistentReplicaState) -> Result<(), RaftBlockError> {
    save_sidecar(path, state)
}

#[allow(dead_code)]
fn save_legacy_json(path: &Path, state: &PersistentReplicaState) -> Result<(), RaftBlockError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| RaftBlockError::Store(format!("create {parent:?}: {e}")))?;
    }
    let tmp_path = tmp_path_for(path);
    let encoded = serde_json::to_vec(state)
        .map_err(|e| RaftBlockError::Store(format!("encode {path:?}: {e}")))?;
    {
        let mut file = std::fs::File::create(&tmp_path)
            .map_err(|e| RaftBlockError::Store(format!("create {tmp_path:?}: {e}")))?;
        file.write_all(&encoded)
            .map_err(|e| RaftBlockError::Store(format!("write {tmp_path:?}: {e}")))?;
        file.sync_all()
            .map_err(|e| RaftBlockError::Store(format!("sync {tmp_path:?}: {e}")))?;
    }
    std::fs::rename(&tmp_path, path)
        .map_err(|e| RaftBlockError::Store(format!("rename {tmp_path:?}: {e}")))?;
    Ok(())
}

fn load_sidecar(path: &Path) -> Result<Option<PersistentReplicaState>, RaftBlockError> {
    let paths = sidecar_paths(path);
    let Some(meta) = load_sidecar_meta(&paths.meta)? else {
        return Ok(None);
    };
    let bytes = std::fs::read(&paths.blocks).map_err(|e| {
        RaftBlockError::Store(format!("read sidecar blocks {:?}: {e}", paths.blocks))
    })?;
    if bytes.len() as u64 != meta.capacity_bytes {
        return Err(RaftBlockError::Store(format!(
            "sidecar blocks length {} does not match capacity {}",
            bytes.len(),
            meta.capacity_bytes
        )));
    }
    let log = read_sidecar_log(&paths.log)?;
    if log.len() != meta.log_len {
        return Err(RaftBlockError::Store(format!(
            "sidecar log length {} does not match meta length {}",
            log.len(),
            meta.log_len
        )));
    }
    Ok(Some(PersistentReplicaState {
        node_id: meta.node_id,
        capacity_bytes: meta.capacity_bytes,
        block_size: meta.block_size,
        highest_term_seen: meta.highest_term_seen,
        applied_indexes: meta.applied_indexes,
        bytes,
        log,
        compacted_through: meta.compacted_through,
    }))
}

fn save_sidecar(path: &Path, state: &PersistentReplicaState) -> Result<(), RaftBlockError> {
    let paths = sidecar_paths(path);
    std::fs::create_dir_all(&paths.dir)
        .map_err(|e| RaftBlockError::Store(format!("create sidecar dir {:?}: {e}", paths.dir)))?;

    let previous_meta = load_sidecar_meta(&paths.meta)?;
    let rewrite_all = previous_meta.as_ref().is_none_or(|meta| {
        meta.node_id != state.node_id
            || meta.capacity_bytes != state.capacity_bytes
            || meta.block_size != state.block_size
            || meta.compacted_through != state.compacted_through
            || state.log.len() < meta.log_len
    });

    if rewrite_all {
        write_full_blocks(&paths.blocks, &state.bytes)?;
        rewrite_sidecar_log(&paths.log, &state.log)?;
    } else if let Some(meta) = previous_meta.as_ref() {
        ensure_blocks_file(&paths.blocks, state.capacity_bytes)?;
        let old_applied: BTreeSet<LogIndex> = meta.applied_indexes.iter().copied().collect();
        apply_new_writes_to_blocks(&paths.blocks, &old_applied, state)?;
        if state.log.len() > meta.log_len {
            append_sidecar_log(&paths.log, &state.log[meta.log_len..])?;
        }
    }

    write_json_atomically(&paths.meta, &SidecarReplicaMeta::from_state(state))
}

fn load_sidecar_meta(path: &Path) -> Result<Option<SidecarReplicaMeta>, RaftBlockError> {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => {
            return Err(RaftBlockError::Store(format!(
                "read sidecar meta {path:?}: {err}"
            )))
        }
    };
    let meta: SidecarReplicaMeta = serde_json::from_slice(&bytes)
        .map_err(|e| RaftBlockError::Store(format!("decode sidecar meta {path:?}: {e}")))?;
    if meta.version != 1 {
        return Err(RaftBlockError::Store(format!(
            "unsupported sidecar replica store version {}",
            meta.version
        )));
    }
    Ok(Some(meta))
}

fn ensure_blocks_file(path: &Path, capacity_bytes: u64) -> Result<(), RaftBlockError> {
    let file = std::fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(false)
        .open(path)
        .map_err(|e| RaftBlockError::Store(format!("open sidecar blocks {path:?}: {e}")))?;
    let current_len = file
        .metadata()
        .map_err(|e| RaftBlockError::Store(format!("stat sidecar blocks {path:?}: {e}")))?
        .len();
    if current_len != capacity_bytes {
        file.set_len(capacity_bytes)
            .map_err(|e| RaftBlockError::Store(format!("resize sidecar blocks {path:?}: {e}")))?;
    }
    Ok(())
}

fn write_full_blocks(path: &Path, bytes: &[u8]) -> Result<(), RaftBlockError> {
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(path)
        .map_err(|e| RaftBlockError::Store(format!("create sidecar blocks {path:?}: {e}")))?;
    file.write_all(bytes)
        .map_err(|e| RaftBlockError::Store(format!("write sidecar blocks {path:?}: {e}")))?;
    file.sync_all()
        .map_err(|e| RaftBlockError::Store(format!("sync sidecar blocks {path:?}: {e}")))
}

fn apply_new_writes_to_blocks(
    path: &Path,
    old_applied: &BTreeSet<LogIndex>,
    state: &PersistentReplicaState,
) -> Result<(), RaftBlockError> {
    let new_applied: BTreeSet<LogIndex> = state.applied_indexes.iter().copied().collect();
    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .map_err(|e| RaftBlockError::Store(format!("open sidecar blocks {path:?}: {e}")))?;
    for entry in &state.log {
        if old_applied.contains(&entry.index) || !new_applied.contains(&entry.index) {
            continue;
        }
        if let BlockOp::Write { offset, bytes, .. } = &entry.op {
            file.seek(SeekFrom::Start(*offset))
                .map_err(|e| RaftBlockError::Store(format!("seek sidecar blocks {path:?}: {e}")))?;
            file.write_all(bytes).map_err(|e| {
                RaftBlockError::Store(format!("write sidecar blocks {path:?}: {e}"))
            })?;
        }
    }
    file.sync_all()
        .map_err(|e| RaftBlockError::Store(format!("sync sidecar blocks {path:?}: {e}")))
}

fn read_sidecar_log(path: &Path) -> Result<Vec<LogEntry>, RaftBlockError> {
    let mut file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => {
            return Err(RaftBlockError::Store(format!(
                "open sidecar log {path:?}: {err}"
            )))
        }
    };
    let mut entries = Vec::new();
    loop {
        let mut prefix = [0u8; 8];
        match file.read_exact(&mut prefix) {
            Ok(()) => {}
            Err(err) if err.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(err) => {
                return Err(RaftBlockError::Store(format!(
                    "read sidecar log prefix {path:?}: {err}"
                )))
            }
        }
        let len = u64::from_le_bytes(prefix);
        if len == 0 {
            return Err(RaftBlockError::Store(format!(
                "zero-length sidecar log entry in {path:?}"
            )));
        }
        let mut buf = vec![0u8; len as usize];
        file.read_exact(&mut buf)
            .map_err(|e| RaftBlockError::Store(format!("read sidecar log body {path:?}: {e}")))?;
        entries.push(
            serde_json::from_slice(&buf)
                .map_err(|e| RaftBlockError::Store(format!("decode sidecar log {path:?}: {e}")))?,
        );
    }
    Ok(entries)
}

fn append_sidecar_log(path: &Path, entries: &[LogEntry]) -> Result<(), RaftBlockError> {
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| RaftBlockError::Store(format!("open sidecar log {path:?}: {e}")))?;
    write_log_entries(&mut file, path, entries)?;
    file.sync_all()
        .map_err(|e| RaftBlockError::Store(format!("sync sidecar log {path:?}: {e}")))
}

fn rewrite_sidecar_log(path: &Path, entries: &[LogEntry]) -> Result<(), RaftBlockError> {
    let tmp_path = tmp_path_for(path);
    {
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&tmp_path)
            .map_err(|e| RaftBlockError::Store(format!("create sidecar log {tmp_path:?}: {e}")))?;
        write_log_entries(&mut file, path, entries)?;
        file.sync_all()
            .map_err(|e| RaftBlockError::Store(format!("sync sidecar log {tmp_path:?}: {e}")))?;
    }
    std::fs::rename(&tmp_path, path)
        .map_err(|e| RaftBlockError::Store(format!("rename {tmp_path:?} -> {path:?}: {e}")))
}

fn write_log_entries(
    file: &mut std::fs::File,
    path: &Path,
    entries: &[LogEntry],
) -> Result<(), RaftBlockError> {
    for entry in entries {
        let encoded = serde_json::to_vec(entry)
            .map_err(|e| RaftBlockError::Store(format!("encode sidecar log {path:?}: {e}")))?;
        file.write_all(&(encoded.len() as u64).to_le_bytes())
            .map_err(|e| {
                RaftBlockError::Store(format!("write sidecar log prefix {path:?}: {e}"))
            })?;
        file.write_all(&encoded)
            .map_err(|e| RaftBlockError::Store(format!("write sidecar log body {path:?}: {e}")))?;
    }
    Ok(())
}

fn write_json_atomically<T: Serialize>(path: &Path, value: &T) -> Result<(), RaftBlockError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| RaftBlockError::Store(format!("create {parent:?}: {e}")))?;
    }
    let tmp_path = tmp_path_for(path);
    let encoded = serde_json::to_vec(value)
        .map_err(|e| RaftBlockError::Store(format!("encode {path:?}: {e}")))?;
    {
        let mut file = std::fs::File::create(&tmp_path)
            .map_err(|e| RaftBlockError::Store(format!("create {tmp_path:?}: {e}")))?;
        file.write_all(&encoded)
            .map_err(|e| RaftBlockError::Store(format!("write {tmp_path:?}: {e}")))?;
        file.sync_all()
            .map_err(|e| RaftBlockError::Store(format!("sync {tmp_path:?}: {e}")))?;
    }
    std::fs::rename(&tmp_path, path)
        .map_err(|e| RaftBlockError::Store(format!("rename {tmp_path:?}: {e}")))
}

fn tmp_path_for(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("replica-state");
    path.with_file_name(format!("{file_name}.tmp"))
}

#[derive(Debug, Clone)]
pub struct PersistentReplica {
    replica: Replica,
    log: Vec<LogEntry>,
    compacted_through: LogIndex,
    next_index: LogIndex,
    store: FileReplicaStore,
}

impl PersistentReplica {
    pub fn create(
        store: FileReplicaStore,
        node_id: NodeId,
        capacity_bytes: u64,
        block_size: u64,
    ) -> Result<Self, RaftBlockError> {
        let replica = Replica::new(node_id, capacity_bytes, block_size)?;
        let out = Self {
            replica,
            log: Vec::new(),
            compacted_through: 0,
            next_index: 1,
            store,
        };
        out.persist()?;
        Ok(out)
    }

    pub fn open(store: FileReplicaStore) -> Result<Option<Self>, RaftBlockError> {
        let Some(state) = store.load()? else {
            return Ok(None);
        };
        let (replica, log, compacted_through) = state.into_replica()?;
        let next_index = log
            .iter()
            .map(|entry| entry.index)
            .max()
            .unwrap_or(compacted_through)
            + 1;
        Ok(Some(Self {
            replica,
            log,
            compacted_through,
            next_index,
            store,
        }))
    }

    pub fn append_command(
        &mut self,
        term: Term,
        command: BlockCommand,
    ) -> Result<BlockResponse, RaftBlockError> {
        let entry = command.into_entry(term, self.next_index)?;
        self.append_entry(entry)
    }

    pub fn append_entry(&mut self, entry: LogEntry) -> Result<BlockResponse, RaftBlockError> {
        self.replica.apply(&entry)?;
        let bytes_written = match &entry.op {
            BlockOp::Write { bytes, .. } => bytes.len() as u64,
            BlockOp::Flush => 0,
        };
        self.next_index = self.next_index.max(entry.index + 1);
        self.log.push(entry.clone());
        self.persist()?;
        Ok(BlockResponse {
            applied_index: entry.index,
            bytes_written,
        })
    }

    pub fn install_snapshot(&mut self, snapshot: &BlockSnapshot) -> Result<(), RaftBlockError> {
        self.replica.install_snapshot(snapshot)?;
        self.log
            .retain(|entry| entry.index > snapshot.last_included_index);
        self.compacted_through = self.compacted_through.max(snapshot.last_included_index);
        self.next_index = self.next_index.max(snapshot.last_included_index + 1);
        self.persist()
    }

    pub fn snapshot(&self) -> BlockSnapshot {
        let last_applied = self
            .replica
            .applied_indexes()
            .iter()
            .next_back()
            .copied()
            .unwrap_or(self.compacted_through);
        self.replica.snapshot(last_applied)
    }

    pub fn read_all(&self) -> &[u8] {
        self.replica.read_all()
    }

    pub fn node_id(&self) -> NodeId {
        self.replica.id()
    }

    pub fn capacity_bytes(&self) -> u64 {
        self.replica.read_all().len() as u64
    }

    pub fn block_size(&self) -> u64 {
        self.replica.block_size
    }

    pub fn compacted_through(&self) -> LogIndex {
        self.compacted_through
    }

    pub fn last_applied_index(&self) -> LogIndex {
        self.replica
            .applied_indexes()
            .iter()
            .next_back()
            .copied()
            .unwrap_or(self.compacted_through)
    }

    pub fn read_range(&self, offset: u64, len: usize) -> Result<Vec<u8>, RaftBlockError> {
        let end = offset
            .checked_add(len as u64)
            .ok_or(RaftBlockError::OutOfBounds)?;
        if end > self.replica.read_all().len() as u64 {
            return Err(RaftBlockError::OutOfBounds);
        }
        Ok(self.replica.read_all()[offset as usize..end as usize].to_vec())
    }

    pub fn log(&self) -> &[LogEntry] {
        &self.log
    }

    fn persist(&self) -> Result<(), RaftBlockError> {
        self.store.save(&PersistentReplicaState::from_replica(
            &self.replica,
            self.log.clone(),
            self.compacted_through,
        ))
    }
}

#[derive(Debug, Clone)]
pub struct OpenraftEntryApplier {
    replica: PersistentReplica,
    last_applied_log_id: Option<openraft::LogId<NodeId>>,
    last_membership: openraft::StoredMembership<NodeId, openraft::BasicNode>,
}

impl OpenraftEntryApplier {
    pub fn create(
        store: FileReplicaStore,
        node_id: NodeId,
        capacity_bytes: u64,
        block_size: u64,
    ) -> Result<Self, RaftBlockError> {
        Ok(Self {
            replica: PersistentReplica::create(store, node_id, capacity_bytes, block_size)?,
            last_applied_log_id: None,
            last_membership: openraft::StoredMembership::default(),
        })
    }

    pub fn open(store: FileReplicaStore) -> Result<Option<Self>, RaftBlockError> {
        let Some(replica) = PersistentReplica::open(store)? else {
            return Ok(None);
        };
        let last_applied_log_id = replica
            .log()
            .last()
            .map(|entry| openraft_log_id(entry.term, replica.node_id(), entry.index))
            .or_else(|| {
                let compacted_through = replica.compacted_through();
                (compacted_through > 0).then(|| {
                    openraft_log_id(
                        replica.snapshot().highest_term_seen,
                        replica.node_id(),
                        compacted_through,
                    )
                })
            });
        Ok(Some(Self {
            replica,
            last_applied_log_id,
            last_membership: openraft::StoredMembership::default(),
        }))
    }

    pub fn apply_entries<I>(&mut self, entries: I) -> Result<Vec<BlockResponse>, RaftBlockError>
    where
        I: IntoIterator<Item = openraft::Entry<BlockRaftTypeConfig>>,
    {
        let mut responses = Vec::new();
        for entry in entries {
            let response = match entry.payload {
                openraft::EntryPayload::Blank => BlockResponse {
                    applied_index: entry.log_id.index,
                    bytes_written: 0,
                },
                openraft::EntryPayload::Normal(command) => {
                    let block_entry =
                        command.into_entry(entry.log_id.leader_id.term, entry.log_id.index)?;
                    self.replica.append_entry(block_entry)?
                }
                openraft::EntryPayload::Membership(membership) => {
                    self.last_membership =
                        openraft::StoredMembership::new(Some(entry.log_id), membership);
                    BlockResponse {
                        applied_index: entry.log_id.index,
                        bytes_written: 0,
                    }
                }
            };
            self.last_applied_log_id = Some(entry.log_id);
            responses.push(response);
        }
        Ok(responses)
    }

    pub fn append_command(
        &mut self,
        term: Term,
        leader_id: NodeId,
        command: BlockCommand,
    ) -> Result<BlockResponse, RaftBlockError> {
        let index = self.replica.next_index;
        let mut responses =
            self.apply_entries([openraft_entry(term, leader_id, index, command)])?;
        responses
            .pop()
            .ok_or_else(|| RaftBlockError::Store("openraft append produced no response".into()))
    }

    pub fn install_snapshot(&mut self, snapshot: &BlockSnapshot) -> Result<(), RaftBlockError> {
        self.replica.install_snapshot(snapshot)?;
        self.last_applied_log_id = Some(openraft_log_id(
            snapshot.highest_term_seen,
            self.node_id(),
            snapshot.last_included_index,
        ));
        Ok(())
    }

    pub fn last_applied_log_id(&self) -> Option<openraft::LogId<NodeId>> {
        self.last_applied_log_id
    }

    pub fn last_membership(&self) -> &openraft::StoredMembership<NodeId, openraft::BasicNode> {
        &self.last_membership
    }

    pub fn replica(&self) -> &PersistentReplica {
        &self.replica
    }

    pub fn node_id(&self) -> NodeId {
        self.replica.node_id()
    }
}

#[derive(Debug, Clone)]
pub struct OpenraftBlockSnapshotBuilder {
    store: InMemoryOpenraftBlockStore,
}

#[derive(Debug, Clone)]
pub struct InMemoryOpenraftBlockStore {
    inner: std::sync::Arc<std::sync::Mutex<InMemoryOpenraftBlockStoreInner>>,
}

#[derive(Debug)]
struct InMemoryOpenraftBlockStoreInner {
    vote: Option<openraft::Vote<NodeId>>,
    committed: Option<openraft::LogId<NodeId>>,
    logs: BTreeMap<LogIndex, openraft::Entry<BlockRaftTypeConfig>>,
    last_purged_log_id: Option<openraft::LogId<NodeId>>,
    applier: OpenraftEntryApplier,
}

impl InMemoryOpenraftBlockStore {
    pub fn create(
        store: FileReplicaStore,
        node_id: NodeId,
        capacity_bytes: u64,
        block_size: u64,
    ) -> Result<Self, RaftBlockError> {
        Ok(Self {
            inner: std::sync::Arc::new(std::sync::Mutex::new(InMemoryOpenraftBlockStoreInner {
                vote: None,
                committed: None,
                logs: BTreeMap::new(),
                last_purged_log_id: None,
                applier: OpenraftEntryApplier::create(store, node_id, capacity_bytes, block_size)?,
            })),
        })
    }

    pub fn open_or_create(
        store: FileReplicaStore,
        node_id: NodeId,
        capacity_bytes: u64,
        block_size: u64,
    ) -> Result<Self, RaftBlockError> {
        let applier = if let Some(existing) = OpenraftEntryApplier::open(store.clone())? {
            existing
        } else {
            OpenraftEntryApplier::create(store, node_id, capacity_bytes, block_size)?
        };
        if applier.node_id() != node_id
            || applier.replica().capacity_bytes() != capacity_bytes
            || applier.replica().block_size() != block_size
        {
            return Err(RaftBlockError::Store(format!(
                "openraft block store exists with node_id={}, capacity_bytes={}, block_size={}; requested node_id={}, capacity_bytes={}, block_size={}",
                applier.node_id(),
                applier.replica().capacity_bytes(),
                applier.replica().block_size(),
                node_id,
                capacity_bytes,
                block_size
            )));
        }
        Ok(Self::from_applier(applier))
    }

    pub fn open_existing(store: FileReplicaStore) -> Result<Option<Self>, RaftBlockError> {
        OpenraftEntryApplier::open(store).map(|applier| applier.map(Self::from_applier))
    }

    fn from_applier(applier: OpenraftEntryApplier) -> Self {
        let node_id = applier.node_id();
        let logs = applier
            .replica()
            .log()
            .iter()
            .map(|entry| (entry.index, block_log_entry_to_openraft(entry, node_id)))
            .collect();
        Self {
            inner: std::sync::Arc::new(std::sync::Mutex::new(InMemoryOpenraftBlockStoreInner {
                vote: None,
                committed: applier.last_applied_log_id(),
                logs,
                last_purged_log_id: if applier.replica().compacted_through() == 0 {
                    None
                } else {
                    Some(openraft_log_id(
                        applier.replica().snapshot().highest_term_seen,
                        node_id,
                        applier.replica().compacted_through(),
                    ))
                },
                applier,
            })),
        }
    }

    pub fn append_command(
        &self,
        term: Term,
        leader_id: NodeId,
        command: BlockCommand,
    ) -> Result<BlockResponse, RaftBlockError> {
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| RaftBlockError::Store("openraft store lock poisoned".into()))?;
        let index = inner.applier.replica().next_index;
        let entry = openraft_entry(term, leader_id, index, command);
        inner.logs.insert(index, entry.clone());
        let mut responses = inner.applier.apply_entries([entry])?;
        inner.committed = inner.applier.last_applied_log_id();
        responses
            .pop()
            .ok_or_else(|| RaftBlockError::Store("openraft append produced no response".into()))
    }

    pub fn append_openraft_entries(
        &self,
        entries: impl IntoIterator<Item = openraft::Entry<BlockRaftTypeConfig>>,
    ) -> Result<Vec<BlockResponse>, RaftBlockError> {
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| RaftBlockError::Store("openraft store lock poisoned".into()))?;
        let entries = entries.into_iter().collect::<Vec<_>>();
        for (expected_index, entry) in (inner.applier.replica().next_index..).zip(entries.iter()) {
            if entry.log_id.index != expected_index {
                return Err(RaftBlockError::Store(format!(
                    "openraft append_entries expected index {}, got {}",
                    expected_index, entry.log_id.index
                )));
            }
        }
        for entry in &entries {
            inner.logs.insert(entry.log_id.index, entry.clone());
        }
        let responses = inner.applier.apply_entries(entries)?;
        inner.committed = inner.applier.last_applied_log_id();
        Ok(responses)
    }

    pub fn request_vote(
        &self,
        term: Term,
        candidate_id: NodeId,
    ) -> Result<VoteOutcome, RaftBlockError> {
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| RaftBlockError::Store("openraft store lock poisoned".into()))?;
        let requested = openraft::Vote::new(term, candidate_id);
        let granted = match inner.vote {
            Some(current)
                if current.leader_id.term == term
                    && current.leader_id.voted_for().is_some()
                    && current.leader_id.voted_for() != Some(candidate_id) =>
            {
                false
            }
            None => {
                inner.vote = Some(requested);
                true
            }
            Some(current) if requested > current => {
                inner.vote = Some(requested);
                true
            }
            Some(current) if requested == current => true,
            Some(_) => false,
        };
        Ok(vote_outcome(inner.vote.unwrap_or_default(), granted))
    }

    pub fn current_vote(&self) -> Result<VoteOutcome, RaftBlockError> {
        let inner = self
            .inner
            .lock()
            .map_err(|_| RaftBlockError::Store("openraft store lock poisoned".into()))?;
        Ok(vote_outcome(inner.vote.unwrap_or_default(), false))
    }

    pub fn block_snapshot(&self) -> Result<BlockSnapshot, RaftBlockError> {
        let inner = self
            .inner
            .lock()
            .map_err(|_| RaftBlockError::Store("openraft store lock poisoned".into()))?;
        Ok(inner.applier.replica().snapshot())
    }

    pub fn install_block_snapshot(&self, snapshot: &BlockSnapshot) -> Result<(), RaftBlockError> {
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| RaftBlockError::Store("openraft store lock poisoned".into()))?;
        inner.applier.install_snapshot(snapshot)?;
        inner
            .logs
            .retain(|index, _| *index > snapshot.last_included_index);
        inner.committed = inner.applier.last_applied_log_id();
        Ok(())
    }

    pub fn install_openraft_snapshot(
        &self,
        meta: &openraft::SnapshotMeta<NodeId, openraft::BasicNode>,
        snapshot: &BlockSnapshot,
    ) -> Result<(), RaftBlockError> {
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| RaftBlockError::Store("openraft store lock poisoned".into()))?;
        inner.applier.install_snapshot(snapshot)?;
        inner.applier.last_applied_log_id = meta.last_log_id;
        inner.applier.last_membership = meta.last_membership.clone();
        inner
            .logs
            .retain(|index, _| meta.last_log_id.is_none_or(|log_id| *index > log_id.index));
        inner.committed = meta.last_log_id;
        Ok(())
    }

    pub fn read_range(&self, offset: u64, len: usize) -> Result<Vec<u8>, RaftBlockError> {
        let inner = self
            .inner
            .lock()
            .map_err(|_| RaftBlockError::Store("openraft store lock poisoned".into()))?;
        inner.applier.replica().read_range(offset, len)
    }

    pub fn node_id(&self) -> Result<NodeId, RaftBlockError> {
        let inner = self
            .inner
            .lock()
            .map_err(|_| RaftBlockError::Store("openraft store lock poisoned".into()))?;
        Ok(inner.applier.node_id())
    }

    pub fn capacity_bytes(&self) -> Result<u64, RaftBlockError> {
        let inner = self
            .inner
            .lock()
            .map_err(|_| RaftBlockError::Store("openraft store lock poisoned".into()))?;
        Ok(inner.applier.replica().capacity_bytes())
    }

    pub fn block_size(&self) -> Result<u64, RaftBlockError> {
        let inner = self
            .inner
            .lock()
            .map_err(|_| RaftBlockError::Store("openraft store lock poisoned".into()))?;
        Ok(inner.applier.replica().block_size())
    }

    pub fn last_applied_index(&self) -> Result<LogIndex, RaftBlockError> {
        let inner = self
            .inner
            .lock()
            .map_err(|_| RaftBlockError::Store("openraft store lock poisoned".into()))?;
        Ok(inner.applier.replica().last_applied_index())
    }

    pub fn compacted_through(&self) -> Result<LogIndex, RaftBlockError> {
        let inner = self
            .inner
            .lock()
            .map_err(|_| RaftBlockError::Store("openraft store lock poisoned".into()))?;
        Ok(inner.applier.replica().compacted_through())
    }

    pub fn retained_log_entries(&self) -> Result<u64, RaftBlockError> {
        let inner = self
            .inner
            .lock()
            .map_err(|_| RaftBlockError::Store("openraft store lock poisoned".into()))?;
        Ok(inner.logs.len() as u64)
    }
}

fn vote_outcome(vote: openraft::Vote<NodeId>, granted: bool) -> VoteOutcome {
    VoteOutcome {
        granted,
        term: vote.leader_id.term,
        voted_for: vote.leader_id.voted_for(),
        committed: vote.committed,
    }
}

fn block_log_entry_to_openraft(
    entry: &LogEntry,
    leader_id: NodeId,
) -> openraft::Entry<BlockRaftTypeConfig> {
    let command = match &entry.op {
        BlockOp::Write { offset, bytes, .. } => BlockCommand::Write {
            offset: *offset,
            bytes: bytes.clone(),
        },
        BlockOp::Flush => BlockCommand::Flush,
    };
    openraft_entry(entry.term, leader_id, entry.index, command)
}

impl openraft::storage::RaftLogReader<BlockRaftTypeConfig> for InMemoryOpenraftBlockStore {
    async fn try_get_log_entries<RB: RangeBounds<u64> + Clone + Debug + openraft::OptionalSend>(
        &mut self,
        range: RB,
    ) -> Result<Vec<openraft::Entry<BlockRaftTypeConfig>>, openraft::StorageError<NodeId>> {
        let inner = self.inner.lock().map_err(openraft_lock_error)?;
        Ok(inner
            .logs
            .iter()
            .filter(|(index, _)| range_contains(&range, **index))
            .map(|(_, entry)| entry.clone())
            .collect())
    }
}

impl openraft::storage::RaftStorage<BlockRaftTypeConfig> for InMemoryOpenraftBlockStore {
    type LogReader = Self;
    type SnapshotBuilder = OpenraftBlockSnapshotBuilder;

    async fn save_vote(
        &mut self,
        vote: &openraft::Vote<NodeId>,
    ) -> Result<(), openraft::StorageError<NodeId>> {
        self.inner.lock().map_err(openraft_lock_error)?.vote = Some(*vote);
        Ok(())
    }

    async fn read_vote(
        &mut self,
    ) -> Result<Option<openraft::Vote<NodeId>>, openraft::StorageError<NodeId>> {
        Ok(self.inner.lock().map_err(openraft_lock_error)?.vote)
    }

    async fn save_committed(
        &mut self,
        committed: Option<openraft::LogId<NodeId>>,
    ) -> Result<(), openraft::StorageError<NodeId>> {
        self.inner.lock().map_err(openraft_lock_error)?.committed = committed;
        Ok(())
    }

    async fn read_committed(
        &mut self,
    ) -> Result<Option<openraft::LogId<NodeId>>, openraft::StorageError<NodeId>> {
        Ok(self.inner.lock().map_err(openraft_lock_error)?.committed)
    }

    async fn get_log_state(
        &mut self,
    ) -> Result<openraft::storage::LogState<BlockRaftTypeConfig>, openraft::StorageError<NodeId>>
    {
        let inner = self.inner.lock().map_err(openraft_lock_error)?;
        let last_log_id = inner
            .logs
            .values()
            .next_back()
            .map(|entry| entry.log_id)
            .or(inner.last_purged_log_id);
        Ok(openraft::storage::LogState {
            last_purged_log_id: inner.last_purged_log_id,
            last_log_id,
        })
    }

    async fn get_log_reader(&mut self) -> Self::LogReader {
        self.clone()
    }

    async fn append_to_log<I>(&mut self, entries: I) -> Result<(), openraft::StorageError<NodeId>>
    where
        I: IntoIterator<Item = openraft::Entry<BlockRaftTypeConfig>> + openraft::OptionalSend,
    {
        let mut inner = self.inner.lock().map_err(openraft_lock_error)?;
        for entry in entries {
            inner.logs.insert(entry.log_id.index, entry);
        }
        Ok(())
    }

    async fn delete_conflict_logs_since(
        &mut self,
        log_id: openraft::LogId<NodeId>,
    ) -> Result<(), openraft::StorageError<NodeId>> {
        self.inner
            .lock()
            .map_err(openraft_lock_error)?
            .logs
            .split_off(&log_id.index);
        Ok(())
    }

    async fn purge_logs_upto(
        &mut self,
        log_id: openraft::LogId<NodeId>,
    ) -> Result<(), openraft::StorageError<NodeId>> {
        let mut inner = self.inner.lock().map_err(openraft_lock_error)?;
        inner.logs.retain(|index, _| *index > log_id.index);
        inner.last_purged_log_id = Some(log_id);
        Ok(())
    }

    async fn last_applied_state(
        &mut self,
    ) -> Result<
        (
            Option<openraft::LogId<NodeId>>,
            openraft::StoredMembership<NodeId, openraft::BasicNode>,
        ),
        openraft::StorageError<NodeId>,
    > {
        let inner = self.inner.lock().map_err(openraft_lock_error)?;
        Ok((
            inner.applier.last_applied_log_id(),
            inner.applier.last_membership().clone(),
        ))
    }

    async fn apply_to_state_machine(
        &mut self,
        entries: &[openraft::Entry<BlockRaftTypeConfig>],
    ) -> Result<Vec<BlockResponse>, openraft::StorageError<NodeId>> {
        self.inner
            .lock()
            .map_err(openraft_lock_error)?
            .applier
            .apply_entries(entries.iter().cloned())
            .map_err(openraft_store_error)
    }

    async fn get_snapshot_builder(&mut self) -> Self::SnapshotBuilder {
        OpenraftBlockSnapshotBuilder {
            store: self.clone(),
        }
    }

    async fn begin_receiving_snapshot(
        &mut self,
    ) -> Result<Box<Cursor<Vec<u8>>>, openraft::StorageError<NodeId>> {
        Ok(Box::new(Cursor::new(Vec::new())))
    }

    async fn install_snapshot(
        &mut self,
        meta: &openraft::SnapshotMeta<NodeId, openraft::BasicNode>,
        snapshot: Box<Cursor<Vec<u8>>>,
    ) -> Result<(), openraft::StorageError<NodeId>> {
        let block_snapshot: BlockSnapshot =
            serde_json::from_slice(&snapshot.into_inner()).map_err(openraft_store_error)?;
        let mut inner = self.inner.lock().map_err(openraft_lock_error)?;
        inner
            .applier
            .install_snapshot(&block_snapshot)
            .map_err(openraft_store_error)?;
        inner.applier.last_applied_log_id = meta.last_log_id;
        inner.applier.last_membership = meta.last_membership.clone();
        Ok(())
    }

    async fn get_current_snapshot(
        &mut self,
    ) -> Result<Option<openraft::Snapshot<BlockRaftTypeConfig>>, openraft::StorageError<NodeId>>
    {
        if self
            .inner
            .lock()
            .map_err(openraft_lock_error)?
            .applier
            .last_applied_log_id()
            .is_none()
        {
            return Ok(None);
        }
        let mut builder = self.get_snapshot_builder().await;
        openraft::storage::RaftSnapshotBuilder::build_snapshot(&mut builder)
            .await
            .map(Some)
    }
}

impl openraft::storage::RaftSnapshotBuilder<BlockRaftTypeConfig> for OpenraftBlockSnapshotBuilder {
    async fn build_snapshot(
        &mut self,
    ) -> Result<openraft::Snapshot<BlockRaftTypeConfig>, openraft::StorageError<NodeId>> {
        let inner = self.store.inner.lock().map_err(openraft_lock_error)?;
        let block_snapshot = inner.applier.replica().snapshot();
        let encoded = serde_json::to_vec(&block_snapshot).map_err(openraft_store_error)?;
        let meta = openraft::SnapshotMeta {
            last_log_id: inner.applier.last_applied_log_id(),
            last_membership: inner.applier.last_membership().clone(),
            snapshot_id: format!(
                "{}-{}",
                inner.applier.node_id(),
                block_snapshot.last_included_index
            ),
        };
        Ok(openraft::Snapshot {
            meta,
            snapshot: Box::new(Cursor::new(encoded)),
        })
    }
}

fn range_contains<RB: RangeBounds<u64>>(range: &RB, index: u64) -> bool {
    let after_start = match range.start_bound() {
        Bound::Included(start) => index >= *start,
        Bound::Excluded(start) => index > *start,
        Bound::Unbounded => true,
    };
    let before_end = match range.end_bound() {
        Bound::Included(end) => index <= *end,
        Bound::Excluded(end) => index < *end,
        Bound::Unbounded => true,
    };
    after_start && before_end
}

fn openraft_lock_error<T>(_err: std::sync::PoisonError<T>) -> openraft::StorageError<NodeId> {
    openraft::StorageError::from_io_error(
        openraft::ErrorSubject::Store,
        openraft::ErrorVerb::Read,
        std::io::Error::other("openraft block store lock poisoned"),
    )
}

fn openraft_store_error(err: impl std::fmt::Display) -> openraft::StorageError<NodeId> {
    openraft::StorageError::from_io_error(
        openraft::ErrorSubject::Store,
        openraft::ErrorVerb::Write,
        std::io::Error::other(err.to_string()),
    )
}

#[derive(Debug, Clone)]
pub struct CommitOutcome {
    pub entry: LogEntry,
    pub acknowledgements: Vec<NodeId>,
}

#[derive(Debug, Clone)]
pub struct FakeRaftBlockCluster {
    replicas: BTreeMap<NodeId, Replica>,
    committed: Vec<LogEntry>,
    next_index: LogIndex,
    current_term: Term,
    leader_id: NodeId,
    compacted_through: LogIndex,
}

impl FakeRaftBlockCluster {
    pub fn new(
        node_ids: impl IntoIterator<Item = NodeId>,
        capacity_bytes: u64,
        block_size: u64,
    ) -> Result<Self, RaftBlockError> {
        let mut replicas = BTreeMap::new();
        for id in node_ids {
            replicas.insert(id, Replica::new(id, capacity_bytes, block_size)?);
        }
        Ok(Self {
            replicas,
            committed: Vec::new(),
            next_index: 1,
            current_term: 1,
            leader_id: 1,
            compacted_through: 0,
        })
    }

    pub fn quorum(&self) -> usize {
        (self.replicas.len() / 2) + 1
    }

    pub fn committed_entries(&self) -> &[LogEntry] {
        &self.committed
    }

    pub fn compacted_through(&self) -> LogIndex {
        self.compacted_through
    }

    pub fn replica(&self, id: NodeId) -> Result<&Replica, RaftBlockError> {
        self.replicas
            .get(&id)
            .ok_or(RaftBlockError::NodeNotFound(id))
    }

    pub fn replica_mut(&mut self, id: NodeId) -> Result<&mut Replica, RaftBlockError> {
        self.replicas
            .get_mut(&id)
            .ok_or(RaftBlockError::NodeNotFound(id))
    }

    pub fn propose_write(
        &mut self,
        offset: u64,
        bytes: Vec<u8>,
        reachable: &[NodeId],
    ) -> Result<CommitOutcome, RaftBlockError> {
        self.propose_write_from(self.leader_id, offset, bytes, reachable)
    }

    pub fn propose_write_from(
        &mut self,
        proposer: NodeId,
        offset: u64,
        bytes: Vec<u8>,
        reachable: &[NodeId],
    ) -> Result<CommitOutcome, RaftBlockError> {
        self.ensure_leader(proposer)?;
        let entry = LogEntry::write(self.current_term, self.next_index, offset, bytes)?;
        self.commit_entry(entry, reachable)
    }

    pub fn propose_flush(&mut self, reachable: &[NodeId]) -> Result<CommitOutcome, RaftBlockError> {
        self.propose_flush_from(self.leader_id, reachable)
    }

    pub fn propose_flush_from(
        &mut self,
        proposer: NodeId,
        reachable: &[NodeId],
    ) -> Result<CommitOutcome, RaftBlockError> {
        self.ensure_leader(proposer)?;
        let entry = LogEntry::flush(self.current_term, self.next_index);
        self.commit_entry(entry, reachable)
    }

    pub fn repair_node(&mut self, node_id: NodeId) -> Result<usize, RaftBlockError> {
        let entries = self.committed.clone();
        let replica = self.replica_mut(node_id)?;
        let mut applied = 0;
        for entry in &entries {
            if replica.apply(entry)? {
                applied += 1;
            }
        }
        Ok(applied)
    }

    pub fn read_from(
        &self,
        node_id: NodeId,
        offset: u64,
        len: usize,
    ) -> Result<Vec<u8>, RaftBlockError> {
        if node_id != self.leader_id {
            return Err(RaftBlockError::NotLeader {
                node_id,
                leader_id: self.leader_id,
            });
        }
        let replica = self.replica(node_id)?;
        let end = offset
            .checked_add(len as u64)
            .ok_or(RaftBlockError::OutOfBounds)?;
        if end > replica.read_all().len() as u64 {
            return Err(RaftBlockError::OutOfBounds);
        }
        Ok(replica.read_all()[offset as usize..end as usize].to_vec())
    }

    pub fn compact_through(&mut self, index: LogIndex) -> Result<BlockSnapshot, RaftBlockError> {
        let leader = self.replica(self.leader_id)?;
        let snapshot = leader.snapshot(index);
        self.committed.retain(|entry| entry.index > index);
        self.compacted_through = self.compacted_through.max(index);
        Ok(snapshot)
    }

    pub fn advance_term(&mut self) -> Term {
        self.current_term += 1;
        self.leader_id = self
            .replicas
            .keys()
            .copied()
            .find(|id| *id != self.leader_id)
            .unwrap_or(self.leader_id);
        self.current_term
    }

    fn ensure_leader(&self, node_id: NodeId) -> Result<(), RaftBlockError> {
        if node_id == self.leader_id {
            Ok(())
        } else {
            Err(RaftBlockError::NotLeader {
                node_id,
                leader_id: self.leader_id,
            })
        }
    }

    fn commit_entry(
        &mut self,
        entry: LogEntry,
        reachable: &[NodeId],
    ) -> Result<CommitOutcome, RaftBlockError> {
        let acknowledgements = reachable.iter().copied().collect::<BTreeSet<_>>();
        let quorum = self.quorum();
        if acknowledgements.len() < quorum {
            return Err(RaftBlockError::NoQuorum {
                acks: acknowledgements.len(),
                quorum,
            });
        }

        for id in &acknowledgements {
            let replica = self.replica(*id)?;
            replica.validate_entry(&entry)?;
        }

        for id in &acknowledgements {
            let replica = self.replica_mut(*id)?;
            replica.apply(&entry)?;
        }

        self.committed.push(entry.clone());
        self.next_index += 1;
        Ok(CommitOutcome {
            entry,
            acknowledgements: acknowledgements.into_iter().collect(),
        })
    }
}

fn validate_write(
    block_size: u64,
    capacity_bytes: u64,
    offset: u64,
    bytes: &[u8],
) -> Result<(), RaftBlockError> {
    if bytes.is_empty() {
        return Err(RaftBlockError::EmptyWrite);
    }
    if !offset.is_multiple_of(block_size) || !(bytes.len() as u64).is_multiple_of(block_size) {
        return Err(RaftBlockError::UnalignedWrite);
    }
    let end = offset
        .checked_add(bytes.len() as u64)
        .ok_or(RaftBlockError::OutOfBounds)?;
    if end > capacity_bytes {
        return Err(RaftBlockError::OutOfBounds);
    }
    Ok(())
}

fn checksum_bytes(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn cluster3() -> FakeRaftBlockCluster {
        FakeRaftBlockCluster::new([1, 2, 3], 4096, 512).unwrap()
    }

    #[test]
    fn quorum_write_applies_in_order_to_reachable_majority() {
        let mut cluster = cluster3();
        cluster.propose_write(0, vec![1; 512], &[1, 2]).unwrap();
        cluster.propose_write(512, vec![2; 512], &[1, 2]).unwrap();

        let replica = cluster.replica(1).unwrap();
        assert_eq!(&replica.read_all()[0..512], &[1; 512]);
        assert_eq!(&replica.read_all()[512..1024], &[2; 512]);
        assert_eq!(cluster.committed_entries().len(), 2);
    }

    #[test]
    fn minority_partition_cannot_commit() {
        let mut cluster = cluster3();
        let err = cluster.propose_write(0, vec![1; 512], &[1]).unwrap_err();
        assert_eq!(err, RaftBlockError::NoQuorum { acks: 1, quorum: 2 });
        assert!(cluster.committed_entries().is_empty());
        assert_eq!(cluster.replica(1).unwrap().read_all(), &[0; 4096]);
    }

    #[test]
    fn duplicate_acknowledgements_do_not_form_quorum() {
        let mut cluster = cluster3();
        let err = cluster.propose_write(0, vec![1; 512], &[1, 1]).unwrap_err();
        assert_eq!(err, RaftBlockError::NoQuorum { acks: 1, quorum: 2 });
        assert!(cluster.committed_entries().is_empty());
        assert_eq!(cluster.replica(1).unwrap().read_all(), &[0; 4096]);
    }

    #[test]
    fn replay_is_idempotent() {
        let mut replica = Replica::new(1, 4096, 512).unwrap();
        let entry = LogEntry::write(1, 1, 0, vec![7; 512]).unwrap();
        assert!(replica.apply(&entry).unwrap());
        assert!(!replica.apply(&entry).unwrap());
        assert_eq!(&replica.read_all()[0..512], &[7; 512]);
    }

    #[test]
    fn stale_leader_entry_is_rejected_after_newer_term_seen() {
        let mut replica = Replica::new(1, 4096, 512).unwrap();
        replica.observe_term(3);
        let entry = LogEntry::write(2, 1, 0, vec![1; 512]).unwrap();
        let err = replica.apply(&entry).unwrap_err();
        assert_eq!(
            err,
            RaftBlockError::StaleTerm {
                entry_term: 2,
                seen_term: 3
            }
        );
    }

    #[test]
    fn repair_replays_committed_entries_to_lagging_follower() {
        let mut cluster = cluster3();
        cluster.propose_write(0, vec![1; 512], &[1, 2]).unwrap();
        cluster.propose_write(512, vec![2; 512], &[1, 2]).unwrap();
        assert_eq!(cluster.replica(3).unwrap().read_all(), &[0; 4096]);

        assert_eq!(cluster.repair_node(3).unwrap(), 2);
        assert_eq!(
            cluster.replica(3).unwrap().read_all(),
            cluster.replica(1).unwrap().read_all()
        );
    }

    #[test]
    fn checksum_mismatch_rejects_corrupt_entry_without_mutation() {
        let mut replica = Replica::new(1, 4096, 512).unwrap();
        let mut entry = LogEntry::write(1, 1, 0, vec![1; 512]).unwrap();
        let BlockOp::Write { bytes, .. } = &mut entry.op else {
            unreachable!();
        };
        bytes[0] = 9;

        let err = replica.apply(&entry).unwrap_err();
        assert_eq!(err, RaftBlockError::ChecksumMismatch);
        assert_eq!(replica.read_all(), &[0; 4096]);
    }

    #[test]
    fn out_of_bounds_write_does_not_partially_mutate() {
        let mut replica = Replica::new(1, 1024, 512).unwrap();
        let entry = LogEntry::write(1, 1, 512, vec![3; 1024]).unwrap();
        let err = replica.apply(&entry).unwrap_err();
        assert_eq!(err, RaftBlockError::OutOfBounds);
        assert_eq!(replica.read_all(), &[0; 1024]);
    }

    #[test]
    fn simulated_disk_full_rejects_without_mutation() {
        let mut replica = Replica::new(1, 4096, 512).unwrap();
        replica.fail_after_applied_entries(1);
        let first = LogEntry::write(1, 1, 0, vec![1; 512]).unwrap();
        let second = LogEntry::write(1, 2, 512, vec![2; 512]).unwrap();

        assert!(replica.apply(&first).unwrap());
        let err = replica.apply(&second).unwrap_err();
        assert_eq!(err, RaftBlockError::DiskFull);
        assert_eq!(&replica.read_all()[0..512], &[1; 512]);
        assert_eq!(&replica.read_all()[512..1024], &[0; 512]);
    }

    #[test]
    fn failed_quorum_validation_does_not_partially_mutate_prefix() {
        let mut cluster = cluster3();
        cluster.replica_mut(2).unwrap().observe_term(3);

        let err = cluster.propose_write(0, vec![1; 512], &[1, 2]).unwrap_err();
        assert_eq!(
            err,
            RaftBlockError::StaleTerm {
                entry_term: 1,
                seen_term: 3
            }
        );
        assert!(cluster.committed_entries().is_empty());
        assert_eq!(cluster.replica(1).unwrap().read_all(), &[0; 4096]);
        assert_eq!(cluster.replica(2).unwrap().read_all(), &[0; 4096]);
    }

    #[test]
    fn leader_only_reads_reject_follower_reads() {
        let mut cluster = cluster3();
        cluster.propose_write(0, vec![9; 512], &[1, 2]).unwrap();

        assert_eq!(cluster.read_from(1, 0, 512).unwrap(), vec![9; 512]);
        let err = cluster.read_from(2, 0, 512).unwrap_err();
        assert_eq!(
            err,
            RaftBlockError::NotLeader {
                node_id: 2,
                leader_id: 1
            }
        );
    }

    #[test]
    fn non_leader_proposals_are_rejected_without_mutation() {
        let mut cluster = cluster3();
        let err = cluster
            .propose_write_from(2, 0, vec![6; 512], &[1, 2])
            .unwrap_err();
        assert_eq!(
            err,
            RaftBlockError::NotLeader {
                node_id: 2,
                leader_id: 1
            }
        );
        assert!(cluster.committed_entries().is_empty());
        assert_eq!(cluster.replica(1).unwrap().read_all(), &[0; 4096]);
        assert_eq!(cluster.replica(2).unwrap().read_all(), &[0; 4096]);
    }

    #[test]
    fn old_leader_is_fenced_after_term_advance() {
        let mut cluster = cluster3();
        cluster
            .propose_write_from(1, 0, vec![1; 512], &[1, 2])
            .unwrap();
        cluster.advance_term();

        let err = cluster
            .propose_flush_from(1, &[1, 2])
            .expect_err("old leader must be fenced");
        assert_eq!(
            err,
            RaftBlockError::NotLeader {
                node_id: 1,
                leader_id: 2
            }
        );
        cluster.propose_flush_from(2, &[1, 2]).unwrap();
    }

    #[test]
    fn snapshot_install_repairs_compacted_history() {
        let mut cluster = cluster3();
        cluster.propose_write(0, vec![1; 512], &[1, 2]).unwrap();
        cluster.propose_write(512, vec![2; 512], &[1, 2]).unwrap();

        let snapshot = cluster.compact_through(2).unwrap();
        assert_eq!(cluster.compacted_through(), 2);
        assert!(cluster.committed_entries().is_empty());

        let replica = cluster.replica_mut(3).unwrap();
        replica.install_snapshot(&snapshot).unwrap();
        assert_eq!(&replica.read_all()[0..512], &[1; 512]);
        assert_eq!(&replica.read_all()[512..1024], &[2; 512]);
        assert!(replica.applied_indexes().contains(&1));
        assert!(replica.applied_indexes().contains(&2));
    }

    #[test]
    fn block_command_maps_to_log_entry_and_response() {
        let entry = BlockCommand::Write {
            offset: 0,
            bytes: vec![4; 512],
        }
        .into_entry(2, 7)
        .unwrap();

        assert_eq!(entry.term, 2);
        assert_eq!(entry.index, 7);
        let BlockOp::Write { offset, bytes, .. } = entry.op else {
            panic!("expected write");
        };
        assert_eq!(offset, 0);
        assert_eq!(bytes, vec![4; 512]);
    }

    #[test]
    fn openraft_type_config_is_pinned_and_valid() {
        assert_eq!(OPENRAFT_VERSION, "0.9.24");
        let config = default_openraft_config().unwrap();
        assert_eq!(config.cluster_name, "nqrust-raft-block");
        assert!(config.election_timeout_min < config.election_timeout_max);
    }

    #[test]
    fn openraft_entries_apply_normal_commands_to_persistent_replica() {
        let dir = tempfile::tempdir().unwrap();
        let store = FileReplicaStore::new(dir.path().join("node-1.json"));
        let mut applier = OpenraftEntryApplier::create(store.clone(), 1, 4096, 512).unwrap();

        let responses = applier
            .apply_entries([
                openraft::Entry {
                    log_id: openraft_log_id(1, 1, 1),
                    payload: openraft::EntryPayload::Blank,
                },
                openraft_entry(
                    1,
                    1,
                    2,
                    BlockCommand::Write {
                        offset: 0,
                        bytes: vec![9; 512],
                    },
                ),
                openraft_entry(1, 1, 3, BlockCommand::Flush),
            ])
            .unwrap();

        assert_eq!(responses.len(), 3);
        assert_eq!(responses[0].bytes_written, 0);
        assert_eq!(responses[1].bytes_written, 512);
        assert_eq!(responses[2].bytes_written, 0);
        assert_eq!(
            applier.last_applied_log_id(),
            Some(openraft_log_id(1, 1, 3))
        );
        assert_eq!(&applier.replica().read_all()[0..512], &[9; 512]);
        drop(applier);

        let reopened = OpenraftEntryApplier::open(store).unwrap().unwrap();
        assert_eq!(&reopened.replica().read_all()[0..512], &[9; 512]);
    }

    #[test]
    fn openraft_membership_entry_tracks_membership_without_mutating_blocks() {
        let dir = tempfile::tempdir().unwrap();
        let store = FileReplicaStore::new(dir.path().join("node-1.json"));
        let mut applier = OpenraftEntryApplier::create(store, 1, 4096, 512).unwrap();
        let membership = openraft::Membership::new(vec![BTreeSet::from([1, 2, 3])], ());

        let responses = applier
            .apply_entries([openraft::Entry {
                log_id: openraft_log_id(2, 2, 4),
                payload: openraft::EntryPayload::Membership(membership),
            }])
            .unwrap();

        assert_eq!(
            responses,
            vec![BlockResponse {
                applied_index: 4,
                bytes_written: 0
            }]
        );
        assert_eq!(
            applier.last_applied_log_id(),
            Some(openraft_log_id(2, 2, 4))
        );
        assert_eq!(
            applier.last_membership().log_id().as_ref(),
            Some(&openraft_log_id(2, 2, 4))
        );
        assert_eq!(applier.replica().read_all(), &[0; 4096]);
    }

    #[tokio::test]
    async fn openraft_storage_harness_appends_applies_and_snapshots() {
        use openraft::storage::{RaftLogReader, RaftSnapshotBuilder, RaftStorage};

        let dir = tempfile::tempdir().unwrap();
        let store_path = FileReplicaStore::new(dir.path().join("node-1.json"));
        let mut store = InMemoryOpenraftBlockStore::create(store_path, 1, 4096, 512).unwrap();
        let entry = openraft_entry(
            1,
            1,
            1,
            BlockCommand::Write {
                offset: 0,
                bytes: vec![8; 512],
            },
        );

        store.append_to_log([entry.clone()]).await.unwrap();
        assert_eq!(
            store.get_log_state().await.unwrap().last_log_id,
            Some(entry.log_id)
        );
        assert_eq!(
            store.try_get_log_entries(1..2).await.unwrap(),
            vec![entry.clone()]
        );

        let responses = store.apply_to_state_machine(&[entry]).await.unwrap();
        assert_eq!(
            responses,
            vec![BlockResponse {
                applied_index: 1,
                bytes_written: 512
            }]
        );
        assert_eq!(store.read_range(0, 512).unwrap(), vec![8; 512]);

        let snapshot = store
            .get_snapshot_builder()
            .await
            .build_snapshot()
            .await
            .unwrap();
        assert_eq!(snapshot.meta.last_log_id, Some(openraft_log_id(1, 1, 1)));
    }

    #[test]
    fn openraft_storage_harness_reopens_persistent_log_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let store_path = FileReplicaStore::new(dir.path().join("node-1.json"));
        let store =
            InMemoryOpenraftBlockStore::open_or_create(store_path.clone(), 1, 4096, 512).unwrap();
        store
            .append_command(
                1,
                1,
                BlockCommand::Write {
                    offset: 0,
                    bytes: vec![6; 512],
                },
            )
            .unwrap();
        drop(store);

        let reopened =
            InMemoryOpenraftBlockStore::open_or_create(store_path, 1, 4096, 512).unwrap();
        assert_eq!(reopened.retained_log_entries().unwrap(), 1);
        assert_eq!(reopened.last_applied_index().unwrap(), 1);
        assert_eq!(reopened.read_range(0, 512).unwrap(), vec![6; 512]);
    }

    #[test]
    fn openraft_upstream_storage_suite_accepts_store_harness() {
        type StoreAdaptor =
            openraft::storage::Adaptor<BlockRaftTypeConfig, InMemoryOpenraftBlockStore>;

        openraft::testing::Suite::<BlockRaftTypeConfig, StoreAdaptor, StoreAdaptor, _, ()>::test_all(
            || async {
                let path = tempfile::NamedTempFile::new()
                    .unwrap()
                    .into_temp_path()
                    .keep()
                    .unwrap();
                InMemoryOpenraftBlockStore::create(FileReplicaStore::new(path), 1, 4096, 512)
                    .unwrap()
            },
        )
        .unwrap();
    }

    #[test]
    fn openraft_storage_harness_rejects_conflicting_vote() {
        let dir = tempfile::tempdir().unwrap();
        let store_path = FileReplicaStore::new(dir.path().join("node-1.json"));
        let store = InMemoryOpenraftBlockStore::create(store_path, 1, 4096, 512).unwrap();

        assert_eq!(
            store.request_vote(2, 2).unwrap(),
            VoteOutcome {
                granted: true,
                term: 2,
                voted_for: Some(2),
                committed: false,
            }
        );
        assert_eq!(
            store.request_vote(2, 3).unwrap(),
            VoteOutcome {
                granted: false,
                term: 2,
                voted_for: Some(2),
                committed: false,
            }
        );
        assert_eq!(
            store.request_vote(3, 3).unwrap(),
            VoteOutcome {
                granted: true,
                term: 3,
                voted_for: Some(3),
                committed: false,
            }
        );
    }

    #[test]
    fn persistent_replica_reopens_with_applied_bytes_and_log() {
        let dir = tempfile::tempdir().unwrap();
        let store = FileReplicaStore::new(dir.path().join("node-1.json"));
        let mut replica = PersistentReplica::create(store.clone(), 1, 4096, 512).unwrap();

        let response = replica
            .append_command(
                1,
                BlockCommand::Write {
                    offset: 0,
                    bytes: vec![8; 512],
                },
            )
            .unwrap();
        assert_eq!(
            response,
            BlockResponse {
                applied_index: 1,
                bytes_written: 512
            }
        );
        drop(replica);

        let reopened = PersistentReplica::open(store).unwrap().unwrap();
        assert_eq!(&reopened.read_all()[0..512], &[8; 512]);
        assert_eq!(reopened.log().len(), 1);
        assert_eq!(reopened.log()[0].index, 1);
        assert_eq!(reopened.read_range(0, 512).unwrap(), vec![8; 512]);
    }

    #[test]
    fn file_store_uses_sidecar_blocks_and_append_log() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("node-1.json");
        let store = FileReplicaStore::new(&path);
        let mut replica = PersistentReplica::create(store.clone(), 1, 4096, 512).unwrap();

        replica
            .append_command(
                1,
                BlockCommand::Write {
                    offset: 0,
                    bytes: vec![3; 512],
                },
            )
            .unwrap();
        replica
            .append_command(
                1,
                BlockCommand::Write {
                    offset: 512,
                    bytes: vec![4; 512],
                },
            )
            .unwrap();
        drop(replica);

        let sidecar = sidecar_paths(&path);
        assert!(sidecar.meta.exists());
        assert!(sidecar.blocks.exists());
        assert!(sidecar.log.exists());
        assert!(
            !path.exists(),
            "new writes should not use legacy monolithic JSON"
        );

        let reopened = PersistentReplica::open(store).unwrap().unwrap();
        assert_eq!(reopened.log().len(), 2);
        assert_eq!(reopened.read_range(0, 512).unwrap(), vec![3; 512]);
        assert_eq!(reopened.read_range(512, 512).unwrap(), vec![4; 512]);
    }

    #[test]
    fn persistent_replica_read_range_checks_bounds() {
        let dir = tempfile::tempdir().unwrap();
        let store = FileReplicaStore::new(dir.path().join("node-1.json"));
        let replica = PersistentReplica::create(store, 1, 1024, 512).unwrap();
        let err = replica.read_range(512, 1024).unwrap_err();
        assert_eq!(err, RaftBlockError::OutOfBounds);
    }

    #[test]
    fn persistent_replica_install_snapshot_compacts_replayed_log() {
        let dir = tempfile::tempdir().unwrap();
        let store = FileReplicaStore::new(dir.path().join("node-1.json"));
        let mut replica = PersistentReplica::create(store.clone(), 1, 4096, 512).unwrap();
        replica
            .append_command(
                1,
                BlockCommand::Write {
                    offset: 0,
                    bytes: vec![1; 512],
                },
            )
            .unwrap();
        replica
            .append_command(
                1,
                BlockCommand::Write {
                    offset: 512,
                    bytes: vec![2; 512],
                },
            )
            .unwrap();

        let snapshot = replica.snapshot();
        replica.install_snapshot(&snapshot).unwrap();
        assert!(replica.log().is_empty());
        drop(replica);

        let reopened = PersistentReplica::open(store).unwrap().unwrap();
        assert_eq!(&reopened.read_all()[0..512], &[1; 512]);
        assert_eq!(&reopened.read_all()[512..1024], &[2; 512]);
        assert!(reopened.log().is_empty());
    }

    proptest! {
        #[test]
        fn aligned_quorum_writes_are_replayable(
            first in any::<u8>(),
            second in any::<u8>(),
            first_block in 0usize..4,
            second_block in 0usize..4,
        ) {
            let mut cluster = cluster3();
            cluster
                .propose_write((first_block * 512) as u64, vec![first; 512], &[1, 2])
                .unwrap();
            cluster
                .propose_write((second_block * 512) as u64, vec![second; 512], &[1, 2])
                .unwrap();

            cluster.repair_node(3).unwrap();
            prop_assert_eq!(
                cluster.replica(1).unwrap().read_all(),
                cluster.replica(3).unwrap().read_all()
            );
        }
    }
}
