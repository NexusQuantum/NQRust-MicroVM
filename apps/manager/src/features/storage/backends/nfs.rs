//! NFS control-plane backend. Operator only configures `server` and
//! `export` — the manager handles its own mount lifecycle. On every
//! provision/destroy/clone/snapshot it ensures the share is mounted at
//! a deterministic path under `mount_base` (default `/var/lib/nqrust/nfs`,
//! overridable per-backend) and proceeds with the filesystem op there.
//!
//! The NFS-ness is captured in the locator JSON `{server, export, file}`
//! so the agent independently re-mounts on the agent host when it
//! attaches the volume — the agent's mount lifecycle is separate from
//! the manager's.

use nexus_storage::{
    BackendInstanceId, BackendKind, Capabilities, ControlPlaneBackend, CreateOpts, StorageError,
    VolumeHandle, VolumeSnapshotHandle,
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use uuid::Uuid;

const DEFAULT_MOUNT_BASE: &str = "/var/lib/nqrust/nfs";

#[derive(Debug, Clone, Deserialize)]
pub struct NfsConfig {
    pub server: String,
    pub export: String,
    /// Where the manager creates per-`(server, export)` mount points.
    /// Defaults to `/var/lib/nqrust/nfs` so most operators never set it.
    /// Each backend gets its own subdirectory under this base, derived
    /// deterministically from `(server, export)`, so two backends
    /// pointing at different exports on the same server don't collide.
    #[serde(default)]
    pub mount_base: Option<PathBuf>,
    /// If true, the manager trusts that the export is already mounted
    /// at the resolved mount point and skips the mount.nfs invocation.
    /// Useful for unit tests, environments managed by systemd.automount,
    /// or hosts where the manager runs unprivileged. Production
    /// deployments leave this false (the default) so the manager
    /// auto-mounts.
    #[serde(default)]
    pub assume_mounted: bool,
}

impl NfsConfig {
    fn mount_base(&self) -> PathBuf {
        self.mount_base
            .clone()
            .unwrap_or_else(|| PathBuf::from(DEFAULT_MOUNT_BASE))
    }

    /// Deterministic per-`(server, export)` mount point under
    /// `mount_base`. Same shape as the agent's `mount_point_for` so
    /// operators can reason about either side from one rule.
    pub fn mount_point(&self) -> PathBuf {
        let exp = self.export.trim_start_matches('/').replace('/', "_");
        let server_safe = self.server.replace([':', '/'], "_");
        self.mount_base().join(format!("{server_safe}:{exp}"))
    }
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
        let loc: NfsLocator = serde_json::from_str(s)
            .map_err(|e| StorageError::InvalidLocator(format!("decode nfs locator: {e}")))?;
        if loc.file.is_empty() || loc.file.contains('/') || loc.file.starts_with('.') {
            return Err(StorageError::InvalidLocator(format!(
                "nfs locator.file must be a plain filename (no '/', no leading '.'), got {:?}",
                loc.file
            )));
        }
        Ok(loc)
    }
}

pub struct NfsControlPlaneBackend {
    pub id: BackendInstanceId,
    pub config: NfsConfig,
}

impl NfsControlPlaneBackend {
    /// Idempotent mount. If the export is already mounted at
    /// `mount_point()` as the expected source, succeed silently. If
    /// nothing is mounted there, run `mount -t nfs <server>:<export>
    /// <mount_point>`. If something else is mounted there, fail loudly.
    ///
    /// Skipped when `config.assume_mounted` is true (test/automount).
    async fn ensure_mounted(&self) -> Result<PathBuf, StorageError> {
        let mount_point = self.config.mount_point();
        if self.config.assume_mounted {
            tokio::fs::create_dir_all(&mount_point).await?;
            return Ok(mount_point);
        }
        tokio::fs::create_dir_all(&mount_point).await?;
        let want = format!("{}:{}", self.config.server, self.config.export);
        let probe = tokio::process::Command::new("findmnt")
            .arg("--target")
            .arg(&mount_point)
            .arg("--noheadings")
            .arg("--output")
            .arg("SOURCE")
            .output()
            .await;
        let source_line = match probe {
            Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).trim().to_string(),
            // findmnt ran, exit != 0: nothing mounted there.
            Ok(_) => String::new(),
            // findmnt failed to spawn — probably missing from PATH. Surface
            // it so operators install nfs-common rather than discover the
            // double-mount silently.
            Err(e) => {
                return Err(StorageError::backend(std::io::Error::other(format!(
                    "findmnt not available (install nfs-common / util-linux): {e}"
                ))));
            }
        };
        if source_line == want {
            return Ok(mount_point);
        }
        if !source_line.is_empty() {
            return Err(StorageError::backend(std::io::Error::other(format!(
                "{} is already mounted from '{}', not '{}'",
                mount_point.display(),
                source_line,
                want
            ))));
        }
        let status = tokio::process::Command::new("mount")
            .arg("-t")
            .arg("nfs")
            .arg(&want)
            .arg(&mount_point)
            .status()
            .await
            .map_err(|e| {
                StorageError::backend(std::io::Error::other(format!("mount.nfs spawn: {e}")))
            })?;
        if !status.success() {
            return Err(StorageError::backend(std::io::Error::other(format!(
                "mount.nfs {} -> {} exited {}",
                want,
                mount_point.display(),
                status
            ))));
        }
        Ok(mount_point)
    }
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

    async fn provision(&self, opts: CreateOpts) -> Result<VolumeHandle, StorageError> {
        let mount = self.ensure_mounted().await?;
        let vol_id = Uuid::new_v4();
        let file = format!("nfs-{vol_id}.raw");
        let path = mount.join(&file);
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

    async fn destroy(&self, h: VolumeHandle) -> Result<(), StorageError> {
        let loc = NfsLocator::from_locator_str(&h.locator)?;
        let mount = self.ensure_mounted().await?;
        let path = mount.join(&loc.file);
        match tokio::fs::remove_file(&path).await {
            Ok(()) => Ok(()),
            // Idempotent: a destroy that races with another caller (or
            // re-runs after a crash) is success, not error.
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(StorageError::from(e)),
        }
    }

    async fn clone_from_image(
        &self,
        src: &Path,
        opts: CreateOpts,
    ) -> Result<VolumeHandle, StorageError> {
        let mount = self.ensure_mounted().await?;
        let vol_id = Uuid::new_v4();
        let file = format!("nfs-{vol_id}.raw");
        let dst = mount.join(&file);
        tokio::fs::copy(src, &dst).await?;
        let cur = tokio::fs::metadata(&dst).await?.len();
        if opts.size_bytes != cur {
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
    ) -> Result<VolumeSnapshotHandle, StorageError> {
        if name.is_empty() || name.contains('/') {
            return Err(StorageError::InvalidLocator(
                "snapshot name must be non-empty and contain no '/'".into(),
            ));
        }
        let mount = self.ensure_mounted().await?;
        let src_loc = NfsLocator::from_locator_str(&v.locator)?;
        let src_path = mount.join(&src_loc.file);
        let snap_file = format!("{}.snap-{name}", src_loc.file);
        let snap_path = mount.join(&snap_file);
        tokio::fs::copy(&src_path, &snap_path).await?;
        // Maintain the volume's provisioned size on the snapshot file so
        // clone_from_snapshot doesn't return a smaller size_bytes when
        // the source has been truncated below the provisioned size.
        let f = tokio::fs::OpenOptions::new()
            .write(true)
            .open(&snap_path)
            .await?;
        f.set_len(v.size_bytes).await?;
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
    ) -> Result<VolumeHandle, StorageError> {
        let mount = self.ensure_mounted().await?;
        let src_loc = NfsLocator::from_locator_str(&s.locator)?;
        let src_path = mount.join(&src_loc.file);
        let vol_id = Uuid::new_v4();
        let file = format!("nfs-{vol_id}.raw");
        let dst = mount.join(&file);
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

    async fn delete_snapshot(&self, s: VolumeSnapshotHandle) -> Result<(), StorageError> {
        let loc = NfsLocator::from_locator_str(&s.locator)?;
        let mount = self.ensure_mounted().await?;
        let path = mount.join(&loc.file);
        match tokio::fs::remove_file(&path).await {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(StorageError::from(e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexus_storage::{BackendInstanceId, ControlPlaneBackend, CreateOpts};
    use uuid::Uuid;

    #[test]
    fn nfs_config_parses_minimal_json_without_mount_path() {
        // Operator only types server + export. No `manager_mount_path` —
        // that step is now the manager's job, not the operator's.
        let json = serde_json::json!({
            "server": "10.0.0.5",
            "export": "/mnt/tank/vms"
        });
        let cfg: NfsConfig = serde_json::from_value(json).unwrap();
        assert_eq!(cfg.server, "10.0.0.5");
        assert_eq!(cfg.export, "/mnt/tank/vms");
        assert_eq!(cfg.mount_base, None);
        assert!(!cfg.assume_mounted);
    }

    #[test]
    fn mount_base_defaults_to_var_lib_nqrust_nfs() {
        let cfg = NfsConfig {
            server: "10.0.0.5".into(),
            export: "/mnt/tank/vms".into(),
            mount_base: None,
            assume_mounted: false,
        };
        assert_eq!(cfg.mount_base(), PathBuf::from(DEFAULT_MOUNT_BASE));
    }

    #[test]
    fn mount_point_is_unique_and_filesystem_safe() {
        let make = |server: &str, export: &str| NfsConfig {
            server: server.into(),
            export: export.into(),
            mount_base: Some(PathBuf::from("/var/lib/nqrust/nfs")),
            assume_mounted: false,
        };
        let a = make("10.0.0.5", "/mnt/tank/vms").mount_point();
        let b = make("10.0.0.5", "/mnt/tank/iso").mount_point();
        let c = make("10.0.0.6", "/mnt/tank/vms").mount_point();
        assert_ne!(a, b);
        assert_ne!(a, c);
        assert_eq!(
            a,
            PathBuf::from("/var/lib/nqrust/nfs/10.0.0.5:mnt_tank_vms")
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

    #[test]
    fn locator_with_slash_in_file_is_rejected() {
        let bad = serde_json::json!({
            "server": "10.0.0.5",
            "export": "/mnt/tank/vms",
            "file": "../../etc/passwd"
        })
        .to_string();
        let err = NfsLocator::from_locator_str(&bad).unwrap_err();
        assert!(matches!(err, StorageError::InvalidLocator(_)), "{err}");
    }

    #[test]
    fn locator_with_leading_dot_in_file_is_rejected() {
        let bad = serde_json::json!({
            "server": "10.0.0.5",
            "export": "/mnt/tank/vms",
            "file": ".hidden"
        })
        .to_string();
        let err = NfsLocator::from_locator_str(&bad).unwrap_err();
        assert!(matches!(err, StorageError::InvalidLocator(_)), "{err}");
    }

    /// Build a backend whose `mount_base` is a tempdir and whose
    /// `assume_mounted` flag is true so the runtime tests don't try
    /// to invoke `mount.nfs`. The mount point under the tempdir
    /// substitutes for a real NFS mount; the rest of the codepath is
    /// identical.
    fn temp_backend() -> (NfsControlPlaneBackend, tempfile::TempDir) {
        let dir = tempfile::tempdir().expect("tempdir");
        let backend = NfsControlPlaneBackend {
            id: BackendInstanceId(Uuid::new_v4()),
            config: NfsConfig {
                server: "10.0.0.5".into(),
                export: "/mnt/tank/vms".into(),
                mount_base: Some(dir.path().to_path_buf()),
                assume_mounted: true,
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
        let path = backend.config.mount_point().join(&loc.file);
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
        let path = backend.config.mount_point().join(&loc.file);
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
        let h = backend
            .provision(CreateOpts {
                name: "v".into(),
                size_bytes: 1024,
                description: None,
            })
            .await
            .unwrap();
        let loc = NfsLocator::from_locator_str(&h.locator).unwrap();
        let src_path = backend.config.mount_point().join(&loc.file);
        // Write LESS data than the provisioned size to verify the
        // snapshot still reports the full provisioned size.
        tokio::fs::write(&src_path, b"original-data").await.unwrap();

        let snap = backend.snapshot(&h, "snap-1").await.expect("snapshot");
        let snap_loc = NfsLocator::from_locator_str(&snap.locator).unwrap();
        let snap_path = backend.config.mount_point().join(&snap_loc.file);
        let snap_meta = tokio::fs::metadata(&snap_path).await.unwrap();
        assert_eq!(
            snap_meta.len(),
            1024,
            "snapshot file must be at provisioned size"
        );
        let snap_data = tokio::fs::read(&snap_path).await.unwrap();
        assert_eq!(&snap_data[..13], b"original-data");

        let cloned = backend.clone_from_snapshot(&snap).await.expect("clone");
        let cloned_loc = NfsLocator::from_locator_str(&cloned.locator).unwrap();
        let cloned_path = backend.config.mount_point().join(&cloned_loc.file);
        let cloned_meta = tokio::fs::metadata(&cloned_path).await.unwrap();
        assert_eq!(
            cloned_meta.len(),
            1024,
            "clone must inherit provisioned size"
        );
        assert_eq!(
            cloned.size_bytes, 1024,
            "VolumeHandle.size_bytes must report provisioned size"
        );
        let cloned_data = tokio::fs::read(&cloned_path).await.unwrap();
        assert_eq!(&cloned_data[..13], b"original-data");

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
        let path = backend.config.mount_point().join(&loc.file);
        let meta = tokio::fs::metadata(&path).await.unwrap();
        assert_eq!(meta.len(), 4096);
        let buf = tokio::fs::read(&path).await.unwrap();
        assert_eq!(&buf[..11], b"hello world");
    }

    #[tokio::test]
    async fn clone_from_image_truncates_when_source_larger_than_requested() {
        let (backend, _guard) = temp_backend();
        let src_dir = tempfile::tempdir().unwrap();
        let src = src_dir.path().join("base.raw");
        // Source is 8 KiB.
        tokio::fs::write(&src, vec![0xab; 8 * 1024]).await.unwrap();
        let opts = CreateOpts {
            name: "v".into(),
            size_bytes: 4 * 1024, // request 4 KiB — smaller than source
            description: None,
        };
        let h = backend.clone_from_image(&src, opts).await.unwrap();
        let loc = NfsLocator::from_locator_str(&h.locator).unwrap();
        let path = backend.config.mount_point().join(&loc.file);
        let meta = tokio::fs::metadata(&path).await.unwrap();
        assert_eq!(
            meta.len(),
            4 * 1024,
            "destination should be truncated to requested size"
        );
        assert_eq!(h.size_bytes, 4 * 1024);
    }
}
