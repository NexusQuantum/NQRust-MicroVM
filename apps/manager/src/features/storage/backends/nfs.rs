//! NFS control-plane backend. The manager accesses the export through a
//! local mount (`manager_mount_path`); all provision / destroy / clone
//! ops are filesystem ops against that mount, just like LocalFile. The
//! NFS-ness is captured in the locator JSON so the agent knows what to
//! mount when it later attaches the volume.

use nexus_storage::StorageError;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize)]
pub struct NfsConfig {
    pub server: String,
    pub export: String,
    pub manager_mount_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NfsLocator {
    pub server: String,
    pub export: String,
    pub file: String,
}

impl NfsLocator {
    pub fn to_locator_string(&self) -> Result<String, StorageError> {
        serde_json::to_string(self)
            .map_err(|e| StorageError::InvalidLocator(format!("encode nfs locator: {e}")))
    }

    pub fn from_locator_str(s: &str) -> Result<Self, StorageError> {
        serde_json::from_str(s)
            .map_err(|e| StorageError::InvalidLocator(format!("decode nfs locator: {e}")))
    }
}

use nexus_storage::{
    BackendInstanceId, BackendKind, Capabilities, ControlPlaneBackend, CreateOpts, VolumeHandle,
    VolumeSnapshotHandle,
};
use uuid::Uuid;

pub struct NfsControlPlaneBackend {
    pub id: BackendInstanceId,
    pub config: NfsConfig,
}

#[async_trait::async_trait]
impl ControlPlaneBackend for NfsControlPlaneBackend {
    fn kind(&self) -> BackendKind {
        BackendKind::Nfs
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            supports_native_snapshots: true,
            supports_concurrent_attach: false,
            supports_live_migration: false,
            supports_clone_from_image: true,
        }
    }

    async fn provision(
        &self,
        opts: CreateOpts,
    ) -> Result<VolumeHandle, nexus_storage::StorageError> {
        let vol_id = Uuid::new_v4();
        let file = format!("nfs-{vol_id}.raw");
        let path = self.config.manager_mount_path.join(&file);
        tokio::fs::create_dir_all(&self.config.manager_mount_path).await?;
        let f = tokio::fs::File::create(&path).await?;
        f.set_len(opts.size_bytes).await?;
        drop(f);
        let locator = NfsLocator {
            server: self.config.server.clone(),
            export: self.config.export.clone(),
            file,
        };
        Ok(VolumeHandle {
            volume_id: vol_id,
            backend_id: self.id,
            backend_kind: BackendKind::Nfs,
            locator: locator.to_locator_string()?,
            size_bytes: opts.size_bytes,
        })
    }

    async fn destroy(&self, h: VolumeHandle) -> Result<(), nexus_storage::StorageError> {
        let loc = NfsLocator::from_locator_str(&h.locator)?;
        let path = self.config.manager_mount_path.join(&loc.file);
        match tokio::fs::remove_file(&path).await {
            Ok(()) => Ok(()),
            // Idempotent: a destroy that races with another caller (or
            // re-runs after a crash) is success, not error.
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(nexus_storage::StorageError::from(e)),
        }
    }

    async fn clone_from_image(
        &self,
        src: &std::path::Path,
        opts: CreateOpts,
    ) -> Result<VolumeHandle, nexus_storage::StorageError> {
        let vol_id = Uuid::new_v4();
        let file = format!("nfs-{vol_id}.raw");
        let dst = self.config.manager_mount_path.join(&file);
        tokio::fs::create_dir_all(&self.config.manager_mount_path).await?;
        tokio::fs::copy(src, &dst).await?;
        let cur = tokio::fs::metadata(&dst).await?.len();
        if opts.size_bytes > cur {
            let f = tokio::fs::OpenOptions::new().write(true).open(&dst).await?;
            f.set_len(opts.size_bytes).await?;
        }
        let locator = NfsLocator {
            server: self.config.server.clone(),
            export: self.config.export.clone(),
            file,
        };
        Ok(VolumeHandle {
            volume_id: vol_id,
            backend_id: self.id,
            backend_kind: BackendKind::Nfs,
            locator: locator.to_locator_string()?,
            size_bytes: opts.size_bytes,
        })
    }

    async fn snapshot(
        &self,
        v: &VolumeHandle,
        name: &str,
    ) -> Result<VolumeSnapshotHandle, nexus_storage::StorageError> {
        if name.is_empty() || name.contains('/') {
            return Err(nexus_storage::StorageError::InvalidLocator(
                "snapshot name must be non-empty and contain no '/'".into(),
            ));
        }
        let src_loc = NfsLocator::from_locator_str(&v.locator)?;
        let src_path = self.config.manager_mount_path.join(&src_loc.file);
        let snap_file = format!("{}.snap-{name}", src_loc.file);
        let snap_path = self.config.manager_mount_path.join(&snap_file);
        tokio::fs::copy(&src_path, &snap_path).await?;
        let snap_locator = NfsLocator {
            server: src_loc.server,
            export: src_loc.export,
            file: snap_file,
        };
        Ok(VolumeSnapshotHandle {
            snapshot_id: Uuid::new_v4(),
            backend_id: self.id,
            backend_kind: BackendKind::Nfs,
            locator: snap_locator.to_locator_string()?,
            source_volume_id: v.volume_id,
        })
    }

    async fn clone_from_snapshot(
        &self,
        s: &VolumeSnapshotHandle,
    ) -> Result<VolumeHandle, nexus_storage::StorageError> {
        let src_loc = NfsLocator::from_locator_str(&s.locator)?;
        let src_path = self.config.manager_mount_path.join(&src_loc.file);
        let vol_id = Uuid::new_v4();
        let file = format!("nfs-{vol_id}.raw");
        let dst = self.config.manager_mount_path.join(&file);
        tokio::fs::copy(&src_path, &dst).await?;
        // The snapshot file is already at the provisioned size (it's a
        // straight copy of the source volume), so no truncation here.
        let size_bytes = tokio::fs::metadata(&dst).await?.len();
        let locator = NfsLocator {
            server: src_loc.server,
            export: src_loc.export,
            file,
        };
        Ok(VolumeHandle {
            volume_id: vol_id,
            backend_id: self.id,
            backend_kind: BackendKind::Nfs,
            locator: locator.to_locator_string()?,
            size_bytes,
        })
    }

    async fn delete_snapshot(
        &self,
        s: VolumeSnapshotHandle,
    ) -> Result<(), nexus_storage::StorageError> {
        let loc = NfsLocator::from_locator_str(&s.locator)?;
        let path = self.config.manager_mount_path.join(&loc.file);
        match tokio::fs::remove_file(&path).await {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(nexus_storage::StorageError::from(e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nfs_config_parses_minimal_json() {
        let json = serde_json::json!({
            "server": "10.0.0.5",
            "export": "/mnt/tank/vms",
            "manager_mount_path": "/mnt/nfs-manager"
        });
        let cfg: NfsConfig = serde_json::from_value(json).unwrap();
        assert_eq!(cfg.server, "10.0.0.5");
        assert_eq!(cfg.export, "/mnt/tank/vms");
        assert_eq!(
            cfg.manager_mount_path,
            std::path::PathBuf::from("/mnt/nfs-manager")
        );
    }

    #[test]
    fn nfs_locator_round_trips() {
        let loc = NfsLocator {
            server: "10.0.0.5".into(),
            export: "/mnt/tank/vms".into(),
            file: "nfs-abc.raw".into(),
        };
        let s = loc.to_locator_string().unwrap();
        let back = NfsLocator::from_locator_str(&s).unwrap();
        assert_eq!(back, loc);
    }

    use nexus_storage::{BackendInstanceId, ControlPlaneBackend, CreateOpts};
    use uuid::Uuid;

    fn temp_backend() -> (NfsControlPlaneBackend, tempfile::TempDir) {
        let dir = tempfile::tempdir().expect("tempdir");
        let backend = NfsControlPlaneBackend {
            id: BackendInstanceId(Uuid::new_v4()),
            config: NfsConfig {
                server: "10.0.0.5".into(),
                export: "/mnt/tank/vms".into(),
                manager_mount_path: dir.path().to_path_buf(),
            },
        };
        (backend, dir)
    }

    #[tokio::test]
    async fn destroy_unlinks_the_file() {
        let (backend, _guard) = temp_backend();
        let h = backend
            .provision(CreateOpts {
                name: "v".into(),
                size_bytes: 1024,
                description: None,
            })
            .await
            .unwrap();
        let loc = NfsLocator::from_locator_str(&h.locator).unwrap();
        let path = backend.config.manager_mount_path.join(&loc.file);
        assert!(tokio::fs::metadata(&path).await.is_ok());
        backend.destroy(h).await.expect("destroy");
        assert!(tokio::fs::metadata(&path).await.is_err());
    }

    #[tokio::test]
    async fn destroy_is_idempotent_when_file_missing() {
        let (backend, _guard) = temp_backend();
        let bogus = VolumeHandle {
            volume_id: Uuid::new_v4(),
            backend_id: backend.id,
            backend_kind: BackendKind::Nfs,
            locator: NfsLocator {
                server: backend.config.server.clone(),
                export: backend.config.export.clone(),
                file: "nfs-does-not-exist.raw".into(),
            }
            .to_locator_string()
            .unwrap(),
            size_bytes: 0,
        };
        backend.destroy(bogus).await.expect("idempotent destroy");
    }

    #[tokio::test]
    async fn provision_creates_a_sparse_file_at_requested_size() {
        let (backend, _guard) = temp_backend();
        let opts = CreateOpts {
            name: "vol-1".into(),
            size_bytes: 4 * 1024 * 1024,
            description: None,
        };
        let h = backend.provision(opts).await.expect("provision");
        let loc = NfsLocator::from_locator_str(&h.locator).unwrap();
        let path = backend.config.manager_mount_path.join(&loc.file);
        let meta = tokio::fs::metadata(&path).await.unwrap();
        assert_eq!(meta.len(), 4 * 1024 * 1024);
        assert_eq!(loc.server, "10.0.0.5");
        assert_eq!(loc.export, "/mnt/tank/vms");
        assert!(loc.file.starts_with("nfs-"));
        assert!(loc.file.ends_with(".raw"));
    }

    #[tokio::test]
    async fn snapshot_then_clone_then_delete_round_trip() {
        let (backend, _guard) = temp_backend();
        // Provision + populate the source.
        let h = backend
            .provision(CreateOpts {
                name: "v".into(),
                size_bytes: 1024,
                description: None,
            })
            .await
            .unwrap();
        let loc = NfsLocator::from_locator_str(&h.locator).unwrap();
        let src_path = backend.config.manager_mount_path.join(&loc.file);
        tokio::fs::write(&src_path, b"original-data").await.unwrap();

        // snapshot
        let snap = backend.snapshot(&h, "snap-1").await.expect("snapshot");
        let snap_loc = NfsLocator::from_locator_str(&snap.locator).unwrap();
        let snap_path = backend.config.manager_mount_path.join(&snap_loc.file);
        assert_eq!(tokio::fs::read(&snap_path).await.unwrap(), b"original-data");

        // clone_from_snapshot
        let cloned = backend.clone_from_snapshot(&snap).await.expect("clone");
        let cloned_loc = NfsLocator::from_locator_str(&cloned.locator).unwrap();
        let cloned_path = backend.config.manager_mount_path.join(&cloned_loc.file);
        let cloned_data = tokio::fs::read(&cloned_path).await.unwrap();
        // The cloned file may be padded to size_bytes; first 13 bytes must match.
        assert_eq!(&cloned_data[..13], b"original-data");

        // delete_snapshot
        backend
            .delete_snapshot(snap)
            .await
            .expect("delete_snapshot");
        assert!(tokio::fs::metadata(&snap_path).await.is_err());
    }

    #[tokio::test]
    async fn clone_from_image_copies_and_resizes() {
        let (backend, _guard) = temp_backend();
        let src_dir = tempfile::tempdir().unwrap();
        let src = src_dir.path().join("base.raw");
        tokio::fs::write(&src, b"hello world").await.unwrap();
        let opts = CreateOpts {
            name: "v".into(),
            size_bytes: 4096,
            description: None,
        };
        let h = backend.clone_from_image(&src, opts).await.unwrap();
        let loc = NfsLocator::from_locator_str(&h.locator).unwrap();
        let path = backend.config.manager_mount_path.join(&loc.file);
        let meta = tokio::fs::metadata(&path).await.unwrap();
        assert_eq!(meta.len(), 4096);
        let buf = tokio::fs::read(&path).await.unwrap();
        assert_eq!(&buf[..11], b"hello world");
    }
}
