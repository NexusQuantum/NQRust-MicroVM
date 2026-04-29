//! Correctness prototype for B-II replicated block semantics.
//!
//! This crate intentionally does not expose a production storage backend. It is
//! a small deterministic model for log entries, quorum commit, idempotent replay,
//! and repair. The production Raft/SPDK backend should be built only after this
//! model grows enough failure coverage to catch ordering, replay, and stale
//! leader bugs.

use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

pub type NodeId = u64;
pub type LogIndex = u64;
pub type Term = u64;

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
    #[error("entry checksum mismatch")]
    ChecksumMismatch,
    #[error("entry term {entry_term} is stale; node has seen term {seen_term}")]
    StaleTerm { entry_term: Term, seen_term: Term },
    #[error("not enough acknowledgements for quorum: {acks}/{quorum}")]
    NoQuorum { acks: usize, quorum: usize },
    #[error("node {0} not found")]
    NodeNotFound(NodeId),
}

#[derive(Debug, Clone, PartialEq, Eq)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
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

        if let BlockOp::Write { offset, bytes, .. } = &entry.op {
            let start = *offset as usize;
            let end = start + bytes.len();
            self.bytes[start..end].copy_from_slice(bytes);
        }

        self.applied.insert(entry.index);
        Ok(true)
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
        })
    }

    pub fn quorum(&self) -> usize {
        (self.replicas.len() / 2) + 1
    }

    pub fn committed_entries(&self) -> &[LogEntry] {
        &self.committed
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

    pub fn advance_term(&mut self) -> Term {
        self.current_term += 1;
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
