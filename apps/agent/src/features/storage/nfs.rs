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
    /// If true, attach trusts that the export is already mounted at
    /// `mount_point_for(...)` and does not invoke mount.nfs. Used in
    /// unit tests and for environments where an external service (e.g.
    /// systemd automount) manages mounts.
    pub assume_mounted: bool,
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

    async fn ensure_mounted(
        &self,
        server: &str,
        export: &str,
        mount_point: &std::path::Path,
    ) -> Result<(), StorageError> {
        tokio::fs::create_dir_all(mount_point).await?;
        // Already mounted? findmnt prints the source if so; success exit.
        let probe = tokio::process::Command::new("findmnt")
            .arg("--target")
            .arg(mount_point)
            .arg("--noheadings")
            .arg("--output")
            .arg("SOURCE")
            .output()
            .await;
        let source_line = match probe {
            Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).trim().to_string(),
            _ => String::new(),
        };
        let want = format!("{server}:{export}");
        if source_line == want {
            return Ok(());
        }
        if !source_line.is_empty() {
            return Err(StorageError::backend(std::io::Error::other(format!(
                "{} is mounted but as '{}', not '{}'",
                mount_point.display(),
                source_line,
                want
            ))));
        }
        // Not mounted — mount it.
        let status = tokio::process::Command::new("mount")
            .arg("-t")
            .arg("nfs")
            .arg(&want)
            .arg(mount_point)
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
        Ok(())
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
        if !self.config.assume_mounted {
            self.ensure_mounted(&loc.server, &loc.export, &mount)
                .await?;
        }
        let path = mount.join(&loc.file);
        if tokio::fs::metadata(&path).await.is_err() {
            return Err(StorageError::backend(std::io::Error::other(format!(
                "expected file {} on mounted export",
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
        attached: &AttachedPath,
        source: &std::path::Path,
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

    async fn resize2fs(&self, attached: &AttachedPath) -> Result<(), StorageError> {
        super::local_file::run_resize2fs(attached.path()).await
    }

    async fn read_snapshot(
        &self,
        snap: &VolumeSnapshotHandle,
    ) -> Result<Box<dyn tokio::io::AsyncRead + Send + Unpin>, StorageError> {
        let loc = self.locator(&snap.locator)?;
        let mount = self.config.mount_point_for(&loc.server, &loc.export);
        if !self.config.assume_mounted {
            self.ensure_mounted(&loc.server, &loc.export, &mount)
                .await?;
        }
        let path = mount.join(&loc.file);
        let f = tokio::fs::File::open(&path).await?;
        Ok(Box::new(f))
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
            assume_mounted: true,
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
            assume_mounted: true,
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

    #[tokio::test]
    async fn populate_streaming_copies_then_truncates() {
        let base = tempfile::tempdir().unwrap();
        let cfg = NfsHostConfig {
            mount_base: base.path().to_path_buf(),
            assume_mounted: true,
        };
        let server = "10.0.0.5";
        let export = "/mnt/tank/vms";
        let file = "nfs-pop.raw";
        let (path, _g) = fake_mounted_export(&cfg, server, export, file);

        let src_dir = tempfile::tempdir().unwrap();
        let src = src_dir.path().join("base.raw");
        tokio::fs::write(&src, b"abc").await.unwrap();

        let backend = NfsHostBackend::new(cfg);
        backend
            .populate_streaming(&AttachedPath::File(path.clone()), &src, 16)
            .await
            .unwrap();

        let written = tokio::fs::read(&path).await.unwrap();
        assert_eq!(&written[..3], b"abc");
        assert_eq!(written.len(), 16);
    }

    #[tokio::test]
    async fn resize2fs_invokes_the_shared_helper() {
        // Smoke: resize2fs against a non-ext4 file returns Err. This
        // confirms wiring (the helper is reachable + invoked) without
        // requiring a real ext4 image in the test.
        let base = tempfile::tempdir().unwrap();
        let cfg = NfsHostConfig {
            mount_base: base.path().to_path_buf(),
            assume_mounted: true,
        };
        let path = base.path().join("not-ext4.raw");
        tokio::fs::write(&path, b"not an ext4 superblock")
            .await
            .unwrap();
        let backend = NfsHostBackend::new(cfg);
        let res = backend.resize2fs(&AttachedPath::File(path)).await;
        assert!(res.is_err());
    }

    /// Live test: requires running as root or with CAP_SYS_ADMIN, and
    /// requires an NFS server reachable at the env-configured address.
    /// Skipped by default; run with `cargo test -- --include-ignored`
    /// after exporting `NQRUST_NFS_SMOKE_SERVER` and
    /// `NQRUST_NFS_SMOKE_EXPORT`.
    #[tokio::test]
    #[ignore]
    async fn attach_mounts_the_export_when_not_mounted() {
        let server = match std::env::var("NQRUST_NFS_SMOKE_SERVER") {
            Ok(s) => s,
            Err(_) => return,
        };
        let export = std::env::var("NQRUST_NFS_SMOKE_EXPORT").expect("NQRUST_NFS_SMOKE_EXPORT");
        let base = tempfile::tempdir().unwrap();
        let cfg = NfsHostConfig {
            mount_base: base.path().to_path_buf(),
            assume_mounted: false,
        };
        let backend = NfsHostBackend::new(cfg.clone());
        // Pre-create the test file directly on the export so attach
        // succeeds. Caller is responsible for ensuring the export is
        // writable from this test host.
        let mount = cfg.mount_point_for(&server, &export);
        std::fs::create_dir_all(&mount).unwrap();
        let mnt_status = std::process::Command::new("mount")
            .args([
                "-t",
                "nfs",
                &format!("{server}:{export}"),
                mount.to_str().unwrap(),
            ])
            .status()
            .unwrap();
        assert!(mnt_status.success(), "pre-mount failed");
        let file = "nfs-attach-test.raw";
        std::fs::write(mount.join(file), b"x").unwrap();
        std::process::Command::new("umount")
            .arg(&mount)
            .status()
            .unwrap();

        // Now exercise attach: it must mount + return the path.
        let v = VolumeHandle {
            volume_id: Uuid::new_v4(),
            backend_id: nexus_storage::BackendInstanceId(Uuid::new_v4()),
            backend_kind: BackendKind::Nfs,
            locator: locator_json(&server, &export, file),
            size_bytes: 1,
        };
        let attached = backend.attach(&v).await.unwrap();
        assert!(attached.path().exists());
        std::process::Command::new("umount")
            .arg(&mount)
            .status()
            .unwrap();
    }

    #[tokio::test]
    async fn read_snapshot_returns_file_contents() {
        use tokio::io::AsyncReadExt;

        let base = tempfile::tempdir().unwrap();
        let cfg = NfsHostConfig {
            mount_base: base.path().to_path_buf(),
            assume_mounted: true,
        };
        let server = "10.0.0.5";
        let export = "/mnt/tank/vms";
        let file = "nfs-abc.raw.snap-x";
        let (path, _g) = fake_mounted_export(&cfg, server, export, file);
        tokio::fs::write(&path, b"snapshot-bytes").await.unwrap();

        let backend = NfsHostBackend::new(cfg);
        let snap = VolumeSnapshotHandle {
            snapshot_id: Uuid::new_v4(),
            backend_id: nexus_storage::BackendInstanceId(Uuid::new_v4()),
            backend_kind: BackendKind::Nfs,
            locator: locator_json(server, export, file),
            source_volume_id: Uuid::new_v4(),
        };
        let mut r = backend.read_snapshot(&snap).await.unwrap();
        let mut buf = Vec::new();
        r.read_to_end(&mut buf).await.unwrap();
        assert_eq!(buf, b"snapshot-bytes");
    }
}
