// Stub implementation. Task 14 replaces this with the real LocalFile control-plane backend
// that delegates to LocalStorage for file operations.
use nexus_storage::{
    BackendInstanceId, BackendKind, Capabilities, ControlPlaneBackend, CreateOpts, StorageError,
    VolumeHandle, VolumeSnapshotHandle,
};
use std::path::Path;

/// Task 14 replaces this stub and wires it into Task 15 AppState.
/// Suppressing dead-code until that task constructs this.
#[allow(dead_code)]
pub struct LocalFileControlPlaneBackend {
    pub id: BackendInstanceId,
}

#[async_trait::async_trait]
impl ControlPlaneBackend for LocalFileControlPlaneBackend {
    fn kind(&self) -> BackendKind { BackendKind::LocalFile }
    fn capabilities(&self) -> Capabilities {
        Capabilities { supports_clone_from_image: true, ..Default::default() }
    }
    async fn provision(&self, _opts: CreateOpts) -> Result<VolumeHandle, StorageError> {
        Err(StorageError::NotSupported("provision not yet implemented".into()))
    }
    async fn destroy(&self, _h: VolumeHandle) -> Result<(), StorageError> { Ok(()) }
    async fn clone_from_image(&self, _src: &Path, _opts: CreateOpts) -> Result<VolumeHandle, StorageError> {
        Err(StorageError::NotSupported("clone_from_image not yet implemented".into()))
    }
    async fn snapshot(&self, _v: &VolumeHandle, _name: &str) -> Result<VolumeSnapshotHandle, StorageError> {
        Err(StorageError::NotSupported("snapshot".into()))
    }
    async fn clone_from_snapshot(&self, _s: &VolumeSnapshotHandle) -> Result<VolumeHandle, StorageError> {
        Err(StorageError::NotSupported("clone_from_snapshot".into()))
    }
    async fn delete_snapshot(&self, _s: VolumeSnapshotHandle) -> Result<(), StorageError> { Ok(()) }
}
