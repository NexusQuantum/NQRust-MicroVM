use nexus_storage::{AttachedPath, BackendKind, HostBackend, StorageError, VolumeHandle};
use std::path::{Path, PathBuf};

/// Agent-side LocalFile backend. Trivial: the locator IS the file path.
/// `attach` returns it as `AttachedPath::File`; `detach` is a no-op (the file
/// stays). `populate_streaming` writes raw bytes from a source file into the
/// destination file with no filesystem awareness.
#[allow(dead_code)]
pub struct LocalFileHostBackend;

#[async_trait::async_trait]
impl HostBackend for LocalFileHostBackend {
    fn kind(&self) -> BackendKind {
        BackendKind::LocalFile
    }

    async fn attach(&self, volume: &VolumeHandle) -> Result<AttachedPath, StorageError> {
        Ok(AttachedPath::File(PathBuf::from(&volume.locator)))
    }

    async fn detach(&self, _v: &VolumeHandle, _a: AttachedPath) -> Result<(), StorageError> {
        Ok(())
    }

    async fn populate_streaming(
        &self,
        attached: &AttachedPath,
        source: &Path,
        target_size_bytes: u64,
    ) -> Result<(), StorageError> {
        use tokio::io::AsyncWriteExt;
        let dst_path = attached.path();
        let mut src = tokio::fs::File::open(source).await?;
        let mut dst = tokio::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(dst_path)
            .await?;
        tokio::io::copy(&mut src, &mut dst).await?;
        let cur = tokio::fs::metadata(dst_path).await?.len();
        if target_size_bytes > cur {
            dst.set_len(target_size_bytes).await?;
        }
        dst.flush().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexus_storage::BackendInstanceId;
    use uuid::Uuid;

    #[tokio::test]
    async fn populate_streaming_is_a_pure_byte_copy() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("src.bin");
        let dst = dir.path().join("dst.bin");
        let data = vec![0xABu8; 8 * 1024];
        std::fs::write(&src, &data).unwrap();
        // Pre-create dst so attach has a path to return.
        std::fs::write(&dst, b"").unwrap();

        let h = VolumeHandle {
            volume_id: Uuid::new_v4(),
            backend_id: BackendInstanceId(Uuid::new_v4()),
            backend_kind: BackendKind::LocalFile,
            locator: dst.display().to_string(),
            size_bytes: 16 * 1024,
        };
        let backend = LocalFileHostBackend;
        let attached = backend.attach(&h).await.unwrap();
        backend
            .populate_streaming(&attached, &src, 16 * 1024)
            .await
            .unwrap();

        // Bytes from source are present.
        let written = std::fs::read(&dst).unwrap();
        assert_eq!(&written[..8 * 1024], &data[..]);
        // File extended to target size (sparse tail OK).
        assert_eq!(std::fs::metadata(&dst).unwrap().len(), 16 * 1024);
    }
}
