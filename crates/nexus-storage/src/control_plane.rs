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

    /// Eager reachability check. Backends with non-trivial setup
    /// (NFS mount, iSCSI login, REST auth) override this so operators
    /// see failures at create-time / startup rather than first-provision.
    /// Default is no-op for stateless backends like local_file.
    async fn probe(&self) -> Result<(), StorageError> {
        Ok(())
    }

    /// Resolve a `VolumeHandle` to a real filesystem path on the host
    /// for callers that need to open/mount/loopback the file (credential
    /// injection, guest-agent installation, Firecracker drive attach).
    ///
    /// Default: treat the locator as a path. Backends with structured
    /// locators (NFS = JSON of {server, export, file}) override this
    /// to combine the locator with their config (mount_base) and
    /// produce a path like `/var/lib/nqrust/nfs/<key>/<file>`.
    ///
    /// Returns `None` if the backend exposes no host-visible path
    /// (e.g. a vhost-user device handed straight to Firecracker).
    fn host_path_for(&self, handle: &VolumeHandle) -> Option<std::path::PathBuf> {
        Some(std::path::PathBuf::from(&handle.locator))
    }
}
