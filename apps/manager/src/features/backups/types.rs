//! RPC request/response types between manager and agent for backup ops.

use nexus_storage::{AttachedPath, BackendKind, VolumeHandle, VolumeSnapshotHandle};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BackupTargetConfig {
    pub endpoint: String,
    pub region: Option<String>,
    pub bucket: String,
    pub prefix: String,
    pub access_key_id: String,
    pub secret_access_key: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct ChunkerParams {
    pub min_size: u32,
    pub avg_size: u32,
    pub max_size: u32,
}

impl Default for ChunkerParams {
    fn default() -> Self {
        Self {
            min_size: 4096,
            avg_size: 65536,
            max_size: 1048576,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BackupReq {
    pub backup_id: Uuid,
    pub snapshot: VolumeSnapshotHandle,
    pub backend_kind: BackendKind,
    pub target: BackupTargetConfig,
    pub encryption_key: [u8; 32],
    pub chunker_params: ChunkerParams,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BackupResp {
    pub manifest_object_key: String,
    pub chunk_count: u64,
    pub bytes_written: u64,
    pub bytes_unique: u64,
    pub duration_ms: u64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RestoreReq {
    pub target_volume: VolumeHandle,
    pub target_attached: AttachedPath,
    pub manifest_object_key: String,
    pub target: BackupTargetConfig,
    pub encryption_key: [u8; 32],
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RestoreResp {
    pub bytes_written: u64,
    pub duration_ms: u64,
}
