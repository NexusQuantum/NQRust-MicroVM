//! SMB / CIFS control-plane backend. Operator configures `server`, `share`,
//! and an agent URL. The manager delegates privileged mount/file work to the
//! agent because the manager is not expected to run with `CAP_SYS_ADMIN`
//! and has no business spawning `mount.cifs`.
//!
//! The SMB-ness is captured in the locator JSON `{server, share, subdir,
//! file}` so the agent independently re-mounts the share on the agent host
//! when it attaches the volume — same pattern as the NFS backend.

use nexus_storage::{
    BackendInstanceId, BackendKind, Capabilities, ControlPlaneBackend, CreateOpts, StorageError,
    VolumeHandle, VolumeSnapshotHandle,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::path::{Path, PathBuf};
use uuid::Uuid;

const DEFAULT_MOUNT_BASE: &str = "/var/lib/nqrust/smb";

#[derive(Debug, Clone, Deserialize)]
pub struct SmbConfig {
    pub server: String,
    pub share: String,
    #[serde(default)]
    pub subdir: Option<String>,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub domain: Option<String>,
    #[serde(default)]
    pub smb_version: Option<String>,
    /// Extra `-o` options passed verbatim to `mount.cifs` (e.g.
    /// `uid=33,gid=33,file_mode=0660`). Up to the operator to keep this
    /// well-formed; the agent does not parse it.
    #[serde(default)]
    pub options: Option<String>,
    /// Where externally-managed manager-local mounts appear when
    /// `assume_mounted=true`. Defaults to `/var/lib/nqrust/smb`. Each
    /// backend gets its own subdirectory under this base, derived
    /// deterministically from `(server, share)`.
    #[serde(default)]
    pub mount_base: Option<PathBuf>,
    /// If true, the manager assumes the agent (or an external system like
    /// systemd.automount) already has the share mounted at the resolved
    /// mount point and skips the agent `/mount` call. Useful for tests and
    /// externally-managed mounts.
    #[serde(default)]
    pub assume_mounted: bool,
    /// Base URL of the agent that owns this SMB share, for example
    /// `http://127.0.0.1:9090`. The manager appends `/v1/storage/smb/*`.
    /// If omitted, only `assume_mounted=true` mode is supported.
    #[serde(default)]
    pub agent_url: Option<String>,
}

impl SmbConfig {
    fn mount_base(&self) -> PathBuf {
        self.mount_base
            .clone()
            .unwrap_or_else(|| PathBuf::from(DEFAULT_MOUNT_BASE))
    }

    /// Deterministic per-`(server, share)` mount point under `mount_base`.
    /// Same shape as the agent's `mount_point_for` so operators can reason
    /// about either side from one rule.
    pub fn mount_point(&self) -> PathBuf {
        let share_safe = self.share.trim_start_matches('/').replace('/', "_");
        let server_safe = self.server.replace([':', '/'], "_");
        self.mount_base()
            .join(format!("{server_safe}:{share_safe}"))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SmbLocator {
    pub server: String,
    pub share: String,
    #[serde(default)]
    pub subdir: Option<String>,
    pub file: String,
}

impl SmbLocator {
    pub fn to_locator_string(&self) -> Result<String, StorageError> {
        serde_json::to_string(self)
            .map_err(|e| StorageError::InvalidLocator(format!("encode smb locator: {e}")))
    }

    pub fn from_locator_str(s: &str) -> Result<Self, StorageError> {
        let loc: SmbLocator = serde_json::from_str(s)
            .map_err(|e| StorageError::InvalidLocator(format!("decode smb locator: {e}")))?;
        if loc.file.is_empty() || loc.file.contains('/') || loc.file.starts_with('.') {
            return Err(StorageError::InvalidLocator(format!(
                "smb locator.file must be a plain filename (no '/', no leading '.'), got {:?}",
                loc.file
            )));
        }
        Ok(loc)
    }
}

pub struct SmbControlPlaneBackend {
    pub id: BackendInstanceId,
    pub config: SmbConfig,
}

impl SmbControlPlaneBackend {
    fn agent_storage_base(&self) -> Option<String> {
        self.config.agent_url.as_ref().map(|raw| {
            let with_scheme = if raw.starts_with("http://") || raw.starts_with("https://") {
                raw.to_string()
            } else {
                format!("http://{raw}")
            };
            let trimmed = with_scheme.trim_end_matches('/');
            if trimmed.ends_with("/v1/storage") {
                trimmed.to_string()
            } else {
                format!("{trimmed}/v1/storage")
            }
        })
    }

    fn agent_smb_url(&self, op: &str) -> Option<String> {
        self.agent_storage_base()
            .map(|base| format!("{base}/smb/{op}"))
    }

    async fn agent_post<Req, Resp>(&self, op: &str, req: &Req) -> Result<Resp, StorageError>
    where
        Req: Serialize + ?Sized,
        Resp: DeserializeOwned,
    {
        let url = self.agent_smb_url(op).ok_or_else(|| {
            StorageError::backend(std::io::Error::other(
                "smb backend requires config.agent_url, or assume_mounted=true for local testing",
            ))
        })?;
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| {
                StorageError::backend(std::io::Error::other(format!(
                    "agent smb client init failed: {e}"
                )))
            })?;
        let resp = client.post(&url).json(req).send().await.map_err(|e| {
            StorageError::backend(std::io::Error::other(format!(
                "agent smb {op} request failed: {e}"
            )))
        })?;
        let status = resp.status();
        let body = resp.text().await.map_err(|e| {
            StorageError::backend(std::io::Error::other(format!(
                "agent smb {op} response read failed: {e}"
            )))
        })?;
        if !status.is_success() {
            return Err(StorageError::backend(std::io::Error::other(format!(
                "agent smb {op} failed: HTTP {status}: {body}"
            ))));
        }
        serde_json::from_str(&body).map_err(|e| {
            StorageError::backend(std::io::Error::other(format!(
                "agent smb {op} response decode failed: {e}; body: {body}"
            )))
        })
    }

    /// Like `agent_post` but tolerant of empty bodies (the agent's SMB
    /// handlers return 204 NO_CONTENT for ops with no payload). Status >=
    /// 400 is still an error.
    async fn agent_post_empty<Req>(&self, op: &str, req: &Req) -> Result<(), StorageError>
    where
        Req: Serialize + ?Sized,
    {
        let url = self.agent_smb_url(op).ok_or_else(|| {
            StorageError::backend(std::io::Error::other(
                "smb backend requires config.agent_url, or assume_mounted=true for local testing",
            ))
        })?;
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| {
                StorageError::backend(std::io::Error::other(format!(
                    "agent smb client init failed: {e}"
                )))
            })?;
        let resp = client.post(&url).json(req).send().await.map_err(|e| {
            StorageError::backend(std::io::Error::other(format!(
                "agent smb {op} request failed: {e}"
            )))
        })?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(StorageError::backend(std::io::Error::other(format!(
                "agent smb {op} failed: HTTP {status}: {body}"
            ))));
        }
        Ok(())
    }

    /// Resolve the mount point used to address volume files. When
    /// `assume_mounted=true`, just return the deterministic local path
    /// (no agent call). Otherwise POST `/mount` and trust the agent's
    /// reported mount point.
    async fn ensure_mounted(&self) -> Result<PathBuf, StorageError> {
        if self.config.assume_mounted {
            let mp = self.config.mount_point();
            tokio::fs::create_dir_all(&mp).await?;
            return Ok(mp);
        }
        if self.config.agent_url.is_none() {
            return Err(StorageError::backend(std::io::Error::other(
                "smb backend requires config.agent_url; manager does not run mount.cifs",
            )));
        }

        #[derive(Serialize)]
        struct MountReq<'a> {
            backend_id: Uuid,
            #[serde(skip_serializing_if = "Option::is_none")]
            mount_base: Option<&'a str>,
            server: &'a str,
            share: &'a str,
            #[serde(skip_serializing_if = "Option::is_none")]
            subdir: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            username: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            domain: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            smb_version: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            options: Option<&'a str>,
        }
        #[derive(Deserialize)]
        struct MountResp {
            mount_point: PathBuf,
        }

        let mount_base_str = self
            .config
            .mount_base
            .as_deref()
            .map(|p| p.to_string_lossy().into_owned());
        let body = MountReq {
            backend_id: self.id.0,
            mount_base: mount_base_str.as_deref(),
            server: &self.config.server,
            share: &self.config.share,
            subdir: self.config.subdir.as_deref(),
            username: self.config.username.as_deref(),
            domain: self.config.domain.as_deref(),
            smb_version: self.config.smb_version.as_deref(),
            options: self.config.options.as_deref(),
        };
        let resp: MountResp = self.agent_post("mount", &body).await?;
        Ok(resp.mount_point)
    }
}

#[async_trait::async_trait]
impl ControlPlaneBackend for SmbControlPlaneBackend {
    fn kind(&self) -> BackendKind {
        BackendKind::Smb
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            // SMB is a file protocol like NFS — we can clone an image file
            // straight onto the share.
            supports_clone_from_image: true,
            // Snapshots are `cp` copies (no server-side CoW guaranteed
            // across CIFS implementations), so we don't claim native.
            supports_native_snapshots: false,
            // The share is shared, but per-volume locking is up to the
            // operator. Don't claim concurrent attach safety.
            supports_concurrent_attach: false,
            supports_live_migration: false,
        }
    }

    async fn probe(&self) -> Result<(), StorageError> {
        self.ensure_mounted().await.map(|_| ())
    }

    fn host_path_for(&self, handle: &VolumeHandle) -> Option<PathBuf> {
        let loc: SmbLocator = serde_json::from_str(&handle.locator).ok()?;
        Some(self.config.mount_point().join(loc.file))
    }

    async fn provision(&self, opts: CreateOpts) -> Result<VolumeHandle, StorageError> {
        let vol_id = Uuid::new_v4();
        let file = format!("smb-{vol_id}.raw");
        if self.config.agent_url.is_some() && !self.config.assume_mounted {
            let mount_point = self.ensure_mounted().await?;
            #[derive(Serialize)]
            struct CreateReq<'a> {
                mount_point: &'a Path,
                file: &'a str,
                size_bytes: u64,
            }
            self.agent_post_empty(
                "create_file",
                &CreateReq {
                    mount_point: &mount_point,
                    file: &file,
                    size_bytes: opts.size_bytes,
                },
            )
            .await?;
        } else {
            // assume_mounted=true: do the file op locally. This is the
            // test/automount path; in production the agent handles it.
            let mount = self.ensure_mounted().await?;
            let path = mount.join(&file);
            let f = tokio::fs::File::create(&path).await?;
            f.set_len(opts.size_bytes).await?;
            drop(f);
        }
        let locator = SmbLocator {
            server: self.config.server.clone(),
            share: self.config.share.clone(),
            subdir: self.config.subdir.clone(),
            file,
        };
        Ok(VolumeHandle {
            volume_id: vol_id,
            backend_id: self.id,
            backend_kind: BackendKind::Smb,
            locator: locator.to_locator_string()?,
            size_bytes: opts.size_bytes,
        })
    }

    async fn destroy(&self, handle: VolumeHandle) -> Result<(), StorageError> {
        let loc = SmbLocator::from_locator_str(&handle.locator)?;
        if self.config.agent_url.is_some() && !self.config.assume_mounted {
            let mount_point = self.ensure_mounted().await?;
            #[derive(Serialize)]
            struct DeleteReq<'a> {
                mount_point: &'a Path,
                file: &'a str,
            }
            self.agent_post_empty(
                "delete_file",
                &DeleteReq {
                    mount_point: &mount_point,
                    file: &loc.file,
                },
            )
            .await
        } else {
            let mount = self.ensure_mounted().await?;
            let path = mount.join(&loc.file);
            match tokio::fs::remove_file(&path).await {
                Ok(()) => Ok(()),
                // Idempotent: a destroy that races with another caller is
                // success, not error.
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(e) => Err(StorageError::from(e)),
            }
        }
    }

    async fn clone_from_image(
        &self,
        source_image: &Path,
        opts: CreateOpts,
    ) -> Result<VolumeHandle, StorageError> {
        let vol_id = Uuid::new_v4();
        let file = format!("smb-{vol_id}.raw");
        let size_bytes = if self.config.agent_url.is_some() && !self.config.assume_mounted {
            let mount_point = self.ensure_mounted().await?;
            #[derive(Serialize)]
            struct CloneReq<'a> {
                source_path: &'a Path,
                mount_point: &'a Path,
                file: &'a str,
            }
            #[derive(Deserialize)]
            struct CloneResp {
                size_bytes: u64,
            }
            let resp: CloneResp = self
                .agent_post(
                    "clone_from_path",
                    &CloneReq {
                        source_path: source_image,
                        mount_point: &mount_point,
                        file: &file,
                    },
                )
                .await?;
            // The agent's clone returns the bytes-copied count from the
            // source. Trust the operator-requested size — rootfs typically
            // wants more headroom and resize2fs runs later to expand into
            // it. Use the larger of (source size, requested size).
            std::cmp::max(resp.size_bytes, opts.size_bytes)
        } else {
            let mount = self.ensure_mounted().await?;
            let dst = mount.join(&file);
            tokio::fs::copy(source_image, &dst).await?;
            let cur = tokio::fs::metadata(&dst).await?.len();
            if opts.size_bytes != cur {
                let f = tokio::fs::OpenOptions::new().write(true).open(&dst).await?;
                f.set_len(opts.size_bytes).await?;
            }
            opts.size_bytes
        };
        let locator = SmbLocator {
            server: self.config.server.clone(),
            share: self.config.share.clone(),
            subdir: self.config.subdir.clone(),
            file,
        };
        Ok(VolumeHandle {
            volume_id: vol_id,
            backend_id: self.id,
            backend_kind: BackendKind::Smb,
            locator: locator.to_locator_string()?,
            size_bytes,
        })
    }

    async fn snapshot(
        &self,
        volume: &VolumeHandle,
        name: &str,
    ) -> Result<VolumeSnapshotHandle, StorageError> {
        if name.is_empty() || name.contains('/') {
            return Err(StorageError::InvalidLocator(
                "snapshot name must be non-empty and contain no '/'".into(),
            ));
        }
        let src_loc = SmbLocator::from_locator_str(&volume.locator)?;
        let snap_file = format!("{}.snap-{name}", src_loc.file);
        if self.config.agent_url.is_some() && !self.config.assume_mounted {
            let mount_point = self.ensure_mounted().await?;
            #[derive(Serialize)]
            struct SnapReq<'a> {
                mount_point: &'a Path,
                source_file: &'a str,
                snap_file: &'a str,
            }
            self.agent_post_empty(
                "snapshot",
                &SnapReq {
                    mount_point: &mount_point,
                    source_file: &src_loc.file,
                    snap_file: &snap_file,
                },
            )
            .await?;
        } else {
            let mount = self.ensure_mounted().await?;
            let src_path = mount.join(&src_loc.file);
            let snap_path = mount.join(&snap_file);
            tokio::fs::copy(&src_path, &snap_path).await?;
            // Maintain the volume's provisioned size on the snapshot file
            // so a later clone reports the right size_bytes even when the
            // source has been truncated below the provisioned size.
            let f = tokio::fs::OpenOptions::new()
                .write(true)
                .open(&snap_path)
                .await?;
            f.set_len(volume.size_bytes).await?;
        }
        let snap_locator = SmbLocator {
            server: src_loc.server,
            share: src_loc.share,
            subdir: src_loc.subdir,
            file: snap_file,
        };
        Ok(VolumeSnapshotHandle {
            snapshot_id: Uuid::new_v4(),
            source_volume_id: volume.volume_id,
            backend_id: self.id,
            backend_kind: BackendKind::Smb,
            locator: snap_locator.to_locator_string()?,
        })
    }

    async fn clone_from_snapshot(
        &self,
        snap: &VolumeSnapshotHandle,
    ) -> Result<VolumeHandle, StorageError> {
        let snap_loc = SmbLocator::from_locator_str(&snap.locator)?;
        let vol_id = Uuid::new_v4();
        let file = format!("smb-{vol_id}.raw");
        let size_bytes = if self.config.agent_url.is_some() && !self.config.assume_mounted {
            let mount_point = self.ensure_mounted().await?;
            #[derive(Serialize)]
            struct CloneSnapReq<'a> {
                mount_point: &'a Path,
                snap_file: &'a str,
                file: &'a str,
            }
            #[derive(Deserialize)]
            struct CloneSnapResp {
                size_bytes: u64,
            }
            let resp: CloneSnapResp = self
                .agent_post(
                    "clone_from_snapshot",
                    &CloneSnapReq {
                        mount_point: &mount_point,
                        snap_file: &snap_loc.file,
                        file: &file,
                    },
                )
                .await?;
            resp.size_bytes
        } else {
            let mount = self.ensure_mounted().await?;
            let src_path = mount.join(&snap_loc.file);
            let dst = mount.join(&file);
            tokio::fs::copy(&src_path, &dst).await?;
            tokio::fs::metadata(&dst).await?.len()
        };
        let locator = SmbLocator {
            server: snap_loc.server,
            share: snap_loc.share,
            subdir: snap_loc.subdir,
            file,
        };
        Ok(VolumeHandle {
            volume_id: vol_id,
            backend_id: self.id,
            backend_kind: BackendKind::Smb,
            locator: locator.to_locator_string()?,
            size_bytes,
        })
    }

    async fn delete_snapshot(&self, snap: VolumeSnapshotHandle) -> Result<(), StorageError> {
        let loc = SmbLocator::from_locator_str(&snap.locator)?;
        if self.config.agent_url.is_some() && !self.config.assume_mounted {
            let mount_point = self.ensure_mounted().await?;
            #[derive(Serialize)]
            struct DeleteReq<'a> {
                mount_point: &'a Path,
                file: &'a str,
            }
            self.agent_post_empty(
                "delete_file",
                &DeleteReq {
                    mount_point: &mount_point,
                    file: &loc.file,
                },
            )
            .await
        } else {
            let mount = self.ensure_mounted().await?;
            let path = mount.join(&loc.file);
            match tokio::fs::remove_file(&path).await {
                Ok(()) => Ok(()),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(e) => Err(StorageError::from(e)),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg_minimal() -> SmbConfig {
        SmbConfig {
            server: "s".into(),
            share: "x".into(),
            subdir: None,
            username: None,
            domain: None,
            smb_version: None,
            options: None,
            mount_base: None,
            assume_mounted: false,
            agent_url: None,
        }
    }

    #[test]
    fn smb_config_parses_minimal_json() {
        let json = r#"{"server":"srv","share":"vms"}"#;
        let cfg: SmbConfig = serde_json::from_str(json).unwrap();
        assert_eq!(cfg.server, "srv");
        assert_eq!(cfg.share, "vms");
        assert!(cfg.username.is_none());
        assert!(cfg.subdir.is_none());
        assert!(!cfg.assume_mounted);
        assert!(cfg.agent_url.is_none());
    }

    #[test]
    fn smb_config_parses_full_json() {
        let json = r#"{
            "server":"srv","share":"vms","subdir":"t1",
            "username":"u","domain":"d","smb_version":"3.0",
            "options":"uid=0","mount_base":"/x","assume_mounted":true,
            "agent_url":"http://127.0.0.1:9090"
        }"#;
        let cfg: SmbConfig = serde_json::from_str(json).unwrap();
        assert_eq!(cfg.subdir.as_deref(), Some("t1"));
        assert_eq!(cfg.username.as_deref(), Some("u"));
        assert_eq!(cfg.domain.as_deref(), Some("d"));
        assert_eq!(cfg.smb_version.as_deref(), Some("3.0"));
        assert_eq!(cfg.options.as_deref(), Some("uid=0"));
        assert_eq!(cfg.mount_base.as_deref(), Some(Path::new("/x")));
        assert!(cfg.assume_mounted);
        assert_eq!(cfg.agent_url.as_deref(), Some("http://127.0.0.1:9090"));
    }

    #[test]
    fn smb_locator_round_trips() {
        let l = SmbLocator {
            server: "srv".into(),
            share: "vms".into(),
            subdir: Some("a".into()),
            file: "rootfs.raw".into(),
        };
        let s = l.to_locator_string().unwrap();
        let back = SmbLocator::from_locator_str(&s).unwrap();
        assert_eq!(l, back);
    }

    #[test]
    fn smb_locator_rejects_path_traversal_in_file() {
        let bad = serde_json::json!({
            "server":"s","share":"x","subdir":null,
            "file":"../../etc/passwd"
        })
        .to_string();
        let err = SmbLocator::from_locator_str(&bad).unwrap_err();
        assert!(matches!(err, StorageError::InvalidLocator(_)), "{err}");
    }

    #[test]
    fn smb_locator_rejects_leading_dot() {
        let bad = serde_json::json!({
            "server":"s","share":"x","subdir":null,
            "file":".hidden"
        })
        .to_string();
        let err = SmbLocator::from_locator_str(&bad).unwrap_err();
        assert!(matches!(err, StorageError::InvalidLocator(_)), "{err}");
    }

    #[test]
    fn smb_mount_point_format() {
        let cfg = SmbConfig {
            server: "192.168.1.10".into(),
            share: "vm/data".into(),
            ..cfg_minimal()
        };
        assert_eq!(
            cfg.mount_point().to_string_lossy(),
            "/var/lib/nqrust/smb/192.168.1.10:vm_data"
        );
    }

    #[test]
    fn smb_mount_point_unique_per_server_and_share() {
        let make = |server: &str, share: &str| SmbConfig {
            server: server.into(),
            share: share.into(),
            ..cfg_minimal()
        };
        let a = make("10.0.0.5", "vms").mount_point();
        let b = make("10.0.0.5", "iso").mount_point();
        let c = make("10.0.0.6", "vms").mount_point();
        assert_ne!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn capabilities_advertise_file_protocol_traits() {
        let backend = SmbControlPlaneBackend {
            id: BackendInstanceId(Uuid::new_v4()),
            config: cfg_minimal(),
        };
        let c = backend.capabilities();
        assert!(c.supports_clone_from_image);
        assert!(!c.supports_native_snapshots);
        assert!(!c.supports_concurrent_attach);
        assert!(!c.supports_live_migration);
    }

    #[test]
    fn host_path_for_uses_mount_point() {
        let backend = SmbControlPlaneBackend {
            id: BackendInstanceId(Uuid::new_v4()),
            config: SmbConfig {
                server: "srv".into(),
                share: "vms".into(),
                ..cfg_minimal()
            },
        };
        let handle = VolumeHandle {
            volume_id: Uuid::new_v4(),
            backend_id: backend.id,
            backend_kind: BackendKind::Smb,
            locator: r#"{"server":"srv","share":"vms","subdir":null,"file":"rootfs.raw"}"#.into(),
            size_bytes: 0,
        };
        let p = backend.host_path_for(&handle).unwrap();
        assert_eq!(p, PathBuf::from("/var/lib/nqrust/smb/srv:vms/rootfs.raw"));
    }

    /// Build a backend whose `mount_base` is a tempdir and whose
    /// `assume_mounted` flag is true so the runtime tests don't try to
    /// invoke `mount.cifs` or POST to a non-existent agent.
    fn temp_backend() -> (SmbControlPlaneBackend, tempfile::TempDir) {
        let dir = tempfile::tempdir().expect("tempdir");
        let backend = SmbControlPlaneBackend {
            id: BackendInstanceId(Uuid::new_v4()),
            config: SmbConfig {
                server: "10.0.0.5".into(),
                share: "vms".into(),
                mount_base: Some(dir.path().to_path_buf()),
                assume_mounted: true,
                ..cfg_minimal()
            },
        };
        (backend, dir)
    }

    #[tokio::test]
    async fn provision_creates_sparse_file_at_requested_size() {
        let (backend, _guard) = temp_backend();
        let h = backend
            .provision(CreateOpts {
                name: "v".into(),
                size_bytes: 4 * 1024 * 1024,
                description: None,
            })
            .await
            .expect("provision");
        let loc = SmbLocator::from_locator_str(&h.locator).unwrap();
        let path = backend.config.mount_point().join(&loc.file);
        let md = tokio::fs::metadata(&path).await.unwrap();
        assert_eq!(md.len(), 4 * 1024 * 1024);
        assert!(loc.file.starts_with("smb-"));
        assert!(loc.file.ends_with(".raw"));
        assert_eq!(loc.server, "10.0.0.5");
        assert_eq!(loc.share, "vms");
    }

    #[tokio::test]
    async fn destroy_unlinks_then_is_idempotent() {
        let (backend, _guard) = temp_backend();
        let h = backend
            .provision(CreateOpts {
                name: "v".into(),
                size_bytes: 1024,
                description: None,
            })
            .await
            .unwrap();
        let loc = SmbLocator::from_locator_str(&h.locator).unwrap();
        let path = backend.config.mount_point().join(&loc.file);
        assert!(tokio::fs::metadata(&path).await.is_ok());
        backend.destroy(h.clone()).await.expect("destroy");
        assert!(tokio::fs::metadata(&path).await.is_err());
        // Second destroy of the same handle must succeed (idempotent).
        backend.destroy(h).await.expect("idempotent destroy");
    }

    #[tokio::test]
    async fn snapshot_clone_delete_round_trip() {
        let (backend, _guard) = temp_backend();
        let h = backend
            .provision(CreateOpts {
                name: "v".into(),
                size_bytes: 1024,
                description: None,
            })
            .await
            .unwrap();
        let loc = SmbLocator::from_locator_str(&h.locator).unwrap();
        let src_path = backend.config.mount_point().join(&loc.file);
        tokio::fs::write(&src_path, b"original-data").await.unwrap();

        let snap = backend.snapshot(&h, "snap-1").await.expect("snapshot");
        let snap_loc = SmbLocator::from_locator_str(&snap.locator).unwrap();
        let snap_path = backend.config.mount_point().join(&snap_loc.file);
        let snap_meta = tokio::fs::metadata(&snap_path).await.unwrap();
        assert_eq!(
            snap_meta.len(),
            1024,
            "snapshot file must be at provisioned size"
        );

        let cloned = backend.clone_from_snapshot(&snap).await.expect("clone");
        let cloned_loc = SmbLocator::from_locator_str(&cloned.locator).unwrap();
        let cloned_path = backend.config.mount_point().join(&cloned_loc.file);
        let cloned_meta = tokio::fs::metadata(&cloned_path).await.unwrap();
        assert_eq!(cloned_meta.len(), 1024);
        assert_eq!(cloned.size_bytes, 1024);
        let restored = tokio::fs::read(&cloned_path).await.unwrap();
        assert_eq!(&restored[..13], b"original-data");

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
        let h = backend
            .clone_from_image(
                &src,
                CreateOpts {
                    name: "v".into(),
                    size_bytes: 4096,
                    description: None,
                },
            )
            .await
            .unwrap();
        let loc = SmbLocator::from_locator_str(&h.locator).unwrap();
        let path = backend.config.mount_point().join(&loc.file);
        let md = tokio::fs::metadata(&path).await.unwrap();
        assert_eq!(md.len(), 4096);
        let buf = tokio::fs::read(&path).await.unwrap();
        assert_eq!(&buf[..11], b"hello world");
    }

    #[tokio::test]
    async fn manager_requires_agent_url_unless_assume_mounted() {
        let backend = SmbControlPlaneBackend {
            id: BackendInstanceId(Uuid::new_v4()),
            config: cfg_minimal(), // agent_url=None, assume_mounted=false
        };
        let err = backend.ensure_mounted().await.unwrap_err();
        assert!(
            err.to_string().contains("config.agent_url"),
            "expected agent_url error, got: {err}"
        );
    }
}
