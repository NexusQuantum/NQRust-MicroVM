use crate::features::storage::LocalStorage;
use nexus_storage::{
    BackendInstanceId, BackendKind, Capabilities, ControlPlaneBackend, CreateOpts, StorageError,
    VolumeHandle, VolumeSnapshotHandle,
};
use std::path::Path;
use std::path::PathBuf;
use uuid::Uuid;

/// Manager-side LocalFile backend. Wraps the existing `LocalStorage` helper
/// so behavior is byte-for-byte identical to pre-foundation code.
pub struct LocalFileControlPlaneBackend {
    pub id: BackendInstanceId,
}

impl LocalFileControlPlaneBackend {
    fn storage(&self) -> LocalStorage {
        // LocalStorage::new() reads MANAGER_STORAGE_ROOT each time.
        LocalStorage::new()
    }

    fn root_for(&self, vol_id: Uuid) -> PathBuf {
        self.storage().vm_dir(vol_id).join("storage")
    }
}

#[async_trait::async_trait]
impl ControlPlaneBackend for LocalFileControlPlaneBackend {
    fn kind(&self) -> BackendKind {
        BackendKind::LocalFile
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            supports_native_snapshots: false,
            supports_concurrent_attach: false,
            supports_live_migration: false,
            supports_clone_from_image: true,
        }
    }

    async fn provision(&self, opts: CreateOpts) -> Result<VolumeHandle, StorageError> {
        let vol_id = Uuid::new_v4();
        let dir = self.root_for(vol_id);
        tokio::fs::create_dir_all(&dir).await?;
        let path = dir.join(format!("disk-{}.img", vol_id));
        let f = tokio::fs::File::create(&path).await?;
        f.set_len(opts.size_bytes).await?;
        Ok(VolumeHandle {
            volume_id: vol_id,
            backend_id: self.id,
            backend_kind: BackendKind::LocalFile,
            locator: path.display().to_string(),
            size_bytes: opts.size_bytes,
        })
    }

    async fn destroy(&self, handle: VolumeHandle) -> Result<(), StorageError> {
        let p = Path::new(&handle.locator);
        if p.exists() {
            tokio::fs::remove_file(p).await?;
        }
        Ok(())
    }

    async fn clone_from_image(
        &self,
        source_image: &Path,
        opts: CreateOpts,
    ) -> Result<VolumeHandle, StorageError> {
        let vol_id = Uuid::new_v4();
        let dir = self.root_for(vol_id);
        tokio::fs::create_dir_all(&dir).await?;

        let ext = source_image
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| format!(".{s}"))
            .unwrap_or_default();
        let dst = dir.join(format!("rootfs-{vol_id}{ext}"));

        tokio::fs::copy(source_image, &dst).await?;

        // Match historical behavior: extend file to requested size if larger
        // than the source. resize2fs on the inner ext4 filesystem is the
        // caller's job (rootfs_allocator), not ours.
        let source_size = tokio::fs::metadata(&dst).await?.len();
        let final_size = if opts.size_bytes > source_size {
            let f = tokio::fs::OpenOptions::new().write(true).open(&dst).await?;
            f.set_len(opts.size_bytes).await?;
            opts.size_bytes
        } else {
            source_size
        };

        Ok(VolumeHandle {
            volume_id: vol_id,
            backend_id: self.id,
            backend_kind: BackendKind::LocalFile,
            locator: dst.display().to_string(),
            size_bytes: final_size,
        })
    }

    async fn snapshot(
        &self,
        volume: &VolumeHandle,
        name: &str,
    ) -> Result<VolumeSnapshotHandle, StorageError> {
        // Slow but correct: byte-copy the file. Native snapshot capability is
        // false for LocalFile so callers expect this to be slow.
        let snap_id = Uuid::new_v4();
        let src = Path::new(&volume.locator);
        let parent = src
            .parent()
            .ok_or_else(|| StorageError::InvalidLocator(volume.locator.clone()))?;
        let dst = parent.join(format!("snap-{snap_id}-{name}.img"));
        tokio::fs::copy(src, &dst).await?;
        Ok(VolumeSnapshotHandle {
            snapshot_id: snap_id,
            source_volume_id: volume.volume_id,
            backend_id: self.id,
            backend_kind: BackendKind::LocalFile,
            locator: dst.display().to_string(),
        })
    }

    async fn clone_from_snapshot(
        &self,
        snap: &VolumeSnapshotHandle,
    ) -> Result<VolumeHandle, StorageError> {
        let vol_id = Uuid::new_v4();
        let dir = self.root_for(vol_id);
        tokio::fs::create_dir_all(&dir).await?;
        let dst = dir.join(format!("disk-{vol_id}.img"));
        tokio::fs::copy(&snap.locator, &dst).await?;
        let size = tokio::fs::metadata(&dst).await?.len();
        Ok(VolumeHandle {
            volume_id: vol_id,
            backend_id: self.id,
            backend_kind: BackendKind::LocalFile,
            locator: dst.display().to_string(),
            size_bytes: size,
        })
    }

    async fn delete_snapshot(&self, snap: VolumeSnapshotHandle) -> Result<(), StorageError> {
        let p = Path::new(&snap.locator);
        if p.exists() {
            tokio::fs::remove_file(p).await?;
        }
        Ok(())
    }
}
