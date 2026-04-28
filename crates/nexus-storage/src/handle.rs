use crate::types::{BackendInstanceId, BackendKind};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Reference to a provisioned volume. Carries enough information that a
/// `HostBackend` can attach it without re-consulting the control plane.
/// The `locator` field is backend-defined (LocalFile: file path; Iscsi: IQN+LUN).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeHandle {
    pub volume_id: Uuid,
    pub backend_id: BackendInstanceId,
    pub backend_kind: BackendKind,
    pub locator: String,
    pub size_bytes: u64,
}

/// Reference to a snapshot. Same shape as `VolumeHandle` for symmetry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeSnapshotHandle {
    pub snapshot_id: Uuid,
    pub source_volume_id: Uuid,
    pub backend_id: BackendInstanceId,
    pub backend_kind: BackendKind,
    pub locator: String,
}

/// What the host backend hands back from `attach`. Firecracker treats `File`
/// and `BlockDevice` interchangeably (both are valid paths on its drive
/// config); `VhostUserSock` is reserved for future SPDK and not used in this
/// PR but defined now so the trait shape is stable.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "path")]
pub enum AttachedPath {
    File(PathBuf),
    BlockDevice(PathBuf),
    VhostUserSock(PathBuf),
}

impl AttachedPath {
    /// Path the caller hands to Firecracker / `resize2fs` / etc. All variants
    /// resolve to a string filesystem path; the variant only documents the
    /// nature of what's behind it.
    pub fn path(&self) -> &Path {
        match self {
            AttachedPath::File(p) => p,
            AttachedPath::BlockDevice(p) => p,
            AttachedPath::VhostUserSock(p) => p,
        }
    }
}
