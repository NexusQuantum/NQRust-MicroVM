use crate::error::StorageError;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

pub const RAFT_SPDK_DEFAULT_BLOCK_SIZE: u64 = 512;
pub const RAFT_SPDK_STATIC_REPLICA_COUNT: usize = 3;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RaftSpdkReplicaLocator {
    pub node_id: u64,
    pub agent_base_url: String,
    pub spdk_lvol_locator: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RaftSpdkLocator {
    pub group_id: Uuid,
    pub size_bytes: u64,
    pub block_size: u64,
    pub replicas: Vec<RaftSpdkReplicaLocator>,
    pub leader_hint: Option<u64>,
}

impl RaftSpdkLocator {
    pub fn new(
        group_id: Uuid,
        size_bytes: u64,
        block_size: u64,
        replicas: Vec<RaftSpdkReplicaLocator>,
        leader_hint: Option<u64>,
    ) -> Result<Self, StorageError> {
        if block_size == 0 {
            return Err(StorageError::InvalidLocator(
                "raft_spdk block_size must be nonzero".into(),
            ));
        }
        if size_bytes == 0 || !size_bytes.is_multiple_of(block_size) {
            return Err(StorageError::InvalidLocator(
                "raft_spdk size_bytes must be a nonzero multiple of block_size".into(),
            ));
        }
        if replicas.len() != RAFT_SPDK_STATIC_REPLICA_COUNT {
            return Err(StorageError::InvalidLocator(format!(
                "raft_spdk requires exactly {RAFT_SPDK_STATIC_REPLICA_COUNT} static replicas"
            )));
        }
        let mut node_ids = std::collections::BTreeSet::new();
        for replica in &replicas {
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
            if replica.spdk_lvol_locator.trim().is_empty() {
                return Err(StorageError::InvalidLocator(
                    "raft_spdk replica spdk_lvol_locator must not be empty".into(),
                ));
            }
        }
        if let Some(leader) = leader_hint {
            if !node_ids.contains(&leader) {
                return Err(StorageError::InvalidLocator(
                    "raft_spdk leader_hint must reference a replica node_id".into(),
                ));
            }
        }
        Ok(Self {
            group_id,
            size_bytes,
            block_size,
            replicas,
            leader_hint,
        })
    }

    pub fn to_locator_string(&self) -> Result<String, StorageError> {
        serde_json::to_string(self).map_err(StorageError::backend)
    }

    pub fn from_locator_str(s: &str) -> Result<Self, StorageError> {
        let parsed: Self =
            serde_json::from_str(s).map_err(|e| StorageError::InvalidLocator(e.to_string()))?;
        Self::new(
            parsed.group_id,
            parsed.size_bytes,
            parsed.block_size,
            parsed.replicas,
            parsed.leader_hint,
        )
    }
}

pub fn raftblk_socket_path(socket_dir: impl Into<PathBuf>, group_id: Uuid) -> PathBuf {
    socket_dir
        .into()
        .join(format!("nq-raftblk-{}.sock", group_id.simple()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn replica(node_id: u64) -> RaftSpdkReplicaLocator {
        RaftSpdkReplicaLocator {
            node_id,
            agent_base_url: format!("http://agent-{node_id}:19090"),
            spdk_lvol_locator: format!("{{\"lvol_uuid\":\"{node_id}\"}}"),
        }
    }

    #[test]
    fn locator_round_trips_and_validates_static_membership() {
        let locator = RaftSpdkLocator::new(
            Uuid::parse_str("018f64ba-97aa-70d9-a7d2-6459256fd111").unwrap(),
            4096,
            512,
            vec![replica(1), replica(2), replica(3)],
            Some(1),
        )
        .unwrap();

        let encoded = locator.to_locator_string().unwrap();
        assert_eq!(
            RaftSpdkLocator::from_locator_str(&encoded).unwrap(),
            locator
        );
    }

    #[test]
    fn locator_rejects_non_three_node_replica_sets() {
        let err = RaftSpdkLocator::new(
            Uuid::new_v4(),
            4096,
            512,
            vec![replica(1), replica(2)],
            Some(1),
        )
        .unwrap_err();
        assert!(err.to_string().contains("exactly 3"));
    }

    #[test]
    fn socket_path_is_stable_and_group_scoped() {
        let group_id = Uuid::parse_str("018f64ba-97aa-70d9-a7d2-6459256fd111").unwrap();
        assert_eq!(
            raftblk_socket_path("/run/nqrust/raftblk", group_id),
            PathBuf::from("/run/nqrust/raftblk/nq-raftblk-018f64ba97aa70d9a7d26459256fd111.sock")
        );
    }
}
