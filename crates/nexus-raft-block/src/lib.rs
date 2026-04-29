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
use std::io::Cursor;
use std::io::{Read, Write};
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
        let entry = LogEntry::write(self.current_term, self.next_index, offset, bytes)?;
        self.commit_entry(entry, reachable)
    }

    pub fn propose_flush(&mut self, reachable: &[NodeId]) -> Result<CommitOutcome, RaftBlockError> {
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
