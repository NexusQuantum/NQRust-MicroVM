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
use std::io::{Read, Write};
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
    let config = openraft::Config {
        cluster_name: "nqrust-raft-block".into(),
        heartbeat_interval: 100,
        election_timeout_min: 500,
        election_timeout_max: 1000,
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

#[derive(Debug, Clone)]
pub struct FileReplicaStore {
    path: PathBuf,
}

impl FileReplicaStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn load(&self) -> Result<Option<PersistentReplicaState>, RaftBlockError> {
        if !self.path.exists() {
            return Ok(None);
        }
        let mut file = std::fs::File::open(&self.path)
            .map_err(|e| RaftBlockError::Store(format!("open {:?}: {e}", self.path)))?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)
            .map_err(|e| RaftBlockError::Store(format!("read {:?}: {e}", self.path)))?;
        serde_json::from_slice(&bytes)
            .map(Some)
            .map_err(|e| RaftBlockError::Store(format!("decode {:?}: {e}", self.path)))
    }

    pub fn save(&self, state: &PersistentReplicaState) -> Result<(), RaftBlockError> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| RaftBlockError::Store(format!("create {parent:?}: {e}")))?;
        }
        let tmp_path = tmp_path_for(&self.path);
        let encoded = serde_json::to_vec(state)
            .map_err(|e| RaftBlockError::Store(format!("encode {:?}: {e}", self.path)))?;
        {
            let mut file = std::fs::File::create(&tmp_path)
                .map_err(|e| RaftBlockError::Store(format!("create {tmp_path:?}: {e}")))?;
            file.write_all(&encoded)
                .map_err(|e| RaftBlockError::Store(format!("write {tmp_path:?}: {e}")))?;
            file.sync_all()
                .map_err(|e| RaftBlockError::Store(format!("sync {tmp_path:?}: {e}")))?;
        }
        std::fs::rename(&tmp_path, &self.path)
            .map_err(|e| RaftBlockError::Store(format!("rename {tmp_path:?}: {e}")))?;
        Ok(())
    }
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
            .map(|entry| openraft_log_id(entry.term, replica.node_id(), entry.index));
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
        let logs = applier
            .replica()
            .log()
            .iter()
            .map(|entry| (entry.index, block_log_entry_to_openraft(entry, node_id)))
            .collect();
        Ok(Self {
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
        })
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
        inner.applier.last_membership = meta.last_membership.clone();
        Ok(())
    }

    async fn get_current_snapshot(
        &mut self,
    ) -> Result<Option<openraft::Snapshot<BlockRaftTypeConfig>>, openraft::StorageError<NodeId>>
    {
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
