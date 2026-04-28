//! Agent-side iSCSI host backend. Logs in via iscsiadm, returns the
//! /dev/disk/by-path symlink as an AttachedPath::BlockDevice. Detach is
//! aggressive logout (per spec recommendation; iSCSI session refcounting is
//! not used in this PR).
//!
//! Locator format (JSON): {"iqn":"...","lun":N,"dataset":"...","portal":"..."}
//! `dataset` is ignored on the host side; it's a control-plane concern.

use nexus_storage::{AttachedPath, BackendKind, HostBackend, StorageError, VolumeHandle};
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Deserialize, Debug)]
struct LocatorJson {
    iqn: String,
    lun: u32,
    #[serde(default)]
    portal: Option<String>,
}

pub struct IscsiHostBackend;

impl IscsiHostBackend {
    fn parse_locator(s: &str) -> Result<LocatorJson, StorageError> {
        serde_json::from_str(s).map_err(|e| StorageError::InvalidLocator(format!("{s}: {e}")))
    }

    async fn iscsiadm_login(loc: &LocatorJson) -> Result<(), StorageError> {
        let portal = loc
            .portal
            .clone()
            .unwrap_or_else(|| "127.0.0.1".to_string());
        // Discovery (idempotent on subsequent calls)
        let _ = tokio::process::Command::new("iscsiadm")
            .args(["-m", "discovery", "-t", "sendtargets", "-p", &portal])
            .output()
            .await
            .map_err(|e| StorageError::Backend(Box::new(e)))?;
        // Login
        let out = tokio::process::Command::new("iscsiadm")
            .args(["-m", "node", "-T", &loc.iqn, "-p", &portal, "--login"])
            .output()
            .await
            .map_err(|e| StorageError::Backend(Box::new(e)))?;
        if !out.status.success() {
            let stderr = String::from_utf8_lossy(&out.stderr);
            // "already logged in" / "already exists" are acceptable
            if !stderr.contains("already") {
                return Err(StorageError::Backend(
                    anyhow::anyhow!("iscsiadm login: {stderr}").into(),
                ));
            }
        }
        Ok(())
    }

    async fn iscsiadm_logout(loc: &LocatorJson) -> Result<(), StorageError> {
        let portal = loc
            .portal
            .clone()
            .unwrap_or_else(|| "127.0.0.1".to_string());
        // Best-effort; ignore errors (aggressive logout per spec)
        let _ = tokio::process::Command::new("iscsiadm")
            .args(["-m", "node", "-T", &loc.iqn, "-p", &portal, "--logout"])
            .output()
            .await;
        Ok(())
    }

    fn block_device_path(loc: &LocatorJson) -> PathBuf {
        let portal = loc
            .portal
            .clone()
            .unwrap_or_else(|| "127.0.0.1".to_string());
        PathBuf::from(format!(
            "/dev/disk/by-path/ip-{portal}:3260-iscsi-{}-lun-{}",
            loc.iqn, loc.lun
        ))
    }
}

#[async_trait::async_trait]
impl HostBackend for IscsiHostBackend {
    fn kind(&self) -> BackendKind {
        BackendKind::Iscsi
    }

    async fn attach(&self, volume: &VolumeHandle) -> Result<AttachedPath, StorageError> {
        let loc = Self::parse_locator(&volume.locator)?;
        Self::iscsiadm_login(&loc).await?;
        let dev = Self::block_device_path(&loc);
        // Wait for udev to create the by-path symlink (~3 seconds max)
        for _ in 0..30 {
            if dev.exists() {
                return Ok(AttachedPath::BlockDevice(dev));
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
        Err(StorageError::Backend(
            anyhow::anyhow!("device {} did not appear after iscsi login", dev.display()).into(),
        ))
    }

    async fn detach(
        &self,
        volume: &VolumeHandle,
        _attached: AttachedPath,
    ) -> Result<(), StorageError> {
        let loc = Self::parse_locator(&volume.locator)?;
        Self::iscsiadm_logout(&loc).await
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
            .open(dst_path)
            .await?;
        tokio::io::copy(&mut src, &mut dst).await?;
        dst.flush().await?;
        // For block devices set_len is not applicable; the LUN size is fixed
        // by the control plane at provision time. target_size_bytes is informational.
        let _ = target_size_bytes;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_locator_json() {
        let s = r#"{"iqn":"iqn.x:tgt","lun":3,"dataset":"tank/v","portal":"10.0.0.5"}"#;
        let loc = IscsiHostBackend::parse_locator(s).unwrap();
        assert_eq!(loc.lun, 3);
        assert_eq!(loc.portal.as_deref(), Some("10.0.0.5"));
    }

    #[test]
    fn rejects_malformed_locator() {
        let err = IscsiHostBackend::parse_locator("not json").unwrap_err();
        assert!(matches!(err, StorageError::InvalidLocator(_)));
    }

    #[test]
    fn block_device_path_format() {
        let loc = LocatorJson {
            iqn: "iqn.x:tgt".into(),
            lun: 3,
            portal: Some("10.0.0.5".into()),
        };
        let p = IscsiHostBackend::block_device_path(&loc);
        assert_eq!(
            p.to_str().unwrap(),
            "/dev/disk/by-path/ip-10.0.0.5:3260-iscsi-iqn.x:tgt-lun-3"
        );
    }
}
