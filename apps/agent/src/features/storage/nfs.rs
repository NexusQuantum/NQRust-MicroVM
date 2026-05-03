//! Agent-side NFS host backend. Each unique (server, export) pair gets
//! its own mount point under `mount_base`. `attach` ensures the export
//! is mounted and returns the path to the volume's file. `detach` is a
//! no-op in v1 — the agent leaves the mount in place across volume
//! lifecycles for two reasons: (1) re-mounting is slow, (2) other
//! volumes on the same export may still be attached.

use std::path::PathBuf;

use async_trait::async_trait;
use nexus_storage::{
    AttachedPath, BackendKind, HostBackend, StorageError, VolumeHandle, VolumeSnapshotHandle,
};
use serde::Deserialize;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct NfsHostConfig {
    pub mount_base: PathBuf,
}

#[allow(dead_code)]
impl NfsHostConfig {
    /// Deterministic per-(server, export) directory name. The export's
    /// leading slash is stripped and remaining slashes become `_` so the
    /// result is a single path component. Server is appended literally
    /// after a `:`.
    pub fn mount_point_for(&self, server: &str, export: &str) -> PathBuf {
        let exp = export.trim_start_matches('/').replace('/', "_");
        let server_safe = server.replace([':', '/'], "_");
        self.mount_base.join(format!("{server_safe}:{exp}"))
    }
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct NfsLocatorWire {
    server: String,
    export: String,
    file: String,
}

#[allow(dead_code)]
pub struct NfsHostBackend {
    config: NfsHostConfig,
}

#[allow(dead_code)]
impl NfsHostBackend {
    pub fn new(config: NfsHostConfig) -> Self {
        Self { config }
    }

    fn locator(&self, raw: &str) -> Result<NfsLocatorWire, StorageError> {
        serde_json::from_str(raw)
            .map_err(|e| StorageError::InvalidLocator(format!("decode nfs locator: {e}")))
    }
}

#[async_trait]
impl HostBackend for NfsHostBackend {
    fn kind(&self) -> BackendKind {
        BackendKind::Nfs
    }

    async fn attach(&self, volume: &VolumeHandle) -> Result<AttachedPath, StorageError> {
        let loc = self.locator(&volume.locator)?;
        let mount = self.config.mount_point_for(&loc.server, &loc.export);
        let path = mount.join(&loc.file);
        if !path.exists() {
            return Err(StorageError::backend(std::io::Error::other(format!(
                "expected file {} on mounted export; mount missing or volume not provisioned",
                path.display()
            ))));
        }
        Ok(AttachedPath::File(path))
    }

    async fn detach(&self, _v: &VolumeHandle, _a: AttachedPath) -> Result<(), StorageError> {
        // v1: no-op. Mounts are kept across volume lifecycles. The
        // operator can unmount manually or via a future cleanup route.
        Ok(())
    }

    async fn populate_streaming(
        &self,
        _attached: &AttachedPath,
        _source: &std::path::Path,
        _target_size_bytes: u64,
    ) -> Result<(), StorageError> {
        // Implemented in Task 11.
        Err(StorageError::NotSupported(
            "populate_streaming not yet implemented".into(),
        ))
    }

    async fn resize2fs(&self, _attached: &AttachedPath) -> Result<(), StorageError> {
        // Implemented in Task 12.
        Err(StorageError::NotSupported(
            "resize2fs not yet implemented".into(),
        ))
    }

    async fn read_snapshot(
        &self,
        _snap: &VolumeSnapshotHandle,
    ) -> Result<Box<dyn tokio::io::AsyncRead + Send + Unpin>, StorageError> {
        // Implemented in Task 13.
        Err(StorageError::NotSupported(
            "read_snapshot not yet implemented".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexus_storage::{BackendKind, HostBackend, VolumeHandle};
    use tempfile::TempDir;
    use uuid::Uuid;

    #[test]
    fn mount_point_is_unique_per_server_export_and_filesystem_safe() {
        let cfg = NfsHostConfig {
            mount_base: PathBuf::from("/var/lib/nqrust/nfs"),
        };
        let a = cfg.mount_point_for("10.0.0.5", "/mnt/tank/vms");
        let b = cfg.mount_point_for("10.0.0.5", "/mnt/tank/iso");
        let c = cfg.mount_point_for("10.0.0.6", "/mnt/tank/vms");
        assert_ne!(a, b);
        assert_ne!(a, c);
        assert_eq!(
            a,
            PathBuf::from("/var/lib/nqrust/nfs/10.0.0.5:mnt_tank_vms")
        );
    }

    /// Pretends the export is already mounted at `mount_point_for(...)`
    /// by creating that directory and dropping a file inside it.
    fn fake_mounted_export(
        cfg: &NfsHostConfig,
        server: &str,
        export: &str,
        file: &str,
    ) -> (PathBuf, TempDir) {
        let mount = cfg.mount_point_for(server, export);
        std::fs::create_dir_all(&mount).unwrap();
        let path = mount.join(file);
        std::fs::write(&path, b"hello").unwrap();
        let guard = tempfile::tempdir().unwrap();
        (path, guard)
    }

    fn locator_json(server: &str, export: &str, file: &str) -> String {
        serde_json::json!({
            "server": server,
            "export": export,
            "file": file
        })
        .to_string()
    }

    #[tokio::test]
    async fn attach_returns_file_path_under_mount_point() {
        let base = tempfile::tempdir().unwrap();
        let cfg = NfsHostConfig {
            mount_base: base.path().to_path_buf(),
        };
        let server = "10.0.0.5";
        let export = "/mnt/tank/vms";
        let file = "nfs-abc.raw";
        let (expected_path, _guard) = fake_mounted_export(&cfg, server, export, file);
        let backend = NfsHostBackend::new(cfg);
        let v = VolumeHandle {
            volume_id: Uuid::new_v4(),
            backend_id: nexus_storage::BackendInstanceId(Uuid::new_v4()),
            backend_kind: BackendKind::Nfs,
            locator: locator_json(server, export, file),
            size_bytes: 5,
        };
        let attached = backend.attach(&v).await.unwrap();
        assert_eq!(attached.path(), expected_path.as_path());
    }
}
