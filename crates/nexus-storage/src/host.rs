use crate::error::StorageError;
use crate::handle::{AttachedPath, VolumeHandle, VolumeSnapshotHandle};
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

    /// Filesystem-aware ext4 growth hook used only by rootfs allocation after
    /// the caller has already identified the source image as ext4.
    async fn resize2fs(&self, attached: &AttachedPath) -> Result<(), StorageError>;

    /// Open a snapshot for reading. Returns a stream of bytes representing
    /// the volume contents at snapshot time. Used by the backup pipeline.
    ///
    /// Implementations:
    /// - LocalFile: open the snapshot file from disk.
    /// - Iscsi/TrueNasIscsi: attach the snapshot LUN read-only and return
    ///   a File handle over the block device.
    ///
    /// Returns `StorageError::NotSupported("read_snapshot")` if the backend
    /// can't expose a snapshot for streaming reads.
    async fn read_snapshot(
        &self,
        snap: &VolumeSnapshotHandle,
    ) -> Result<Box<dyn tokio::io::AsyncRead + Send + Unpin>, StorageError>;
}
