use crate::error::StorageError;
use crate::handle::{VolumeHandle, VolumeSnapshotHandle};
use crate::types::{BackendKind, Capabilities, CreateOpts};
use async_trait::async_trait;
use std::path::Path;

/// Manager-side operations on a storage backend: provisioning lifecycle and
/// snapshot lifecycle. Lives in the manager binary; never called from the
/// agent. Implementations are stored as `Arc<dyn ControlPlaneBackend>` in the
/// `Registry`.
#[async_trait]
pub trait ControlPlaneBackend: Send + Sync {
    fn kind(&self) -> BackendKind;
    fn capabilities(&self) -> Capabilities;

    async fn provision(&self, opts: CreateOpts) -> Result<VolumeHandle, StorageError>;
    async fn destroy(&self, handle: VolumeHandle) -> Result<(), StorageError>;

    /// Fast path. Only valid to call when `capabilities().supports_clone_from_image`.
    /// Implementations that don't support this MUST return
    /// `Err(StorageError::NotSupported("clone_from_image".into()))`.
    async fn clone_from_image(
        &self,
        source_image: &Path,
        opts: CreateOpts,
    ) -> Result<VolumeHandle, StorageError>;

    async fn snapshot(
        &self,
        volume: &VolumeHandle,
        name: &str,
    ) -> Result<VolumeSnapshotHandle, StorageError>;

    /// Always creates a NEW volume. Never mutates the source volume.
    /// (See spec for the rollback-vs-clone distinction.)
    async fn clone_from_snapshot(
        &self,
        snap: &VolumeSnapshotHandle,
    ) -> Result<VolumeHandle, StorageError>;

    async fn delete_snapshot(&self, snap: VolumeSnapshotHandle) -> Result<(), StorageError>;
}
