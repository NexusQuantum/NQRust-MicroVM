use crate::error::StorageError;
use crate::handle::{AttachedPath, VolumeHandle};
use crate::types::BackendKind;
use async_trait::async_trait;
use std::path::Path;

/// Agent-side operations: making volume bytes accessible to Firecracker on
/// this host. Lives in the agent binary. The manager never imports these
/// impls; it asks the agent to perform an operation via the existing
/// manager→agent HTTP API.
#[async_trait]
pub trait HostBackend: Send + Sync {
    fn kind(&self) -> BackendKind;

    async fn attach(&self, volume: &VolumeHandle) -> Result<AttachedPath, StorageError>;
    async fn detach(
        &self,
        volume: &VolumeHandle,
        attached: AttachedPath,
    ) -> Result<(), StorageError>;

    /// Pure byte copy: open the AttachedPath, write `source` bytes into it,
    /// ensure the underlying storage is at least `target_size_bytes` (sparse
    /// extension OK).
    ///
    /// MUST NOT do filesystem-aware operations (no `resize2fs`, `e2fsck`,
    /// `mkfs`). Filesystem-aware steps belong in the rootfs-allocation
    /// caller, not the trait — the trait remains agnostic to ext4/xfs/btrfs/
    /// qcow2/raw-without-fs.
    async fn populate_streaming(
        &self,
        attached: &AttachedPath,
        source: &Path,
        target_size_bytes: u64,
    ) -> Result<(), StorageError>;
}
